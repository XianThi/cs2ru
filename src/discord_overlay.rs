//! Discord V3 out-of-process overlay köprüsü (CDP üzerinden canvas enjeksiyonu).
//!
//! Discord 1.0.9xxx sürümlerinde overlay, oyuna hook enjekte etmiyor; ayrı bir
//! `WS_EX_LAYERED | TOPMOST` Chromium penceresi ("Discord Overlay") olarak çalışıyor.
//! Bu pencerenin sayfasına `--remote-debugging-port` üzerinden erişip kendi
//! `<canvas>` katmanımızı kuruyoruz; ESP geometrisini `window.__ESP` üzerinden
//! her karede itiyoruz.
//!
//! Kullanım:
//! ```ignore
//! discord_overlay::spawn(snapshot, settings, running);
//! ```

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use crate::app::{EspSnapshot, VisualSettings};
use crate::esp;
use crate::hash::{json_get_by_hash, rt_hash};
use crate::hud;
use crate::mode::ModeController;
use obfstr::obfstr;

/// Ana Discord süreci `--remote-debugging-port=9222` ile başlatılmalı (sabit port derleme zamanında maskelenir).
#[inline]
fn cdp_port() -> u16 {
    crate::obf_u16!(9222)
}
/// ESP gönderim hızı.
const SEND_HZ: u64 = 60;

// ---------------------------------------------------------------------------
// Minimal WebSocket (RFC6455) istemcisi — sadece metin çerçevesi, istemci maskeli
// ---------------------------------------------------------------------------

pub struct CdpSocket {
    stream: TcpStream,
    next_id: u64,
}

impl CdpSocket {
    /// `ws://127.0.0.1:<port>/devtools/page/<id>` adresine bağlanır.
    fn connect(ws_url: &str) -> Result<Self, String> {
        let rest = ws_url
            .strip_prefix(obfstr!("ws://"))
            .ok_or_else(|| format!("desteklenmeyen ws url: {ws_url}"))?;
        let (hostport, path) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };
        let hostport = if hostport.contains(':') {
            hostport.to_string()
        } else {
            format!("{hostport}:80")
        };

        let mut stream = TcpStream::connect(&hostport)
            .map_err(|e| format!("CDP TCP bağlantısı başarısız ({hostport}): {e}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .map_err(|e| e.to_string())?;
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .map_err(|e| e.to_string())?;
        let _ = stream.set_nodelay(true);

        let key = ws_key();
        let req = format!(
            "GET {path} HTTP/1.1\r\nHost: {hostport}\r\n{}: {key}\r\n{}",
            obfstr!("Sec-WebSocket-Key"),
            obfstr!(
                "Upgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nUser-Agent: Mozilla/5.0\r\n\r\n"
            )
        );
        stream
            .write_all(req.as_bytes())
            .map_err(|e| format!("CDP el sıkışma yazımı başarısız: {e}"))?;

        // Yanıt başlıklarını oku (101 bekleniyor)
        let mut head = Vec::with_capacity(1024);
        let mut byte = [0u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            match stream.read(&mut byte) {
                Ok(0) => return Err("CDP el sıkışması sırasında bağlantı kapandı".into()),
                Ok(_) => head.push(byte[0]),
                Err(e) => return Err(format!("CDP el sıkışma okuması başarısız: {e}")),
            }
            if head.len() > 8192 {
                return Err("CDP el sıkışma yanıtı çok büyük".into());
            }
        }
        let head_str = String::from_utf8_lossy(&head);
        if !head_str.starts_with("HTTP/1.1 101") && !head_str.starts_with("HTTP/1.0 101") {
            let first = head_str.lines().next().unwrap_or("").to_string();
            return Err(format!("CDP upgrade reddedildi: {first}"));
        }

        Ok(Self {
            stream,
            next_id: 1,
        })
    }

    fn send_text(&mut self, payload: &[u8]) -> Result<(), String> {
        let mut frame = Vec::with_capacity(payload.len() + 14);
        frame.push(0x81); // FIN + text

        let len = payload.len();
        if len < 126 {
            frame.push(0x80 | len as u8);
        } else if len <= 0xFFFF {
            frame.push(0x80 | 126);
            frame.extend_from_slice(&(len as u16).to_be_bytes());
        } else {
            frame.push(0x80 | 127);
            frame.extend_from_slice(&(len as u64).to_be_bytes());
        }

        let mask = mask_key();
        frame.extend_from_slice(&mask);
        frame.extend(payload.iter().enumerate().map(|(i, b)| b ^ mask[i % 4]));

        self.stream
            .write_all(&frame)
            .map_err(|e| format!("CDP çerçeve yazımı başarısız: {e}"))
    }

    fn read_exact_n(&mut self, n: usize) -> Result<Vec<u8>, String> {
        let mut buf = vec![0u8; n];
        self.stream
            .read_exact(&mut buf)
            .map_err(|e| format!("CDP çerçeve okuması başarısız: {e}"))?;
        Ok(buf)
    }

    /// Tek bir metin çerçevesi okur (parçalanmış çerçeveler birleştirilir).
    fn read_text(&mut self) -> Result<String, String> {
        let mut out = Vec::new();
        loop {
            let h = self.read_exact_n(2)?;
            let fin = h[0] & 0x80 != 0;
            let opcode = h[0] & 0x0F;
            let masked = h[1] & 0x80 != 0;
            let mut len = (h[1] & 0x7F) as usize;
            if len == 126 {
                let b = self.read_exact_n(2)?;
                len = u16::from_be_bytes([b[0], b[1]]) as usize;
            } else if len == 127 {
                let b = self.read_exact_n(8)?;
                len = u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]) as usize;
            }
            let mask = if masked { Some(self.read_exact_n(4)?) } else { None };
            let mut data = self.read_exact_n(len)?;
            if let Some(m) = mask {
                for (i, b) in data.iter_mut().enumerate() {
                    *b ^= m[i % 4];
                }
            }
            match opcode {
                0x8 => return Err("CDP WebSocket kapandı".into()),
                0x9 => {
                    // ping -> pong (boş yanıt yeterli değil, aynı yükü geri gönder)
                    let mut frame = vec![0x8A];
                    frame.push(0x80 | data.len() as u8);
                    let m = mask_key();
                    frame.extend_from_slice(&m);
                    frame.extend(data.iter().enumerate().map(|(i, b)| b ^ m[i % 4]));
                    let _ = self.stream.write_all(&frame);
                }
                0x1 | 0x0 => {
                    out.extend_from_slice(&data);
                    if fin {
                        return String::from_utf8(out).map_err(|e| e.to_string());
                    }
                }
                _ => {}
            }
            if fin && opcode != 0x0 {
                return String::from_utf8(out).map_err(|e| e.to_string());
            }
        }
    }

    /// `Runtime.evaluate` çalıştırır ve sonucun metin değerini döner.
    fn eval(&mut self, js: &str) -> Result<String, String> {
        let id = self.next_id;
        self.next_id += 1;

        let method_name = obfstr!("Runtime.evaluate").to_string();
        let msg = serde_json::json!({
            "id": id,
            "method": method_name,
            "params": {
                "expression": js,
                "returnByValue": true,
                "awaitPromise": false,
                "userGesture": true
            }
        });
        self.send_text(msg.to_string().as_bytes())?;

        // Kendi id'mize ait yanıtı bekle (event'leri atla)
        for _ in 0..64 {
            let text = match self.read_text() {
                Ok(t) => t,
                Err(e) => return Err(e),
            };
            let v: serde_json::Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if json_get_by_hash(&v, crate::kh!("id")).and_then(|x| x.as_u64()) != Some(id) {
                continue;
            }
            if let Some(err) = json_get_by_hash(&v, crate::kh!("error")) {
                return Err(format!("CDP hatası: {err}"));
            }
            let result = json_get_by_hash(&v, crate::kh!("result"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            if let Some(ex) = json_get_by_hash(&result, crate::kh!("exceptionDetails")) {
                return Err(format!("JS hatası: {ex}"));
            }
            return Ok(json_get_by_hash(&result, crate::kh!("result"))
                .and_then(|r| json_get_by_hash(r, crate::kh!("value")))
                .map(|x| match x {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .unwrap_or_default());
        }
        Err("CDP yanıtı zaman aşımı".into())
    }
}

fn ws_key() -> String {
    // 16 rastgele bayt -> base64
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E3779B97F4A7C15)
        ^ (std::process::id() as u64).wrapping_mul(0x2545F4914F6CDD1D);
    let mut state = seed | 1;
    let mut bytes = [0u8; 16];
    for b in bytes.iter_mut() {
        // xorshift64*
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let v = state.wrapping_mul(0x2545F4914F6CDD1D);
        *b = (v >> 33) as u8;
    }
    let start = ws_key_counter();
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = b.wrapping_add((start.wrapping_mul(131) >> (i % 8)) as u8);
    }
    base64(&bytes)
}

fn ws_key_counter() -> u32 {
    use std::sync::atomic::AtomicU32;
    static C: AtomicU32 = AtomicU32::new(0x1234_5678);
    C.fetch_add(0x9E37_79B9, Ordering::Relaxed)
}

fn mask_key() -> [u8; 4] {
    let n = ws_key_counter();
    let mut s = (n as u64) | 1;
    s ^= s >> 12;
    s ^= s << 25;
    s ^= s >> 27;
    s = s.wrapping_mul(0x2545F4914F6CDD1D);
    [(s >> 24) as u8, (s >> 16) as u8, (s >> 8) as u8, s as u8]
}

fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            T[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

// ---------------------------------------------------------------------------
// Hedef bulma
// ---------------------------------------------------------------------------

/// `http://127.0.0.1:9222/json/list` içinden Discord overlay sayfasının
/// `webSocketDebuggerUrl` adresini seçer (anahtarlar runtime hash ile çözülür).
fn find_overlay_target(port: u16) -> Result<String, String> {
    let url = format!("{}{port}{}", obfstr!("http://127.0.0.1:"), obfstr!("/json/list"));
    let body = ureq::get(&url)
        .timeout(Duration::from_secs(2))
        .call()
        .map_err(|e| format!("CDP target listesi alınamadı ({url}): {e}"))?
        .into_string()
        .map_err(|e| e.to_string())?;

    let list: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("CDP target JSON hatası: {e}"))?;
    let arr = list.as_array().ok_or("CDP target listesi dizi değil")?;

    let mut fallback: Option<String> = None;
    for t in arr {
        let ty = json_get_by_hash(t, crate::kh!("type"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if rt_hash(ty) != crate::kh!("page") {
            continue;
        }
        let ws = match json_get_by_hash(t, crate::kh!("webSocketDebuggerUrl")).and_then(|v| v.as_str()) {
            Some(w) => w.to_string(),
            None => continue,
        };
        let title = json_get_by_hash(t, crate::kh!("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let turl = json_get_by_hash(t, crate::kh!("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if title.to_ascii_lowercase().contains(obfstr!("overlay")) || turl.contains(obfstr!("popout")) {
            return Ok(ws);
        }
        if fallback.is_none() {
            fallback = Some(ws);
        }
    }
    fallback.ok_or_else(|| "CDP'de uygun target bulunamadı".to_string())
}

// ---------------------------------------------------------------------------
// Çalışma zamanı rastgele DOM / JS tanımlayıcı üreteci (sıfır sabit imza)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SessionTokens {
    pub canvas_id: String,
    pub state_var: String,
    pub gen_var: String,
    pub fb_var: String,
    pub stop_var: String,
    pub fps_var: String,
    pub frames_var: String,
    pub err_var: String,
}

impl SessionTokens {
    /// Her süreç çalışmasında benzersiz, sıradan görünümlü DOM ve `window`
    /// tanımlayıcıları üretir (`__hermes_fb` veya `window.__ESP` gibi sabit imzalar bırakmaz).
    pub fn random() -> Self {
        let mut s = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xA5A5_5A5A_F0F0_0F0F)
            ^ (std::process::id() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (ws_key_counter() as u64);
        if s == 0 {
            s = 1;
        }

        let mut next_ident = |prefix: &str| -> String {
            const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
            let mut out = String::with_capacity(prefix.len() + 8);
            out.push_str(prefix);
            for _ in 0..8 {
                s ^= s >> 12;
                s ^= s << 25;
                s ^= s >> 27;
                let v = s.wrapping_mul(0x2545_F491_4F6C_DD1D);
                out.push(CHARS[(v as usize) % CHARS.len()] as char);
            }
            out
        };

        Self {
            canvas_id: next_ident("ui_layer_"),
            state_var: next_ident("_d"),
            gen_var: next_ident("_g"),
            fb_var: next_ident("_c"),
            stop_var: next_ident("_s"),
            fps_var: next_ident("_r"),
            frames_var: next_ident("_f"),
            err_var: next_ident("_e"),
        }
    }

    pub fn build_install_js(&self) -> String {
        let id = &self.canvas_id;
        let st = &self.state_var;
        let gn = &self.gen_var;
        let fb = &self.fb_var;
        let sp = &self.stop_var;
        let fp = &self.fps_var;
        let fr = &self.frames_var;
        let er = &self.err_var;

        format!(
            r#"(function(){{
  var ID='{id}';
  window.{gn}=(window.{gn}||0)+1;
  var GEN=window.{gn};
  if(window.{sp}){{try{{window.{sp}();}}catch(e){{}}}}
  document.querySelectorAll('#'+ID).forEach(function(n){{try{{n.style.display='none';n.style.visibility='hidden';}}catch(e){{}}}});
  var c=document.createElement('canvas');
  c.id=ID;
  c.style.cssText='position:fixed!important;left:0!important;top:0!important;width:100vw!important;height:100vh!important;z-index:2147483647!important;pointer-events:none!important;display:block!important;';
  function fit(){{var w=(window.innerWidth||1920),h=(window.innerHeight||1080); if(c.width!==w)c.width=w; if(c.height!==h)c.height=h;}}
  fit();
  var g=c.getContext('2d');
  if(GEN===window.{gn}&&!window.{fb}){{window.{st}=window.{st}||{{boxes:[],lines:[],text:[],fills:[]}};}}
  window.{fb}=c;
  window.{fp}=0; window.{fr}=0; window.{er}='';
  var stop=false,lastT=performance.now(),frames=0,mo=null;
  function die(){{stop=true;try{{if(mo)mo.disconnect();}}catch(e){{}}try{{c.remove();}}catch(e){{}}}}
  window.{sp}=function(){{die();}};
  function loop(t){{
    if(stop) return;
    if(GEN!==window.{gn}||window.{fb}!==c){{die();return;}}
    try{{
      fit();
      g.clearRect(0,0,c.width,c.height);
      var d=window.{st}||{{}};
      g.lineWidth=1.2;
      g.font='bold 14px Consolas, monospace';
      g.textBaseline='top';
      g.textAlign='left';
      (d.fills||[]).forEach(function(f){{g.fillStyle=f.c||'#000';g.fillRect(f.x,f.y,f.w,f.h);}});
      (d.lines||[]).forEach(function(l){{g.strokeStyle=l.c||'#ffff00';g.beginPath();g.moveTo(l.x1,l.y1);g.lineTo(l.x2,l.y2);g.stroke();}});
      (d.boxes||[]).forEach(function(b){{g.strokeStyle=b.c||'#00ff00';g.strokeRect(b.x,b.y,b.w,b.h);}});
      (d.text||[]).forEach(function(s){{g.fillStyle=s.c||'#fff';g.fillText(s.s,s.x,s.y);}});
      frames++;
      if(t-lastT>=1000){{window.{fp}=Math.round(frames*1000/(t-lastT));frames=0;lastT=t;}}
      window.{fr}++;
    }}catch(e){{window.{er}=String(e);}}
    requestAnimationFrame(loop);
  }}
  function ensure(){{
    if(stop||GEN!==window.{gn}||window.{fb}!==c) return;
    var all=document.querySelectorAll('#'+ID);
    for(var i=0;i<all.length;i++){{
      var n=all[i];
      if(n===c) continue;
      try{{ n.style.display='none'; n.style.visibility='hidden'; }}catch(e){{}}
    }}
    if(!c.isConnected){{(document.body||document.documentElement).appendChild(c);}}
  }}
  ensure();
  setTimeout(ensure,120); setTimeout(ensure,600); setTimeout(ensure,1500);
  try{{mo=new MutationObserver(function(){{ensure();}});mo.observe(document.documentElement,{{childList:true,subtree:true}});}}catch(e){{}}
  requestAnimationFrame(loop);
  return JSON.stringify({{ok:true,href:location.href,w:c.width,h:c.height,gen:GEN}});
}})()"#
        )
    }

    #[inline]
    pub fn update_js(&self, payload: &str) -> String {
        format!("window.{}={payload};0", self.state_var)
    }

    #[inline]
    pub fn clear_js(&self) -> String {
        format!(
            "window.{}={{boxes:[],lines:[],text:[],fills:[]}};0",
            self.state_var
        )
    }

    #[inline]
    pub fn teardown_js(&self) -> String {
        format!(
            "(function(){{if(window.{sp})window.{sp}();var c=document.getElementById('{id}');if(c)c.remove();return 1;}})()",
            sp = self.stop_var,
            id = self.canvas_id
        )
    }
}

// ---------------------------------------------------------------------------
// ESP geometrisi -> overlay primitifleri
// ---------------------------------------------------------------------------

fn rgba(r: u8, g: u8, b: u8, a: u8) -> String {
    if a >= 255 {
        format!("rgb({r},{g},{b})")
    } else {
        format!("rgba({r},{g},{b},{:.2})", a as f32 / 255.0)
    }
}

/// Ortak `crate::esp::build_esp_draw_list` çıktısını ve `crate::hud::HudModel`
/// verisini Discord canvas primitiflerine çevirir. Koordinatlar piksel, sol-üst orijinli.
pub fn build_primitives(
    snapshot: &EspSnapshot,
    settings: &VisualSettings,
    display_w: f32,
    display_h: f32,
    hud_model: Option<&hud::HudModel>,
    esp_enabled: bool,
) -> String {
    let mut boxes: Vec<serde_json::Value> = Vec::new();
    let mut lines: Vec<serde_json::Value> = Vec::new();
    let mut texts: Vec<serde_json::Value> = Vec::new();
    let mut fills: Vec<serde_json::Value> = Vec::new();

    if display_w <= 0.0 || display_h <= 0.0 {
        return "{\"boxes\":[],\"lines\":[],\"text\":[],\"fills\":[]}".to_string();
    }

    if esp_enabled {
        let cmds = esp::build_esp_draw_list(snapshot, settings, display_w, display_h);

        for f in cmds.filled_rects {
            fills.push(serde_json::json!({
                "x": f.x, "y": f.y, "w": f.w, "h": f.h, "c": f.color.to_css()
            }));
        }

        for l in cmds.lines {
            lines.push(serde_json::json!({
                "x1": l.x1, "y1": l.y1, "x2": l.x2, "y2": l.y2, "c": l.color.to_css()
            }));
        }

        for c in cmds.circles {
            let steps = 8;
            let col_str = c.color.to_css();
            for i in 0..steps {
                let a1 = (i as f32) * std::f32::consts::TAU / steps as f32;
                let a2 = ((i + 1) as f32) * std::f32::consts::TAU / steps as f32;
                lines.push(serde_json::json!({
                    "x1": c.cx + c.radius * a1.cos(), "y1": c.cy + c.radius * a1.sin(),
                    "x2": c.cx + c.radius * a2.cos(), "y2": c.cy + c.radius * a2.sin(),
                    "c": col_str
                }));
            }
        }

        for r in cmds.rects {
            boxes.push(serde_json::json!({
                "x": r.x, "y": r.y, "w": r.w, "h": r.h, "c": r.color.to_css()
            }));
        }

        for t in cmds.texts {
            texts.push(serde_json::json!({
                "x": t.x, "y": t.y, "s": t.text, "c": t.color.to_css()
            }));
        }
    }

    // ===== HUD gösterge paneli (Discord canvas) =====
    // İçerik `crate::hud::build` ile üretilir; ImGui yoluyla birebir aynıdır.
    if let Some(hud_model) = hud_model {
        let x0 = hud::ORIGIN[0];
        let y0 = hud::ORIGIN[1];
        let pad = hud::PADDING;
        let row_h = hud::ROW_HEIGHT;
        let width = hud::PANEL_WIDTH;
        let height = hud_model.height();
        let x1 = x0 + width;
        let y1 = y0 + height;
        let text_x = x0 + pad + 4.0;

        // Zemin + sol aksan
        fills.push(serde_json::json!({
            "x": x0, "y": y0, "w": width, "h": height, "c": rgba(11, 13, 17, 210)
        }));
        fills.push(serde_json::json!({
            "x": x0, "y": y0, "w": 3.0, "h": height, "c": rgba(65, 160, 255, 255)
        }));

        // Kenarlık
        for (a, b) in [
            ((x0, y0), (x1, y0)),
            ((x1, y0), (x1, y1)),
            ((x1, y1), (x0, y1)),
            ((x0, y1), (x0, y0)),
        ] {
            lines.push(serde_json::json!({
                "x1": a.0, "y1": a.1, "x2": b.0, "y2": b.1,
                "c": rgba(70, 80, 95, 160)
            }));
        }

        let mut y = y0 + pad;
        texts.push(serde_json::json!({
            "x": text_x, "y": y, "s": hud_model.header, "c": rgba(65, 160, 255, 255)
        }));
        y += row_h;

        // Canvas fontu bold 14px Consolas: genişlik ~8.4px/karakter (sağa hizalama için).
        const CHAR_W: f32 = 8.4;
        for row in &hud_model.rows {
            texts.push(serde_json::json!({
                "x": text_x, "y": y, "s": row.label, "c": rgba(150, 158, 170, 255)
            }));
            let value_color = if row.on {
                rgba(90, 220, 120, 255)
            } else {
                rgba(230, 90, 90, 255)
            };
            let value_w = CHAR_W * row.detail.chars().count() as f32;
            texts.push(serde_json::json!({
                "x": x1 - pad - value_w, "y": y, "s": row.detail, "c": value_color
            }));
            y += row_h;
        }

        y += 3.0;
        texts.push(serde_json::json!({
            "x": text_x, "y": y, "s": hud_model.footer, "c": rgba(105, 112, 124, 255)
        }));
    }

    serde_json::json!({
        "boxes": boxes, "lines": lines, "text": texts, "fills": fills
    })
    .to_string()
}

/// Oyun olmadan Discord overlay köprüsünü doğrular:
/// `cs2ru.exe --discord-selftest`
pub fn selftest() {
    let port = cdp_port();
    let tokens = SessionTokens::random();
    println!("[*] Discord overlay self-test başlıyor (CDP :{port})...");
    let (mut sock, _size) = match connect_and_install(port, &tokens) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[!] Bağlantı kurulamadı: {e}");
            eprintln!("    Discord '--remote-debugging-port={port}' ile açık mı?");
            std::process::exit(1);
        }
    };
    println!("[+] Canvas enjekte edildi (id={}). 5 sn animasyon gönderiliyor...", tokens.canvas_id);

    let t0 = std::time::Instant::now();
    let mut n: u32 = 0;
    while t0.elapsed() < Duration::from_secs(5) {
        let tt = t0.elapsed().as_secs_f32();
        let x = 200.0 + (tt * 220.0) % 1100.0;
        let payload = format!(
            "{{\"boxes\":[{{\"x\":{x},\"y\":420,\"w\":120,\"h\":60,\"c\":\"#00ff00\"}}],\
              \"lines\":[{{\"x1\":960,\"y1\":540,\"x2\":{x},\"y2\":450,\"c\":\"#ffff00\"}}],\
              \"text\":[{{\"x\":40,\"y\":60,\"s\":\"selftest #{n}\",\"c\":\"#00ff00\"}}],\
              \"fills\":[]}}"
        );
        if let Err(e) = sock.eval(&tokens.update_js(&payload)) {
            eprintln!("[!] Gönderim hatası: {e}");
            break;
        }
        n += 1;
        thread::sleep(Duration::from_millis(16));
    }

    let report_js = format!(
        "JSON.stringify({{fps:window.{},frames:window.{},err:window.{}||''}})",
        tokens.fps_var, tokens.frames_var, tokens.err_var
    );
    match sock.eval(&report_js) {
        Ok(v) => println!("[+] Sayfa raporu: {v}"),
        Err(e) => eprintln!("[!] Sayfa raporu alınamadı: {e}"),
    }
    println!("[+] {n} kare gönderildi. Self-test bitti (çizim ekranda kalıyor).");
}


fn connect_and_install(port: u16, tokens: &SessionTokens) -> Result<(CdpSocket, (f32, f32)), String> {
    let ws = find_overlay_target(port)?;
    let mut sock = CdpSocket::connect(&ws)?;
    sock.eval(&tokens.build_install_js())?;
    // Overlay sayfasının gerçek ölçüsü (CSS piksel) — koordinatlar buna göre üretilir
    let size_js = "JSON.stringify({w:window.innerWidth||1920,h:window.innerHeight||1080})";
    let size = sock
        .eval(size_js)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .map(|v| {
            (
                v.get("w").and_then(|x| x.as_f64()).unwrap_or(1920.0) as f32,
                v.get("h").and_then(|x| x.as_f64()).unwrap_or(1080.0) as f32,
            )
        })
        .unwrap_or((1920.0, 1080.0));
    Ok((sock, size))
}

/// Snapshot'ı okuyup Discord overlay canvas'ına 60 Hz'de ESP basan thread başlatır.
/// Discord/CDP yoksa sessizce yeniden bağlanmayı dener (oyunu etkilemez).
pub fn spawn(
    snapshot: Arc<RwLock<EspSnapshot>>,
    settings: Arc<RwLock<VisualSettings>>,
    visual_esp_enabled: Arc<AtomicBool>,
    sound_esp_enabled: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
    mode_controller: ModeController,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name(obfstr!("worker-ipc").into())
        .spawn(move || {
            let port = cdp_port();
            let tokens = SessionTokens::random();
            println!(
                "[*] Discord overlay köprüsü aranıyor (CDP :{port}, dom=#{})...",
                tokens.canvas_id
            );
            let mut sock: Option<CdpSocket> = None;
            let mut page_size: (f32, f32) = (1920.0, 1080.0);
            let mut dbg_tick = std::time::Instant::now();
            let mut last_err = String::new();
            let mut was_dormant = false;
            let mut was_dormant_game = false;
            let interval = Duration::from_millis(1000 / SEND_HZ);

            while running.load(Ordering::Relaxed) {
                let start = std::time::Instant::now();

                // 1. Mod Kontrolü (Mutual Exclusion):
                // Mod 1 (Basic) ve Mod 2 (DirectStealth) modlarında Discord Overlay tamamen dormant kalır.
                if !mode_controller.is_discord_active() {
                    if !was_dormant {
                        if let Some(s) = sock.as_mut() {
                            let _ = s.eval(&tokens.clear_js());
                        }
                        was_dormant = true;
                    }
                    thread::sleep(Duration::from_millis(50));
                    continue;
                }
                was_dormant = false;

                if sock.is_none() {
                    match connect_and_install(port, &tokens) {
                        Ok((s, size)) => {
                            println!(
                                "[+] Discord overlay canvas enjekte edildi (sayfa {}x{}).",
                                size.0 as u32, size.1 as u32
                            );
                            page_size = size;
                            sock = Some(s);
                            last_err.clear();
                        }
                        Err(e) => {
                            if e != last_err {
                                println!("[!] Discord overlay bağlantısı yok: {e}");
                                println!("    (Discord '--remote-debugging-port={port}' ile açık olmalı)");
                                last_err = e;
                            }
                            thread::sleep(Duration::from_secs(2));
                            continue;
                        }
                    }
                }

                // 2. In-Game Kontrolü: Oyun içinde değilken (menü/yükleme) çizim gönderme
                let is_in_game = snapshot.read().map(|s| s.is_in_game).unwrap_or(false);
                if !is_in_game {
                    if !was_dormant_game {
                        if let Some(s) = sock.as_mut() {
                            let _ = s.eval(&tokens.clear_js());
                        }
                        was_dormant_game = true;
                    }
                    thread::sleep(Duration::from_millis(50));
                    continue;
                }
                was_dormant_game = false;

                let esp_on = visual_esp_enabled.load(Ordering::Relaxed);

                let payload = match (snapshot.read(), settings.read()) {
                    (Ok(snap), Ok(set)) => {
                        // HUD paneli: mod seviyesi + özellik ON/OFF (visual ESP kapalıyken de görünür).
                        let hud_model = hud::build(
                            &mode_controller,
                            &set,
                            esp_on,
                            sound_esp_enabled.load(Ordering::Relaxed),
                        );
                        build_primitives(
                            &snap,
                            &set,
                            page_size.0,
                            page_size.1,
                            hud_model.as_ref(),
                            esp_on,
                        )
                    }
                    _ => String::from("{\"boxes\":[],\"lines\":[],\"text\":[],\"fills\":[]}"),
                };

                if dbg_tick.elapsed() >= Duration::from_secs(2) {
                    dbg_tick = std::time::Instant::now();
                    if let Some(s) = sock.as_mut() {
                        if let Ok(sz_str) = s.eval(
                            "JSON.stringify({w:window.innerWidth||1920,h:window.innerHeight||1080})",
                        ) {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&sz_str) {
                                let w = v.get("w").and_then(|x| x.as_f64()).unwrap_or(1920.0) as f32;
                                let h = v.get("h").and_then(|x| x.as_f64()).unwrap_or(1080.0) as f32;
                                if w > 0.0 && h > 0.0 {
                                    page_size = (w, h);
                                }
                            }
                        }
                    }
                    let n_players = snapshot.read().map(|s| s.players.len()).unwrap_or(0);
                    println!(
                        "[dbg] oyuncu={n_players} primitif_uzunluk={} ilk80={}",
                        payload.len(),
                        &payload[..payload.len().min(80)]
                    );
                }

                if let Some(s) = sock.as_mut() {
                    let js = tokens.update_js(&payload);
                    match s.eval(&js) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("[!] Discord overlay gönderimi koptu: {e}");
                            sock = None;
                            last_err.clear();
                            continue;
                        }
                    }
                }

                let elapsed = start.elapsed();
                if elapsed < interval {
                    thread::sleep(interval - elapsed);
                }
            }
            if let Some(mut s) = sock {
                let _ = s.eval(&tokens.teardown_js());
            }
            println!("[*] Discord overlay thread'i sonlandı.");
        })
        .expect("discord overlay thread'i başlatılamadı")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{EspSnapshot, VisualSettings};
    use crate::mode::{ModeController, StealthMode};

    fn test_snapshot() -> EspSnapshot {
        EspSnapshot {
            view_matrix: crate::matrix::ViewMatrix::default(),
            local_team: 2,
            has_local: true,
            is_in_game: true,
            map_name: "de_test".to_string(),
            players: Vec::new(),
            planted_c4: None,
        }
    }

    /// Discord canvas yolu, ImGui yolunun ürettiği HUD modelini birebir tüketmeli:
    /// aynı satır etiketleri ve aynı ON/OFF detayları payload'da bulunmalı.
    #[test]
    fn discord_payload_renders_same_hud_model_as_imgui() {
        let mc = ModeController::new(StealthMode::Indirect);
        let settings = VisualSettings::default_for_mode(sysmemkit::Mode::Indirect);
        let model = hud::build(&mc, &settings, true, false).expect("HUD modeli üretilmeli");

        let json = build_primitives(
            &test_snapshot(),
            &settings,
            1920.0,
            1080.0,
            Some(&model),
            false, // ESP kapalı: HUD yine de çizilmeli
        );
        let value: serde_json::Value = serde_json::from_str(&json).expect("payload JSON olmalı");

        let texts: Vec<String> = value["text"]
            .as_array()
            .expect("text dizisi")
            .iter()
            .map(|t| t["s"].as_str().unwrap_or_default().to_string())
            .collect();

        assert!(!value["fills"].as_array().unwrap().is_empty(), "panel zemini çizilmeli");
        assert!(!texts.is_empty());

        // Başlık ve satırlar ImGui modelindeki içerikle aynı olmalı.
        assert!(texts.iter().any(|t| t.contains(&model.header)), "başlık eksik: {:?}", model.header);
        for row in &model.rows {
            assert!(
                texts.iter().any(|t| t.contains(row.label)),
                "satır etiketi eksik: {} (payload: {:?})",
                row.label,
                texts
            );
            assert!(
                texts.iter().any(|t| t.contains(&row.detail)),
                "satır detayı eksik: {}/{}",
                row.label,
                row.detail
            );
        }

        // Determinizm: aynı model + aynı snapshot aynı payload'ı üretir.
        let again = build_primitives(
            &test_snapshot(),
            &settings,
            1920.0,
            1080.0,
            Some(&model),
            false,
        );
        assert_eq!(json, again);
    }

    /// HUD kapalıyken payload'da panel olmamalı (süreçler karışmamalı).
    #[test]
    fn payload_has_no_hud_when_disabled() {
        let mc = ModeController::new(StealthMode::Indirect);
        let settings = VisualSettings::default_for_mode(sysmemkit::Mode::Indirect);
        mc.set_hud_visible(false);
        assert!(hud::build(&mc, &settings, true, false).is_none());

        let json = build_primitives(&test_snapshot(), &settings, 1920.0, 1080.0, None, false);
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(value["text"].as_array().unwrap().is_empty());
        assert!(value["fills"].as_array().unwrap().is_empty());
    }

    #[test]
    fn session_tokens_are_unique_and_signature_free() {
        let t1 = SessionTokens::random();
        let t2 = SessionTokens::random();
        assert_ne!(t1.canvas_id, t2.canvas_id);
        assert_ne!(t1.state_var, t2.state_var);

        let js = t1.build_install_js();
        assert!(!js.contains("__hermes"));
        assert!(!js.contains("__ESP"));
        assert!(js.contains(&t1.canvas_id));
        assert!(js.contains(&t1.state_var));
    }
}

