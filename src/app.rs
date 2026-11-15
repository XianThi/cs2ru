use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_F1, VK_F2, VK_F3, VK_F4, VK_F5, VK_SHIFT,
};

use overlay::{OverlayOptions, OverlayTarget};
use sysmemkit::{Mode, SysMemKit};

use crate::matrix::ViewMatrix;
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
    pub team: i32,
    pub name: String,
    pub weapon: String,
}

/// Çift thread arasında paylaşılan dünya ve oyuncu snapshot'ı
#[derive(Debug, Clone, Default)]
pub struct EspSnapshot {
    pub view_matrix: ViewMatrix,
    pub local_team: i32,
    pub has_local: bool,
    pub players: Vec<PlayerSnapshot>,
}

/// Kısayol tuşlarını dinleyen ve durumları güncelleyen işleyici
pub struct InputHandler {
    sound_esp_enabled: Arc<AtomicBool>,
    visual_esp_enabled: Arc<AtomicBool>,
    settings: Arc<RwLock<VisualSettings>>,
    f1_was_down: bool,
    f2_was_down: bool,
    f3_was_down: bool,
    f4_was_down: bool,
    f5_was_down: bool,
}

impl InputHandler {
    pub fn new(
        sound_esp_enabled: Arc<AtomicBool>,
        visual_esp_enabled: Arc<AtomicBool>,
        settings: Arc<RwLock<VisualSettings>>,
    ) -> Self {
        Self {
            sound_esp_enabled,
            visual_esp_enabled,
            settings,
            f1_was_down: false,
            f2_was_down: false,
            f3_was_down: false,
            f4_was_down: false,
            f5_was_down: false,
        }
    }

    #[inline]
    fn is_key_down(v_key: u16) -> bool {
        unsafe { (GetAsyncKeyState(v_key as i32) as u16 & 0x8000) != 0 }
    }

    pub fn update(&mut self) {
        let shift = Self::is_key_down(VK_SHIFT);
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
    }
}

/// Model A Çift-Thread Uygulama Yöneticisi
pub struct App {
    pub config: GameConfig,
    pub kit: Arc<SysMemKit>,
    pub clientdll: usize,
    pub sound_tx: Sender<SoundEvent>,
    pub sound_esp_enabled: Arc<AtomicBool>,
    pub visual_esp_enabled: Arc<AtomicBool>,
    pub running: Arc<AtomicBool>,
    pub settings: Arc<RwLock<VisualSettings>>,
    pub snapshot: Arc<RwLock<EspSnapshot>>,
}

impl App {
    pub fn new(
        config: GameConfig,
        kit: Arc<SysMemKit>,
        clientdll: usize,
        sound_tx: Sender<SoundEvent>,
    ) -> Self {
        let initial_settings = VisualSettings::default_for_mode(kit.mode());
        println!(
            "[+] App oluşturuldu. Mod: {:?} (Minimal ESP: {})",
            kit.mode(),
            initial_settings.is_minimal
        );

        Self {
            config,
            kit,
            clientdll,
            sound_tx,
            sound_esp_enabled: Arc::new(AtomicBool::new(false)),
            visual_esp_enabled: Arc::new(AtomicBool::new(true)),
            running: Arc::new(AtomicBool::new(true)),
            settings: Arc::new(RwLock::new(initial_settings)),
            snapshot: Arc::new(RwLock::new(EspSnapshot::default())),
        }
    }

    /// Model A döngüsünü başlatır:
    /// - Arka Plan Thread: CS2 bellek okuma + Sound ESP + Anlık durum güncelleme
    /// - Ana Thread: DirectX 11 / ImGui Overlay döngüsü (mesafeye göre küçülen dinamik 2D box + mod duyarlı çizimler)
    pub fn run(self) {
        let kit = Arc::clone(&self.kit);
        let config = self.config.clone();
        let clientdll = self.clientdll;
        let sound_tx = self.sound_tx.clone();
        let sound_esp_enabled = Arc::clone(&self.sound_esp_enabled);
        let snapshot_shared = Arc::clone(&self.snapshot);
        let running_reader = Arc::clone(&self.running);

        // 1. Arka plan bellek okuma thread'ini başlat
        let reader_thread = thread::Builder::new()
            .name("cs2ru-memory-reader".into())
            .spawn(move || {
                Self::memory_reader_loop(
                    kit,
                    config,
                    clientdll,
                    sound_tx,
                    sound_esp_enabled,
                    snapshot_shared,
                    running_reader,
                );
            })
            .expect("Bellek okuma thread'i başlatılamadı");

        // 2. Ana thread: Overlay başlatma prosedürü
        let pid = self.kit.get_pid();
        let options = OverlayOptions {
            title: "CS2 Overlay".to_string(),
            target: OverlayTarget::WindowOfProcess(pid),
            register_fonts_callback: None,
        };

        println!("[*] Overlay başlatılıyor (Target PID: {})...", pid);
        let mut input_handler = InputHandler::new(
            Arc::clone(&self.sound_esp_enabled),
            Arc::clone(&self.visual_esp_enabled),
            Arc::clone(&self.settings),
        );

        let running = Arc::clone(&self.running);

        match overlay::init(options) {
            Ok(system) => {
                println!("[+] Overlay penceresi başarıyla oluşturuldu.");
                println!("[*] Kısayollar:");
                println!("    Shift + F1: Sound ESP aç/kapat");
                println!("    Shift + F2: Visual ESP aç/kapat");
                println!("    Shift + F3: İskelet çizimi aç/kapat");
                println!("    Shift + F4: İsim / HP / Silah detayları aç/kapat");
                println!("    Shift + F5: Minimal / Detaylı çizim modu değiştir");

                let snapshot_ref = Arc::clone(&self.snapshot);
                let settings_ref = Arc::clone(&self.settings);
                let visual_esp_ref = Arc::clone(&self.visual_esp_enabled);
                let running_ref = Arc::clone(&self.running);

                // Overlay Winit ana döngüsü
                system.main_loop(
                    move |_controller| {
                        input_handler.update();
                        running_ref.load(Ordering::Relaxed)
                    },
                    move |ui, _unicode_renderer| {
                        if !visual_esp_ref.load(Ordering::Relaxed) {
                            return true;
                        }

                        let snapshot = match snapshot_ref.read() {
                            Ok(snap) => snap.clone(),
                            Err(_) => return true,
                        };

                        let settings = match settings_ref.read() {
                            Ok(set) => set.clone(),
                            Err(_) => return true,
                        };

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
                    input_handler.update();
                    thread::sleep(Duration::from_millis(50));
                }
            }
        }

        let _ = reader_thread.join();
        println!("[*] cs2ru sonlandırıldı.");
    }

    /// Arka plan bellek okuma döngüsü (~100Hz)
    fn memory_reader_loop(
        kit: Arc<SysMemKit>,
        config: GameConfig,
        clientdll: usize,
        sound_tx: Sender<SoundEvent>,
        sound_esp_enabled: Arc<AtomicBool>,
        snapshot_shared: Arc<RwLock<EspSnapshot>>,
        running: Arc<AtomicBool>,
    ) {
        println!("[+] Arka plan bellek okuyucu thread'i devrede.");

        let target_interval = Duration::from_millis(10); // ~100Hz

        while running.load(Ordering::Relaxed) {
            let start = Instant::now();

            let entitylist: usize = unsafe { kit.read(clientdll + config.offsets.entityList as usize) };
            if entitylist != 0 {
                let list_entry: usize = unsafe { kit.read(entitylist + 0x10) };
                if list_entry != 0 {
                    // 1. ViewMatrix oku
                    let view_matrix_bytes = unsafe {
                        kit.read_buffer(clientdll + config.offsets.viewMatrix as usize, 64)
                    };
                    let view_matrix = ViewMatrix::from_bytes(&view_matrix_bytes);

                    // 2. Local player bul
                    let mut local_player_info: Option<(Vec3, i32)> = None;
                    for i in 0..64 {
                        let mut p = Player::new(i, list_entry, &kit, &config.offsets, clientdll);
                        if p.update(true) && p.localplayer {
                            local_player_info = Some((p.pos, p.team));
                            break;
                        }
                    }

                    let forward = [
                        view_matrix.matrix[0][2],
                        view_matrix.matrix[1][2],
                        view_matrix.matrix[2][2],
                    ];

                    let mut player_snapshots = Vec::new();
                    let mut closest_enemies: Vec<(f32, f32, f32)> = Vec::new();

                    let (has_local, local_pos, local_team) = match local_player_info {
                        Some((pos, team)) => (true, pos, team),
                        None => (false, Vec3::default(), 0),
                    };

                    // 3. Diğer oyuncuları tara
                    for i in 0..64 {
                        let mut p = Player::new(i, list_entry, &kit, &config.offsets, clientdll);
                        if !p.update(false) || !p.alive || p.localplayer {
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
                            if dist < 2000.0 {
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
                            team: p.team,
                            name: p.name.clone(),
                            weapon: p.clean_weapon.clone(),
                        });
                    }

                    // 4. Sound ESP olayı gönder
                    if sound_esp_enabled.load(Ordering::Relaxed) && !closest_enemies.is_empty() {
                        closest_enemies.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                        for (_, l, r) in closest_enemies.iter().take(3) {
                            let _ = sound_tx.send(SoundEvent {
                                left: *l,
                                right: *r,
                            });
                        }
                    }

                    // 5. Snapshot güncelle
                    if let Ok(mut snap) = snapshot_shared.write() {
                        snap.view_matrix = view_matrix;
                        snap.local_team = local_team;
                        snap.has_local = has_local;
                        snap.players = player_snapshots;
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

    /// ImGui DrawList üzerinden ESP çizimlerini gerçekleştirir.
    /// - Mesafeye göre dinamik küçülen 2D bounding box
    /// - Mode / minimal ayarına göre şekillenen çizimler (Rage: minimal, Legit: detaylı)
    fn render_esp(ui: &imgui::Ui, snapshot: &EspSnapshot, settings: &VisualSettings) {
        let draw_list = ui.get_background_draw_list();
        let [display_w, display_h] = ui.io().display_size;

        if display_w <= 0.0 || display_h <= 0.0 {
            return;
        }

        let is_minimal = settings.is_minimal;

        for player in &snapshot.players {
            if player.health <= 0 {
                continue;
            }

            let is_enemy = snapshot.local_team != 0 && player.team != snapshot.local_team;
            let base_color = if is_enemy {
                col(255, 65, 65, 255) // Düşman: Kırmızı
            } else {
                col(65, 160, 255, 255) // Takım arkadaşı: Mavi
            };

            // Kafa üstü sınır (kafa kemiğinin 8 birim yukarısı)
            let head_top = player.head_pos + Vec3::new(0.0, 0.0, 8.0);

            // 1. Ekran izdüşüm koordinatlarını hesapla (World-to-Screen)
            let feet_proj =
                snapshot.view_matrix.world_to_screen(&player.pos, display_w, display_h);
            let head_proj =
                snapshot.view_matrix.world_to_screen(&head_top, display_w, display_h);

            let (fx, fy) = match feet_proj {
                Some(p) => p,
                None => continue,
            };
            let (_hx, hy) = match head_proj {
                Some(p) => p,
                None => continue,
            };

            // 2. Dinamik 2D Bounding Box:
            // Karakter uzaklaştıkça W2S perspektif bölmesi büyür,
            // dolayısıyla ekrandaki dikey piksel boyutu (fy - hy).abs() doğal olarak küçülür.
            let box_height = (fy - hy).abs();
            if box_height < 4.0 || box_height > display_h * 2.5 {
                continue;
            }

            let box_width = box_height * 0.45;
            let center_x = fx;
            let min_x = center_x - box_width / 2.0;
            let max_x = center_x + box_width / 2.0;
            let min_y = hy.min(fy);
            let max_y = hy.max(fy);

            // 3. Mod Duyarlı Çizim Mantığı
            if is_minimal {
                // RAGE / MINIMAL MOD: Sadece sade 2D kutu çizilir (dikkat dağıtıcı öğeler yok)
                if settings.show_box {
                    draw_list
                        .add_rect([min_x, min_y], [max_x, max_y], base_color)
                        .thickness(1.0)
                        .build();
                }
            } else {
                // LEGIT / DETAYLI MOD: Kutu + HP Barı + İsim + Silah + İskelet

                // A) Sınırlayıcı Kutu (Kontrastlı dış ve iç siyah kenarlık)
                if settings.show_box {
                    draw_list
                        .add_rect(
                            [min_x - 1.0, min_y - 1.0],
                            [max_x + 1.0, max_y + 1.0],
                            col(0, 0, 0, 255),
                        )
                        .thickness(1.0)
                        .build();
                    draw_list
                        .add_rect([min_x, min_y], [max_x, max_y], base_color)
                        .thickness(1.0)
                        .build();
                    draw_list
                        .add_rect(
                            [min_x + 1.0, min_y + 1.0],
                            [max_x - 1.0, max_y - 1.0],
                            col(0, 0, 0, 255),
                        )
                        .thickness(1.0)
                        .build();
                }

                // B) Can Barı (Sol kenarda dikey çubuk)
                if settings.show_health {
                    let bar_w = (box_width * 0.08).clamp(2.0, 4.0);
                    let bar_x = min_x - 4.0 - bar_w;
                    let bar_h = box_height;

                    // Arka plan (Siyah gölge)
                    draw_list
                        .add_rect(
                            [bar_x - 1.0, min_y - 1.0],
                            [bar_x + bar_w + 1.0, max_y + 1.0],
                            col(0, 0, 0, 220),
                        )
                        .filled(true)
                        .build();

                    // Sağlık yüzdesi ve rengi
                    let hp_pct = (player.health.clamp(0, 100) as f32) / 100.0;
                    let hp_h = bar_h * hp_pct;
                    let hp_top_y = max_y - hp_h;

                    let hp_color = if player.health > 60 {
                        col(50, 220, 50, 255) // Yeşil
                    } else if player.health > 25 {
                        col(240, 180, 20, 255) // Turuncu/Sarı
                    } else {
                        col(240, 40, 40, 255) // Kırmızı
                    };

                    draw_list
                        .add_rect([bar_x, hp_top_y], [bar_x + bar_w, max_y], hp_color)
                        .filled(true)
                        .build();

                    // HP 100'den az ise sayısal değeri yaz
                    if player.health < 100 && player.health > 0 {
                        let hp_text = format!("{}", player.health);
                        draw_list.add_text(
                            [bar_x - 16.0, hp_top_y - 2.0],
                            col(255, 255, 255, 255),
                            &hp_text,
                        );
                    }
                }

                // C) Oyuncu İsmi (Kutunun üstünde)
                if settings.show_name && !player.name.is_empty() {
                    draw_list.add_text(
                        [min_x, min_y - 14.0],
                        col(255, 255, 255, 255),
                        &player.name,
                    );
                }

                // D) Silah İsmi (Kutunun altında)
                if settings.show_weapon && !player.weapon.is_empty() {
                    draw_list.add_text(
                        [min_x, max_y + 2.0],
                        col(210, 210, 210, 255),
                        &player.weapon,
                    );
                }

                // E) İskelet / Kemik Hatları (Skeleton)
                if settings.show_skeleton && player.bone_list.len() >= 28 {
                    let skeleton_color = col(255, 255, 255, 200);
                    let bone_pairs: &[(usize, usize)] = &[
                        // Baş - Boyun - Omurga - Pelvis
                        (6, 5),
                        (5, 4),
                        (4, 2),
                        (2, 0),
                        // Sol Kol
                        (5, 8),
                        (8, 9),
                        (9, 10),
                        // Sağ Kol
                        (5, 13),
                        (13, 14),
                        (14, 15),
                        // Sol Bacak
                        (0, 22),
                        (22, 23),
                        (23, 24),
                        // Sağ Bacak
                        (0, 25),
                        (25, 26),
                        (26, 27),
                    ];

                    for &(b1, b2) in bone_pairs {
                        if b1 < player.bone_list.len() && b2 < player.bone_list.len() {
                            let p1 = snapshot.view_matrix.world_to_screen(
                                &player.bone_list[b1],
                                display_w,
                                display_h,
                            );
                            let p2 = snapshot.view_matrix.world_to_screen(
                                &player.bone_list[b2],
                                display_w,
                                display_h,
                            );

                            if let (Some(pt1), Some(pt2)) = (p1, p2) {
                                draw_list
                                    .add_line([pt1.0, pt1.1], [pt2.0, pt2.1], skeleton_color)
                                    .thickness(1.2)
                                    .build();
                            }
                        }
                    }

                    // Kafa dairesi
                    if let Some(head_pt) = snapshot.view_matrix.world_to_screen(
                        &player.bone_list[6],
                        display_w,
                        display_h,
                    ) {
                        let radius = (box_height * 0.07).clamp(2.5, 7.0);
                        draw_list
                            .add_circle([head_pt.0, head_pt.1], radius, skeleton_color)
                            .thickness(1.2)
                            .build();
                    }
                }
            }
        }
    }
}
