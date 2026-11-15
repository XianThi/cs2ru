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
use crate::vec3::Vec3;

/// Ana Discord süreci `--remote-debugging-port=9222` ile başlatılmalı.
const CDP_PORT: u16 = 9222;
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
            .strip_prefix("ws://")
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
            "GET {path} HTTP/1.1\r\n\
             Host: {hostport}\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Key: {key}\r\n\
             Sec-WebSocket-Version: 13\r\n\
             User-Agent: cs2ru-overlay\r\n\r\n"
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

        let msg = serde_json::json!({
            "id": id,
            "method": "Runtime.evaluate",
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
            if v.get("id").and_then(|x| x.as_u64()) != Some(id) {
                continue;
            }
            if let Some(err) = v.get("error") {
                return Err(format!("CDP hatası: {err}"));
            }
            let result = v.get("result").cloned().unwrap_or(serde_json::Value::Null);
            if let Some(ex) = result.get("exceptionDetails") {
                return Err(format!("JS hatası: {ex}"));
            }
            return Ok(result
                .get("result")
                .and_then(|r| r.get("value"))
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
/// `webSocketDebuggerUrl` adresini seçer.
fn find_overlay_target(port: u16) -> Result<String, String> {
    let url = format!("http://127.0.0.1:{port}/json/list");
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
        let ty = t.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if ty != "page" {
            continue;
        }
        let ws = match t.get("webSocketDebuggerUrl").and_then(|v| v.as_str()) {
            Some(w) => w.to_string(),
            None => continue,
        };
        let title = t.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let turl = t.get("url").and_then(|v| v.as_str()).unwrap_or("");
        if title.to_ascii_lowercase().contains("overlay") || turl.contains("popout") {
            return Ok(ws);
        }
        if fallback.is_none() {
            fallback = Some(ws);
        }
    }
    fallback.ok_or_else(|| "CDP'de uygun 'page' target bulunamadı".to_string())
}

// ---------------------------------------------------------------------------
// Enjeksiyon betiği (canvas + rAF döngüsü)
// ---------------------------------------------------------------------------

pub const INSTALL_JS: &str = r#"
(function(){
  var ID='__hermes_fb';
  // Nesil sayacı: eski örnekler kendini imha eder, üst üste katman kalmaz.
  window.__hermesGen=(window.__hermesGen||0)+1;
  var GEN=window.__hermesGen;
  if(window.__hermesStop){try{window.__hermesStop();}catch(e){}}
  // Eski kopyaları DOM'dan silmek eski MutationObserver'ları tetikleyip sonsuz
  // ping-pong'a yol açıyor; gizlemek yeterli ve güvenli.
  document.querySelectorAll('#'+ID).forEach(function(n){try{n.style.display='none';n.style.visibility='hidden';}catch(e){}});
  var c=document.createElement('canvas');
  c.id=ID;
  c.style.cssText='position:fixed!important;left:0!important;top:0!important;width:100vw!important;height:100vh!important;z-index:2147483647!important;pointer-events:none!important;display:block!important;';
  function fit(){var w=(window.innerWidth||1920),h=(window.innerHeight||1080); if(c.width!==w)c.width=w; if(c.height!==h)c.height=h;}
  fit();
  var g=c.getContext('2d');
  if(GEN===window.__hermesGen&&!window.__hermesFb){window.__ESP=window.__ESP||{boxes:[],lines:[],text:[],fills:[]};}
  window.__hermesFb=c;
  window.__fps=0; window.__frames=0; window.__hermesErr='';
  var stop=false,lastT=performance.now(),frames=0,mo=null;
  function die(){stop=true;try{if(mo)mo.disconnect();}catch(e){}try{c.remove();}catch(e){}}
  window.__hermesStop=function(){die();};
  function loop(t){
    if(stop) return;
    if(GEN!==window.__hermesGen||window.__hermesFb!==c){die();return;}
    try{
      fit();
      g.clearRect(0,0,c.width,c.height);
      var d=window.__ESP||{};
      g.lineWidth=1.2;
      g.font='bold 14px Consolas, monospace';
      // ImGui ile aynı referansı kullan: aksi halde iki katman üst üste binmeyip
      // "ghost" (çift yazı) gibi görünür.
      g.textBaseline='top';
      g.textAlign='left';
      (d.fills||[]).forEach(function(f){g.fillStyle=f.c||'#000';g.fillRect(f.x,f.y,f.w,f.h);});
      (d.lines||[]).forEach(function(l){g.strokeStyle=l.c||'#ffff00';g.beginPath();g.moveTo(l.x1,l.y1);g.lineTo(l.x2,l.y2);g.stroke();});
      (d.boxes||[]).forEach(function(b){g.strokeStyle=b.c||'#00ff00';g.strokeRect(b.x,b.y,b.w,b.h);});
      (d.text||[]).forEach(function(s){g.fillStyle=s.c||'#fff';g.fillText(s.s,s.x,s.y);});
      frames++;
      if(t-lastT>=1000){window.__fps=Math.round(frames*1000/(t-lastT));frames=0;lastT=t;}
      window.__frames++;
    }catch(e){window.__hermesErr=String(e);}
    requestAnimationFrame(loop);
  }
  function ensure(){
    if(stop||GEN!==window.__hermesGen||window.__hermesFb!==c) return;
    // Eski enjeksiyonlardan kalan canvas'ları DOM'dan silmek MutationObserver'lar
    // arasında ping-pong yaratıp sayfayı kilitliyor; onun yerine sadece gizle.
    var all=document.querySelectorAll('#'+ID);
    for(var i=0;i<all.length;i++){
      var n=all[i];
      if(n===c) continue;
      try{ n.style.display='none'; n.style.visibility='hidden'; }catch(e){}
    }
    if(!c.isConnected){(document.body||document.documentElement).appendChild(c);}
  }
  ensure();
  setTimeout(ensure,120); setTimeout(ensure,600); setTimeout(ensure,1500);
  try{mo=new MutationObserver(function(){ensure();});mo.observe(document.documentElement,{childList:true,subtree:true});}catch(e){}
  requestAnimationFrame(loop);
  return JSON.stringify({ok:true,href:location.href,w:c.width,h:c.height,gen:GEN});
})()
"#;

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

/// ImGui çizim mantığının aynısını (kutu, hp barı, isim, silah, iskelet)
/// canvas primitiflerine çevirir. Koordinatlar piksel, sol-üst orijinli.
pub fn build_primitives(
    snapshot: &EspSnapshot,
    settings: &VisualSettings,
    display_w: f32,
    display_h: f32,
) -> String {
    let mut boxes: Vec<serde_json::Value> = Vec::new();
    let mut lines: Vec<serde_json::Value> = Vec::new();
    let mut texts: Vec<serde_json::Value> = Vec::new();
    let mut fills: Vec<serde_json::Value> = Vec::new();

    if display_w <= 0.0 || display_h <= 0.0 {
        return "{\"boxes\":[],\"lines\":[],\"text\":[],\"fills\":[]}".to_string();
    }

    for player in &snapshot.players {
        if player.health <= 0 {
            continue;
        }
        let is_enemy = snapshot.local_team != 0 && player.team != snapshot.local_team;
        let base_color = if is_enemy {
            rgba(255, 65, 65, 255)
        } else {
            rgba(65, 160, 255, 255)
        };

        let head_top = player.head_pos + Vec3::new(0.0, 0.0, 8.0);
        let (fx, fy) = match snapshot
            .view_matrix
            .world_to_screen(&player.pos, display_w, display_h)
        {
            Some(p) => p,
            None => continue,
        };
        let (_hx, hy) = match snapshot
            .view_matrix
            .world_to_screen(&head_top, display_w, display_h)
        {
            Some(p) => p,
            None => continue,
        };

        let box_height = (fy - hy).abs();
        if box_height < 4.0 || box_height > display_h * 2.5 {
            continue;
        }
        let box_width = box_height * 0.45;
        let min_x = fx - box_width / 2.0;
        let min_y = hy.min(fy);
        let max_y = hy.max(fy);

        if settings.is_minimal {
            if settings.show_box {
                boxes.push(serde_json::json!({
                    "x": min_x, "y": min_y, "w": box_width, "h": box_height, "c": base_color
                }));
            }
            continue;
        }

        if settings.show_box {
            // Dış/iç siyah kenarlık + renkli çerçeve
            boxes.push(serde_json::json!({"x":min_x-1.0,"y":min_y-1.0,"w":box_width+2.0,"h":box_height+2.0,"c":rgba(0,0,0,255)}));
            boxes.push(serde_json::json!({"x":min_x,"y":min_y,"w":box_width,"h":box_height,"c":base_color}));
            boxes.push(serde_json::json!({"x":min_x+1.0,"y":min_y+1.0,"w":box_width-2.0,"h":box_height-2.0,"c":rgba(0,0,0,255)}));
        }

        if settings.show_health {
            let bar_w = (box_width * 0.08).clamp(2.0, 4.0);
            let bar_x = min_x - 4.0 - bar_w;
            fills.push(serde_json::json!({
                "x": bar_x - 1.0, "y": min_y - 1.0, "w": bar_w + 2.0, "h": box_height + 2.0,
                "c": rgba(0, 0, 0, 220)
            }));
            let hp_pct = (player.health.clamp(0, 100) as f32) / 100.0;
            let hp_h = box_height * hp_pct;
            let hp_top_y = max_y - hp_h;
            let hp_color = if player.health > 60 {
                rgba(50, 220, 50, 255)
            } else if player.health > 25 {
                rgba(240, 180, 20, 255)
            } else {
                rgba(240, 40, 40, 255)
            };
            fills.push(serde_json::json!({
                "x": bar_x, "y": hp_top_y, "w": bar_w, "h": hp_h, "c": hp_color
            }));
            if player.health < 100 {
                texts.push(serde_json::json!({
                    "x": bar_x - 16.0, "y": hp_top_y - 2.0, "s": player.health.to_string(),
                    "c": rgba(255, 255, 255, 255)
                }));
            }
        }

        if settings.show_name && !player.name.is_empty() {
            texts.push(serde_json::json!({
                "x": min_x, "y": min_y - 14.0, "s": player.name, "c": rgba(255, 255, 255, 255)
            }));
        }
        if settings.show_weapon && !player.weapon.is_empty() {
            texts.push(serde_json::json!({
                "x": min_x, "y": max_y + 2.0, "s": player.weapon, "c": rgba(210, 210, 210, 255)
            }));
        }

        if settings.show_skeleton && player.bone_list.len() >= 28 {
            let sc = rgba(255, 255, 255, 200);
            let pairs: &[(usize, usize)] = &[
                (6, 5),
                (5, 4),
                (4, 2),
                (2, 0),
                (5, 8),
                (8, 9),
                (9, 10),
                (5, 13),
                (13, 14),
                (14, 15),
                (0, 22),
                (22, 23),
                (23, 24),
                (0, 25),
                (25, 26),
                (26, 27),
            ];
            for &(b1, b2) in pairs {
                let p1 = snapshot
                    .view_matrix
                    .world_to_screen(&player.bone_list[b1], display_w, display_h);
                let p2 = snapshot
                    .view_matrix
                    .world_to_screen(&player.bone_list[b2], display_w, display_h);
                if let (Some(a), Some(b)) = (p1, p2) {
                    lines.push(serde_json::json!({
                        "x1": a.0, "y1": a.1, "x2": b.0, "y2": b.1, "c": sc
                    }));
                }
            }
            if let Some(h) = snapshot
                .view_matrix
                .world_to_screen(&player.bone_list[6], display_w, display_h)
            {
                // Kafa: küçük daireyi 4 çizgiyle yaklaşık çiz
                let r = (box_height * 0.07).clamp(2.5, 7.0);
                let steps = 8;
                for i in 0..steps {
                    let a1 = (i as f32) * std::f32::consts::TAU / steps as f32;
                    let a2 = ((i + 1) as f32) * std::f32::consts::TAU / steps as f32;
                    lines.push(serde_json::json!({
                        "x1": h.0 + r * a1.cos(), "y1": h.1 + r * a1.sin(),
                        "x2": h.0 + r * a2.cos(), "y2": h.1 + r * a2.sin(),
                        "c": sc
                    }));
                }
            }
        }
    }

    serde_json::json!({
        "boxes": boxes, "lines": lines, "text": texts, "fills": fills
    })
    .to_string()
}

/// Oyun olmadan Discord overlay köprüsünü doğrular:
/// `cs2ru.exe --discord-selftest`
pub fn selftest() {
    println!("[*] Discord overlay self-test başlıyor (CDP :{CDP_PORT})...");
    let (mut sock, _size) = match connect_and_install(CDP_PORT) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[!] Bağlantı kurulamadı: {e}");
            eprintln!("    Discord '--remote-debugging-port={CDP_PORT}' ile açık mı?");
            std::process::exit(1);
        }
    };
    println!("[+] Canvas enjekte edildi. 5 sn animasyon gönderiliyor...");

    let t0 = std::time::Instant::now();
    let mut n: u32 = 0;
    while t0.elapsed() < Duration::from_secs(5) {
        let tt = t0.elapsed().as_secs_f32();
        let x = 200.0 + (tt * 220.0) % 1100.0;
        let payload = format!(
            "{{\"boxes\":[{{\"x\":{x},\"y\":420,\"w\":120,\"h\":60,\"c\":\"#00ff00\"}}],\
              \"lines\":[{{\"x1\":960,\"y1\":540,\"x2\":{x},\"y2\":450,\"c\":\"#ffff00\"}}],\
              \"text\":[{{\"x\":40,\"y\":60,\"s\":\"cs2ru selftest #{n}\",\"c\":\"#00ff00\"}}],\
              \"fills\":[]}}"
        );
        if let Err(e) = sock.eval(&format!("window.__ESP={payload};0")) {
            eprintln!("[!] Gönderim hatası: {e}");
            break;
        }
        n += 1;
        thread::sleep(Duration::from_millis(16));
    }

    match sock.eval("JSON.stringify({fps:window.__fps,frames:window.__frames,err:window.__hermesErr||''})")
    {
        Ok(v) => println!("[+] Sayfa raporu: {v}"),
        Err(e) => eprintln!("[!] Sayfa raporu alınamadı: {e}"),
    }
    println!("[+] {n} kare gönderildi. Self-test bitti (çizim ekranda kalıyor).");
}


fn connect_and_install(port: u16) -> Result<(CdpSocket, (f32, f32)), String> {
    let ws = find_overlay_target(port)?;
    let mut sock = CdpSocket::connect(&ws)?;
    sock.eval(INSTALL_JS)?;
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
    running: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("cs2ru-discord-overlay".into())
        .spawn(move || {
            println!("[*] Discord overlay köprüsü aranıyor (CDP :{CDP_PORT})...");
            let mut sock: Option<CdpSocket> = None;
            let mut page_size: (f32, f32) = (1920.0, 1080.0);
            let mut dbg_tick = std::time::Instant::now();
            let mut last_err = String::new();
            let mut announced = false;
            let interval = Duration::from_millis(1000 / SEND_HZ);

            while running.load(Ordering::Relaxed) {
                let start = std::time::Instant::now();

                if sock.is_none() {
                    match connect_and_install(CDP_PORT) {
                        Ok((s, size)) => {
                            println!(
                                "[+] Discord overlay canvas enjekte edildi (sayfa {}x{}).",
                                size.0 as u32, size.1 as u32
                            );
                            page_size = size;
                            sock = Some(s);
                            announced = true;
                        }
                        Err(e) => {
                            if e != last_err {
                                println!("[!] Discord overlay bağlantısı yok: {e}");
                                println!("    (Discord '--remote-debugging-port={CDP_PORT}' ile açık olmalı)");
                                last_err = e;
                            }
                            thread::sleep(Duration::from_secs(2));
                            continue;
                        }
                    }
                }

                if !visual_esp_enabled.load(Ordering::Relaxed) {
                    if announced {
                        if let Some(s) = sock.as_mut() {
                            let _ = s.eval("window.__ESP={boxes:[],lines:[],text:[],fills:[]};0");
                        }
                    }
                    thread::sleep(Duration::from_millis(250));
                    continue;
                }

                let payload = match (snapshot.read(), settings.read()) {
                    (Ok(snap), Ok(set)) => {
                        build_primitives(&snap, &set, page_size.0, page_size.1)
                    }
                    _ => String::from("{\"boxes\":[],\"lines\":[],\"text\":[],\"fills\":[]}"),
                };

                if dbg_tick.elapsed() >= Duration::from_secs(2) {
                    dbg_tick = std::time::Instant::now();
                    let n_players = snapshot.read().map(|s| s.players.len()).unwrap_or(0);
                    println!(
                        "[dbg] oyuncu={n_players} primitif_uzunluk={} ilk80={}",
                        payload.len(),
                        &payload[..payload.len().min(80)]
                    );
                }

                if let Some(s) = sock.as_mut() {
                    let js = format!("window.__ESP={payload};0");
                    match s.eval(&js) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("[!] Discord overlay gönderimi koptu: {e}");
                            sock = None;
                            announced = false;
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
                let _ = s.eval("(function(){if(window.__hermesStop)window.__hermesStop();var c=document.getElementById('__hermes_fb');if(c)c.remove();return 1;})()");
            }
            println!("[*] Discord overlay thread'i sonlandı.");
        })
        .expect("discord overlay thread'i başlatılamadı")
}
