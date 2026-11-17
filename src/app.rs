use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_F1, VK_F2, VK_F3, VK_F4, VK_F5, VK_F9, VK_F11, VK_INSERT, VK_LSHIFT,
    VK_RSHIFT, VK_SHIFT,
};

use overlay::{OverlayOptions, OverlayTarget};
use sysmemkit::Mode;

use crate::discord_overlay;
use crate::esp;
use crate::hud;
use crate::matrix::ViewMatrix;
use crate::memory::MemoryRouter;
use crate::mode::{ModeController, StealthMode};
use crate::offset::GameConfig;
use crate::player::Player;
use crate::sound::SoundEvent;
use crate::utils;
use crate::vec3::Vec3;

#[inline]
fn col(r: u8, g: u8, b: u8, a: u8) -> [f32; 4] {
    [
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        a as f32 / 255.0,
    ]
}

/// Overlay görselleştirme ayarları
#[derive(Debug, Clone, Copy)]
pub struct VisualSettings {
    /// Karakterin etrafında 2D sınırlayıcı kutu (mesafeye duyarlı küçülen boyutlandırma)
    pub show_box: bool,
    /// Kemik ve iskelet bağlantı hatları
    pub show_skeleton: bool,
    /// Sağlık çubuğu ve sayısal HP değeri
    pub show_health: bool,
    /// Oyuncu adı
    pub show_name: bool,
    /// Tutulan silah adı
    pub show_weapon: bool,
    /// Minimal çizim modu (Rage modunda sadece sade/minimal kutu, Legit modda tüm detaylar)
    pub is_minimal: bool,
}

impl Default for VisualSettings {
    fn default() -> Self {
        Self {
            show_box: true,
            show_skeleton: true,
            show_health: true,
            show_name: true,
            show_weapon: true,
            is_minimal: false,
        }
    }
}

impl VisualSettings {
    /// sysmemkit::Mode değerine göre başlangıç ayarlarını oluşturur:
    /// - Mode::Rage => Minimal kutu aktif, iskelet/isim/hp kapalı (performans ve sade ekran)
    /// - Diğer modlar (Legit) => Detaylı kutu, iskelet, HP ve isim açık
    pub fn default_for_mode(mode: Mode) -> Self {
        match mode {
            Mode::Rage => Self {
                show_box: true,
                show_skeleton: false,
                show_health: false,
                show_name: false,
                show_weapon: false,
                is_minimal: true,
            },
            _ => Self {
                show_box: true,
                show_skeleton: true,
                show_health: true,
                show_name: true,
                show_weapon: true,
                is_minimal: false,
            },
        }
    }
}

/// Render thread'ine aktarılacak tek bir oyuncunun anlık durumu
#[derive(Debug, Clone)]
pub struct PlayerSnapshot {
    pub pos: Vec3,
    pub head_pos: Vec3,
    pub bone_list: Vec<Vec3>,
    pub health: i32,
    pub armor: i32,
    pub team: i32,
    pub name: String,
    pub weapon: String,
    pub scoped: bool,
    pub defusing: bool,
    pub flashed: bool,
}

/// Kurulu C4 anlık durumu
#[derive(Debug, Clone, Default)]
pub struct PlantedC4Snapshot {
    pub pos: Vec3,
    pub site: i32,
    pub is_planted: bool,
}

/// Çift thread arasında paylaşılan dünya ve oyuncu snapshot'ı
#[derive(Debug, Clone, Default)]
pub struct EspSnapshot {
    pub view_matrix: ViewMatrix,
    pub local_team: i32,
    pub has_local: bool,
    pub is_in_game: bool,
    pub map_name: String,
    pub players: Vec<PlayerSnapshot>,
    pub planted_c4: Option<PlantedC4Snapshot>,
}

/// Kısayol tuşlarını dinleyen ve durumları güncelleyen işleyici
pub struct InputHandler {
    sound_esp_enabled: Arc<AtomicBool>,
    visual_esp_enabled: Arc<AtomicBool>,
    settings: Arc<RwLock<VisualSettings>>,
    mode_controller: ModeController,
    /// F9 ile mod değişince syscall katmanını da yeniden kurabilmek için.
    memory: Arc<MemoryRouter>,
    f1_was_down: bool,
    f2_was_down: bool,
    f3_was_down: bool,
    f4_was_down: bool,
    f5_was_down: bool,
    f9_was_down: bool,
    f11_was_down: bool,
    insert_was_down: bool,
}

impl InputHandler {
    pub fn new(
        sound_esp_enabled: Arc<AtomicBool>,
        visual_esp_enabled: Arc<AtomicBool>,
        settings: Arc<RwLock<VisualSettings>>,
        mode_controller: ModeController,
        memory: Arc<MemoryRouter>,
    ) -> Self {
        Self {
            sound_esp_enabled,
            visual_esp_enabled,
            settings,
            mode_controller,
            memory,
            f1_was_down: false,
            f2_was_down: false,
            f3_was_down: false,
            f4_was_down: false,
            f5_was_down: false,
            f9_was_down: false,
            f11_was_down: false,
            insert_was_down: false,
        }
    }

    #[inline]
    fn is_key_down(v_key: u16) -> bool {
        unsafe { (GetAsyncKeyState(v_key as i32) as u16 & 0x8000) != 0 }
    }

    pub fn update(&mut self) {
        let shift = Self::is_key_down(VK_SHIFT)
            || Self::is_key_down(VK_LSHIFT)
            || Self::is_key_down(VK_RSHIFT);
        let f1 = Self::is_key_down(VK_F1);
        let f2 = Self::is_key_down(VK_F2);
        let f3 = Self::is_key_down(VK_F3);
        let f4 = Self::is_key_down(VK_F4);
        let f5 = Self::is_key_down(VK_F5);

        // Shift + F1: Sound ESP Toggle
        if shift && f1 {
            if !self.f1_was_down {
                let cur = self.sound_esp_enabled.load(Ordering::Relaxed);
                self.sound_esp_enabled.store(!cur, Ordering::Relaxed);
                println!(
                    ">>> Sound ESP: {}",
                    if !cur { "AKTIF" } else { "KAPALI" }
                );
                self.f1_was_down = true;
            }
        } else {
            self.f1_was_down = false;
        }

        // Shift + F2: Visual ESP Toggle
        if shift && f2 {
            if !self.f2_was_down {
                let cur = self.visual_esp_enabled.load(Ordering::Relaxed);
                self.visual_esp_enabled.store(!cur, Ordering::Relaxed);
                println!(
                    ">>> Visual ESP (Overlay): {}",
                    if !cur { "AKTIF" } else { "KAPALI" }
                );
                self.f2_was_down = true;
            }
        } else {
            self.f2_was_down = false;
        }

        // Shift + F3: İskelet (Skeleton) Toggle
        if shift && f3 {
            if !self.f3_was_down {
                if let Ok(mut s) = self.settings.write() {
                    s.show_skeleton = !s.show_skeleton;
                    println!(
                        ">>> İskelet Çizimi: {}",
                        if s.show_skeleton { "AKTIF" } else { "KAPALI" }
                    );
                }
                self.f3_was_down = true;
            }
        } else {
            self.f3_was_down = false;
        }

        // Shift + F4: İsim & HP Barı Toggle
        if shift && f4 {
            if !self.f4_was_down {
                if let Ok(mut s) = self.settings.write() {
                    let new_val = !s.show_health;
                    s.show_health = new_val;
                    s.show_name = new_val;
                    s.show_weapon = new_val;
                    println!(
                        ">>> İsim / HP / Silah Detayları: {}",
                        if new_val { "AKTIF" } else { "KAPALI" }
                    );
                }
                self.f4_was_down = true;
            }
        } else {
            self.f4_was_down = false;
        }

        // Shift + F5: Minimal Mod Toggle (Rage vs Detaylı)
        if shift && f5 {
            if !self.f5_was_down {
                if let Ok(mut s) = self.settings.write() {
                    s.is_minimal = !s.is_minimal;
                    println!(
                        ">>> Minimal Çizim Modu (Rage Stili): {}",
                        if s.is_minimal {
                            "AKTIF (Sadece Sade Kutu)"
                        } else {
                            "KAPALI (Detaylı Çizim)"
                        }
                    );
                }
                self.f5_was_down = true;
            }
        } else {
            self.f5_was_down = false;
        }

        let f9 = Self::is_key_down(VK_F9);
        let f11 = Self::is_key_down(VK_F11);
        let insert = Self::is_key_down(VK_INSERT);

        // F9: Stealth Mod Seviye Değişimi (1 -> 2 -> 3 -> 4 -> 1)
        // Sıra önemli: önce syscall katmanı kurulur, başarılıysa mod commit edilir.
        // Böylece "overlay Discord'a geçti ama syscall hâlâ direct" gibi yarım durum oluşmaz.
        if f9 {
            if !self.f9_was_down {
                let target = self.mode_controller.mode().next();
                let sys_result = self.memory.apply_mode(target.to_sys_mode());
                println!();
                println!("==================================================");
                match sys_result {
                    Ok(effective) => {
                        let committed = StealthMode::from_sys_mode(effective);
                        self.mode_controller.set_mode(committed);
                        // ESP profili de moda bağlıdır (Rage -> minimal, Basic -> tam set).
                        {
                            let mut guard = self
                                .settings
                                .write()
                                .unwrap_or_else(|e| e.into_inner());
                            *guard = VisualSettings::default_for_mode(effective);
                        }
                        println!(">>> [MOD DEĞİŞTİRİLDİ]: {}", committed.name());
                        println!(">>> [AKTİF OVERLAY]    : {}", committed.overlay_name());
                        println!(">>> [SYSCALL KATMANI]  : {:?}", effective);
                        println!(
                            ">>> [NOT] Mod 3/4'te ImGui penceresi gizlenir; çizim Discord CDP üzerinden yapılır."
                        );
                    }
                    Err(err) => {
                        println!(">>> [MOD DEĞİŞTİRİLEMEDİ]: {}", target.name());
                        println!(">>> [SYSCALL HATASI]     : {}", err);
                        println!(">>> [NOT] Mod ve overlay yönlendirmesi değiştirilmedi.");
                    }
                }
                println!("==================================================");
                self.f9_was_down = true;
            }
        } else {
            self.f9_was_down = false;
        }

        // F11 veya Insert: HUD Gösterge Paneli Hide/Show Toggle
        if f11 || insert {
            if !self.f11_was_down && !self.insert_was_down {
                let visible = self.mode_controller.toggle_hud();
                println!(
                    ">>> [HUD GÖSTERGE PANELİ]: {}",
                    if visible { "GÖRÜNÜR" } else { "GİZLİ" }
                );
                self.f11_was_down = true;
                self.insert_was_down = true;
            }
        } else {
            self.f11_was_down = false;
            self.insert_was_down = false;
        }
    }
}

/// Model A Çift-Thread Uygulama Yöneticisi
pub struct App {
    pub config: GameConfig,
    pub memory: Arc<MemoryRouter>,
    pub clientdll: usize,
    pub sound_tx: Sender<SoundEvent>,
    pub sound_esp_enabled: Arc<AtomicBool>,
    pub visual_esp_enabled: Arc<AtomicBool>,
    pub running: Arc<AtomicBool>,
    pub settings: Arc<RwLock<VisualSettings>>,
    pub snapshot: Arc<RwLock<EspSnapshot>>,
    pub mode_controller: ModeController,
}

impl App {
    pub fn new(
        config: GameConfig,
        memory: Arc<MemoryRouter>,
        clientdll: usize,
        sound_tx: Sender<SoundEvent>,
        mode_controller: ModeController,
    ) -> Self {
        let initial_settings = VisualSettings::default_for_mode(memory.mode());
        println!(
            "[+] App oluşturuldu. Mod: {:?} (Minimal ESP: {})",
            memory.mode(),
            initial_settings.is_minimal
        );

        let sound_initial = config.options.soundESPEnabled;
        let visual_initial = config.options.glowESPEnabled;

        Self {
            config,
            memory,
            clientdll,
            sound_tx,
            sound_esp_enabled: Arc::new(AtomicBool::new(sound_initial)),
            visual_esp_enabled: Arc::new(AtomicBool::new(visual_initial)),
            running: Arc::new(AtomicBool::new(true)),
            settings: Arc::new(RwLock::new(initial_settings)),
            snapshot: Arc::new(RwLock::new(EspSnapshot::default())),
            mode_controller,
        }
    }

    /// Model A döngüsünü başlatır:
    /// - Arka Plan Thread: CS2 bellek okuma + Sound ESP + Anlık durum güncelleme
    /// - Ana Thread: DirectX 11 / ImGui Overlay döngüsü (mesafeye göre küçülen dinamik 2D box + mod duyarlı çizimler)
    pub fn run(self) {
        let memory = Arc::clone(&self.memory);
        let config = self.config.clone();
        let clientdll = self.clientdll;
        let sound_tx = self.sound_tx.clone();
        let sound_esp_enabled = Arc::clone(&self.sound_esp_enabled);
        let snapshot_shared = Arc::clone(&self.snapshot);
        let running_reader = Arc::clone(&self.running);

        // 1. Arka plan bellek okuma thread'ini başlat
        let reader_thread = thread::Builder::new()
            .name(obfstr::obfstr!("worker-mem").into())
            .spawn(move || {
                Self::memory_reader_loop(
                    memory,
                    config,
                    clientdll,
                    sound_tx,
                    sound_esp_enabled,
                    snapshot_shared,
                    running_reader,
                );
            })
            .expect("Bellek okuma thread'i başlatılamadı");

        // 1b. Discord V3 overlay köprüsü (CDP + canvas). Discord yoksa sessizce bekler.
        let _discord_thread = discord_overlay::spawn(
            Arc::clone(&self.snapshot),
            Arc::clone(&self.settings),
            Arc::clone(&self.visual_esp_enabled),
            Arc::clone(&self.sound_esp_enabled),
            Arc::clone(&self.running),
            self.mode_controller.clone(),
        );

        // 2. Ana thread: Overlay başlatma prosedürü
        let pid = self.memory.get_pid();
        let options = OverlayOptions {
            title: obfstr::obfstr!("AppHostOverlay").to_string(),
            target: OverlayTarget::WindowOfProcess(pid),
            register_fonts_callback: None,
        };

        println!("[*] Overlay başlatılıyor (Target PID: {})...", pid);
        let mut input_handler = InputHandler::new(
            Arc::clone(&self.sound_esp_enabled),
            Arc::clone(&self.visual_esp_enabled),
            Arc::clone(&self.settings),
            self.mode_controller.clone(),
            Arc::clone(&self.memory),
        );

        let running = Arc::clone(&self.running);
        let running_input = Arc::clone(&self.running);

        // 1c. Bağımsız kısayol (hotkey) dinleyici thread'i (~100Hz):
        // ImGui penceresi SW_HIDE ile gizli olsa veya WM_PAINT gelmese dahi
        // F9, F11/Insert ve Shift+F1..F5 kısayollarının kesintisiz çalışmasını garanti eder.
        let _input_thread = thread::Builder::new()
            .name(obfstr::obfstr!("worker-inp").into())
            .spawn(move || {
                while running_input.load(Ordering::Relaxed) {
                    input_handler.update();
                    thread::sleep(Duration::from_millis(10));
                }
            })
            .expect("Kısayol dinleyici thread'i başlatılamadı");

        match overlay::init(options) {
            Ok(system) => {
                println!("[+] Overlay penceresi başarıyla oluşturuldu.");
                println!("[*] Kısayollar:");
                println!("    Shift + F1: Sound ESP aç/kapat");
                println!("    Shift + F2: Visual ESP aç/kapat");
                println!("    Shift + F3: İskelet çizimi aç/kapat");
                println!("    Shift + F4: İsim / HP / Silah detayları aç/kapat");
                println!("    Shift + F5: Minimal / Detaylı çizim modu değiştir");
                println!("    F9        : Stealth Mod değiştir (1 -> 2 -> 3 -> 4)");
                println!("    F11 / INS : HUD gösterge panelini gizle/göster");

                let snapshot_ref = Arc::clone(&self.snapshot);
                let settings_ref = Arc::clone(&self.settings);
                let visual_esp_ref = Arc::clone(&self.visual_esp_enabled);
                let sound_esp_ref = Arc::clone(&self.sound_esp_enabled);
                let running_ref = Arc::clone(&self.running);
                let mode_controller_loop = self.mode_controller.clone();
                let mode_controller_render = self.mode_controller.clone();
                let mut last_imgui_visible = mode_controller_loop.is_imgui_active();

                // Başlangıçta mod 3 veya 4 ise ImGui penceresini derhal gizleme bayrağı
                let mut initial_checked = false;

                // Overlay Winit ana döngüsü
                system.main_loop(
                    move |controller| {
                        let should_be_visible = mode_controller_loop.is_imgui_active();
                        if !initial_checked || should_be_visible != last_imgui_visible {
                            controller.set_visible(should_be_visible);
                            last_imgui_visible = should_be_visible;
                            initial_checked = true;
                            if should_be_visible {
                                println!("[+] ImGui Overlay görünür (Mod: {:?})", mode_controller_loop.mode());
                            } else {
                                println!("[+] ImGui Overlay tamamen gizlendi (SW_HIDE) - AC Stealth devrede");
                            }
                        }

                        running_ref.load(Ordering::Relaxed)
                    },
                    move |ui, _unicode_renderer| {
                        // 1. Mod Kontrolü: Mod 3 (Indirect) veya Mod 4 (Rage) modlarında
                        // ImGui KESİNLİKLE hiçbir çizim yapmaz (zero-draw).
                        if !mode_controller_render.is_imgui_active() {
                            return true;
                        }

                        let snapshot = match snapshot_ref.read() {
                            Ok(snap) => snap.clone(),
                            Err(_) => return true,
                        };

                        // 2. In-Game Kontrolü: Maçta değilken çizim yapma
                        if !snapshot.is_in_game {
                            return true;
                        }

                        let settings = match settings_ref.read() {
                            Ok(set) => set.clone(),
                            Err(_) => return true,
                        };

                        // 3. HUD Gösterge Paneli (mod seviyesi + özellik ON/OFF).
                        // Visual ESP kapalı olsa da panel görünür kalır; yalnızca
                        // Mod 1/2'de (ImGui aktif) ve HUD açıkken çizilir.
                        if let Some(hud_model) = hud::build(
                            &mode_controller_render,
                            &settings,
                            visual_esp_ref.load(Ordering::Relaxed),
                            sound_esp_ref.load(Ordering::Relaxed),
                        ) {
                            Self::render_hud(ui, &hud_model);
                        }

                        // 4. ESP çizimi (visual ESP kapalıysa atla).
                        if !visual_esp_ref.load(Ordering::Relaxed) {
                            return true;
                        }

                        Self::render_esp(ui, &snapshot, &settings);
                        true
                    },
                );

                running.store(false, Ordering::SeqCst);
            }
            Err(e) => {
                eprintln!("[!] Overlay başlatılamadı: {:?}.", e);
                eprintln!("[*] Sound ESP konsol modunda çalışmaya devam ediyor.");
                println!("[*] Çıkmak için Ctrl+C tuşlayabilirsiniz.");

                while running.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(50));
                }
            }
        }

        let _ = reader_thread.join();
        println!("[*] cs2ru sonlandırıldı.");
    }

    /// Arka plan bellek okuma döngüsü (~100Hz)
    fn memory_reader_loop(
        memory: Arc<MemoryRouter>,
        config: GameConfig,
        clientdll: usize,
        sound_tx: Sender<SoundEvent>,
        sound_esp_enabled: Arc<AtomicBool>,
        snapshot_shared: Arc<RwLock<EspSnapshot>>,
        running: Arc<AtomicBool>,
    ) {
        println!("[+] Arka plan bellek okuyucu thread'i devrede.");

        let target_interval = Duration::from_millis(10); // ~100Hz
        let initial_kit = memory.kit();
        let mut player_cache: Vec<Player> = (0..64)
            .map(|i| Player::new(i, 0, Arc::clone(&initial_kit), &config.offsets, clientdll))
            .collect();

        while running.load(Ordering::Relaxed) {
            let start = Instant::now();

            // Mod geçişi bu turun ortasında olsa bile okumalar tek bir tutarlı
            // syscall katmanı üzerinden yapılır.
            let kit = memory.kit();

            let entitylist: usize = unsafe { kit.read(clientdll + config.offsets.entityList as usize) };
            if entitylist != 0 {
                let list_entry: usize = unsafe { kit.read(entitylist + 0x10) };
                if list_entry != 0 {
                    for p in player_cache.iter_mut() {
                        p.reset_context(entitylist, list_entry, Arc::clone(&kit));
                    }

                    // 1. ViewMatrix oku
                    let view_matrix_bytes = unsafe {
                        kit.read_buffer(clientdll + config.offsets.viewMatrix as usize, 64)
                    };
                    let view_matrix = ViewMatrix::from_bytes(&view_matrix_bytes);

                    // 2. Local player'ı doğrudan offset üzerinden çöz (64 slot taramadan)
                    let mut local_player_info: Option<(Vec3, i32)> = None;
                    if config.offsets.localPlayerController != 0 {
                        let local_ctrl: usize = unsafe {
                            kit.read(clientdll + config.offsets.localPlayerController as usize)
                        };
                        if local_ctrl != 0 {
                            let pawn_handle: usize = unsafe {
                                kit.read(local_ctrl + config.offsets.controller.m_hPawn)
                            };
                            if pawn_handle != 0 && (pawn_handle & 0x7FFF) != 0x7FFF {
                                let pawn_entry: usize = unsafe {
                                    kit.read(entitylist + 0x10 + 0x8 * ((pawn_handle & 0x7FFF) >> 9))
                                };
                                if pawn_entry != 0 {
                                    let local_pawn: usize = unsafe {
                                        kit.read(pawn_entry + 0x70 * (pawn_handle & 0x1FF))
                                    };
                                    if local_pawn != 0 {
                                        let team: i32 = unsafe {
                                            kit.read(local_pawn + config.offsets.pawn.m_iTeamNum)
                                        };
                                        let pos_buf = unsafe {
                                            kit.read_buffer(
                                                local_pawn + config.offsets.pawn.m_vOldOrigin,
                                                12,
                                            )
                                        };
                                        if pos_buf.len() == 12 {
                                            let px = f32::from_le_bytes(
                                                pos_buf[0..4].try_into().unwrap_or([0; 4]),
                                            );
                                            let py = f32::from_le_bytes(
                                                pos_buf[4..8].try_into().unwrap_or([0; 4]),
                                            );
                                            let pz = f32::from_le_bytes(
                                                pos_buf[8..12].try_into().unwrap_or([0; 4]),
                                            );
                                            local_player_info = Some((Vec3::new(px, py, pz), team));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // 3. Tüm oyuncuları kalıcı önbellek üzerinden güncelle
                    for p in player_cache.iter_mut() {
                        if p.update(false) && p.localplayer && local_player_info.is_none() {
                            local_player_info = Some((p.pos, p.team));
                        }
                    }

                    let forward = view_matrix.forward_vector();

                    let mut player_snapshots = Vec::new();
                    let mut closest_enemies: Vec<(f32, f32, f32)> = Vec::new();

                    let (has_local, local_pos, local_team) = match local_player_info {
                        Some((pos, team)) => (true, pos, team),
                        None => (false, Vec3::default(), 0),
                    };

                    for p in player_cache.iter() {
                        if !p.alive || p.localplayer {
                            continue;
                        }

                        let is_enemy = has_local && local_team != 0 && p.team != local_team;

                        // Sound ESP mesafe ve açı hesaplama
                        if is_enemy {
                            let (l, r, dist) = utils::Utils::calculate_3d_audio(
                                [local_pos.x, local_pos.y, local_pos.z],
                                [p.pos.x, p.pos.y, p.pos.z],
                                forward,
                            );
                            if dist.is_finite() && dist < 2000.0 {
                                closest_enemies.push((dist, l, r));
                            }
                        }

                        // Kafa pozisyonu (6. kemik veya varsayılan yükseklik)
                        let head_pos = if p.bone_list.len() > 6 {
                            p.bone_list[6]
                        } else {
                            p.pos + Vec3::new(0.0, 0.0, 72.0)
                        };

                        player_snapshots.push(PlayerSnapshot {
                            pos: p.pos,
                            head_pos,
                            bone_list: p.bone_list.clone(),
                            health: p.health,
                            armor: p.armor,
                            team: p.team,
                            name: p.name.clone(),
                            weapon: p.clean_weapon.clone(),
                            scoped: p.scoped,
                            defusing: p.defusing,
                            flashed: p.flashed,
                        });
                    }

                    // 4. Sound ESP olayı gönder (en yakın düşman)
                    if sound_esp_enabled.load(Ordering::Relaxed) && !closest_enemies.is_empty() {
                        closest_enemies.sort_by(|a, b| a.0.total_cmp(&b.0));
                        if let Some(&(_, l, r)) = closest_enemies.first() {
                            let _ = sound_tx.send(SoundEvent { left: l, right: r });
                        }
                    }

                    // 5. Kurulu C4 Kontrolü
                    let mut planted_c4: Option<PlantedC4Snapshot> = None;
                    if config.offsets.plantedC4 != 0 {
                        let c4_ptr_addr = clientdll + config.offsets.plantedC4 as usize;
                        let is_planted_flag: bool = unsafe {
                            kit.read(c4_ptr_addr.saturating_sub(config.offsets.bomb.m_isPlanted))
                        };
                        let c4_list_ptr: usize = unsafe { kit.read(c4_ptr_addr) };
                        if c4_list_ptr != 0 {
                            let c4_entity: usize = unsafe { kit.read(c4_list_ptr) };
                            if c4_entity != 0 {
                                let activated: bool = unsafe {
                                    kit.read(c4_entity + config.offsets.bomb.m_bC4Activated)
                                };
                                if activated || is_planted_flag {
                                    let site: i32 = unsafe {
                                        kit.read(c4_entity + config.offsets.bomb.m_nBombSite)
                                    };
                                    let scene_node: usize = unsafe {
                                        kit.read(c4_entity + config.offsets.pawn.m_pGameSceneNode)
                                    };
                                    if scene_node != 0 {
                                        let pos_buf = unsafe {
                                            kit.read_buffer(
                                                scene_node + config.offsets.bomb.m_vecAbsOrigin,
                                                12,
                                            )
                                        };
                                        if pos_buf.len() == 12 {
                                            let bx = f32::from_le_bytes(
                                                pos_buf[0..4].try_into().unwrap_or([0; 4]),
                                            );
                                            let by = f32::from_le_bytes(
                                                pos_buf[4..8].try_into().unwrap_or([0; 4]),
                                            );
                                            let bz = f32::from_le_bytes(
                                                pos_buf[8..12].try_into().unwrap_or([0; 4]),
                                            );
                                            planted_c4 = Some(PlantedC4Snapshot {
                                                pos: Vec3::new(bx, by, bz),
                                                site,
                                                is_planted: true,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // 6. In-Game ve Harita Kontrolü
                    let mut map_name = String::new();
                    let global_vars_ptr: usize = unsafe { kit.read(clientdll + config.offsets.globalVars as usize) };
                    if global_vars_ptr != 0 {
                        let map_name_addr = global_vars_ptr + config.offsets.global_vars.currentMapName;
                        let map_ptr: usize = unsafe { kit.read(map_name_addr) };
                        let target_addr = if map_ptr > 0x10000 && map_ptr < 0x7FFFFFFFFFFF {
                            map_ptr
                        } else {
                            map_name_addr
                        };
                        let buf = unsafe { kit.read_buffer(target_addr, 64) };
                        if let Some(pos) = buf.iter().position(|&b| b == 0) {
                            if let Ok(s) = std::str::from_utf8(&buf[..pos]) {
                                map_name = s.trim().to_string();
                            }
                        }
                    }

                    let map_is_valid = !map_name.is_empty()
                        && !map_name.contains(obfstr::obfstr!("<empty>"))
                        && crate::hash::rt_hash(&map_name) != crate::kh!("unknown");
                    let is_in_game = map_is_valid && has_local && local_team != 0;

                    // 7. Snapshot güncelle
                    if let Ok(mut snap) = snapshot_shared.write() {
                        snap.view_matrix = view_matrix;
                        snap.local_team = local_team;
                        snap.has_local = has_local;
                        snap.is_in_game = is_in_game;
                        snap.map_name = map_name;
                        snap.players = if is_in_game { player_snapshots } else { Vec::new() };
                        snap.planted_c4 = if is_in_game { planted_c4 } else { None };
                    }
                }
            }

            let elapsed = start.elapsed();
            if elapsed < target_interval {
                thread::sleep(target_interval - elapsed);
            }
        }

        println!("[*] Arka plan bellek okuyucu thread'i sonlandı.");
    }

    /// HUD gösterge panelini ImGui DrawList üzerinden çizer (yalnızca Mod 1/2).
    ///
    /// İçerik `crate::hud::build` tarafından üretilir; Discord canvas overlay'i
    /// aynı modeli kullandığı için iki yol arasında bilgi farkı oluşmaz.
    fn render_hud(ui: &imgui::Ui, model: &hud::HudModel) {
        let draw_list = ui.get_background_draw_list();

        let x0 = hud::ORIGIN[0];
        let y0 = hud::ORIGIN[1];
        let pad = hud::PADDING;
        let row_h = hud::ROW_HEIGHT;
        let x1 = x0 + hud::PANEL_WIDTH;
        let y1 = y0 + model.height();
        let text_x = x0 + pad + 4.0;

        // Panel zemini, kenarlık ve sol aksan çizgisi
        draw_list
            .add_rect([x0, y0], [x1, y1], col(11, 13, 17, 210))
            .filled(true)
            .rounding(4.0)
            .build();
        draw_list
            .add_rect([x0, y0], [x1, y1], col(70, 80, 95, 160))
            .rounding(4.0)
            .build();
        draw_list
            .add_rect([x0, y0 + 4.0], [x0 + 3.0, y1 - 4.0], col(65, 160, 255, 255))
            .filled(true)
            .build();

        let mut y = y0 + pad;
        draw_list.add_text([text_x, y], col(65, 160, 255, 255), &model.header);
        y += row_h;

        for row in &model.rows {
            draw_list.add_text([text_x, y], col(150, 158, 170, 255), row.label);
            let value_color = if row.on {
                col(90, 220, 120, 255)
            } else {
                col(230, 90, 90, 255)
            };
            let value_w = ui.calc_text_size(&row.detail)[0];
            draw_list.add_text([x1 - pad - value_w, y], value_color, &row.detail);
            y += row_h;
        }

        y += 3.0;
        draw_list.add_text([text_x, y], col(105, 112, 124, 255), model.footer);
    }

    /// Ortak `crate::esp::build_esp_draw_list` çıktısını ImGui DrawList üzerinden çizer.
    fn render_esp(ui: &imgui::Ui, snapshot: &EspSnapshot, settings: &VisualSettings) {
        let draw_list = ui.get_background_draw_list();
        let [display_w, display_h] = ui.io().display_size;

        let cmds = esp::build_esp_draw_list(snapshot, settings, display_w, display_h);

        for f in &cmds.filled_rects {
            draw_list
                .add_rect([f.x, f.y], [f.x + f.w, f.y + f.h], f.color.to_f32_array())
                .filled(true)
                .build();
        }

        for l in &cmds.lines {
            draw_list
                .add_line([l.x1, l.y1], [l.x2, l.y2], l.color.to_f32_array())
                .thickness(l.thickness)
                .build();
        }

        for c in &cmds.circles {
            draw_list
                .add_circle([c.cx, c.cy], c.radius, c.color.to_f32_array())
                .thickness(c.thickness)
                .build();
        }

        for r in &cmds.rects {
            draw_list
                .add_rect([r.x, r.y], [r.x + r.w, r.y + r.h], r.color.to_f32_array())
                .thickness(r.thickness)
                .build();
        }

        for t in &cmds.texts {
            draw_list.add_text([t.x, t.y], t.color.to_f32_array(), &t.text);
        }
    }
}
