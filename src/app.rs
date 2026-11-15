use crate::matrix::ViewMatrix;
use crate::offset::GameConfig;
use crate::player::Player;
use crate::sound::SoundEvent;
use crate::utils::Utils;
use std::sync::mpsc::Sender;
use sysmemkit::SysMemKit;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_F1, VK_F2, VK_SHIFT};

pub struct App {
    pub is_running: bool,
    pub sound_esp_enabled: bool,
    pub visual_esp_enabled: bool,
    f1_was_down: bool,
    f2_was_down: bool,
    //pub overlay: Overlay,
}

impl App {
    pub fn new() -> Self {
        Self {
            is_running: true,
            sound_esp_enabled: false,
            visual_esp_enabled: false,
            f1_was_down: false,
            f2_was_down: false,
            // overlay: Overlay::new(1920, 1068).unwrap_or_else(|| {
            //     println!("Overlay oluşturulamadı!");
            //     std::process::exit(1);
            // }),
        }
    }
    fn is_key_down(v_key: i32) -> bool {
        unsafe { (GetAsyncKeyState(v_key) as u16 & 0x8000) != 0 }
    }
    pub fn update_input(&mut self) {
        let shift = Self::is_key_down(VK_SHIFT as i32);
        let f1 = Self::is_key_down(VK_F1 as i32);
        let f2 = Self::is_key_down(VK_F2 as i32);
        // Shift basılıyken F1'e "yeni" basıldıysa (Toggle)
        if shift && f1 {
            if !self.f1_was_down {
                self.sound_esp_enabled = !self.sound_esp_enabled;
                println!(
                    ">>> Sound ESP: {}",
                    if self.sound_esp_enabled {
                        "AKTIF"
                    } else {
                        "KAPALI"
                    }
                );
            }
            self.f1_was_down = true;
        } else {
            self.f1_was_down = false;
        }
        if shift && f2 {
            if !self.f2_was_down {
                self.visual_esp_enabled = !self.visual_esp_enabled;
                if self.visual_esp_enabled {
                    //self.overlay.create_window();
                    println!("Visual ESP: AKTIF");
                } else {
                    //self.overlay.destroy_window();
                    println!("Visual ESP: KAPALI");
                }
            }
            self.f2_was_down = true;
        } else {
            self.f2_was_down = false;
        }
    }
    
    pub fn run_sound_esp(
        &self,
        kit: &SysMemKit,
        config: &GameConfig,
        clientdll: usize,
        tx: &Sender<SoundEvent>,
    ) {
        if !self.sound_esp_enabled {
            return;
        }

        unsafe {
            let entitylist: usize = kit.read(clientdll + config.offsets.entityList as usize);
            let list_entry: usize = kit.read(entitylist + 0x10);
            let local_player = crate::find_local_player(kit, config, list_entry, clientdll);
            //println!("Local player: {}", local_player.is_some());
            let view_matrix_bytes =
                kit.read_buffer(clientdll + config.offsets.viewMatrix as usize, 64);
            let view_matrix = ViewMatrix::from_bytes(&view_matrix_bytes);

            if let Some(local) = local_player {
                let forward = [
                    view_matrix.matrix[0][2],
                    view_matrix.matrix[1][2],
                    view_matrix.matrix[2][2],
                ];
                let mut closest = vec![];

                for i in 0..64 {
                    let mut player = Player::new(i, list_entry, kit, &config.offsets, clientdll);
                    if player.update(false) && player.alive && !player.localplayer {
                        if player.team == local.team {
                            continue;
                        }
                        let (l, r, dist) = Utils::calculate_3d_audio(
                            [local.pos.x, local.pos.y, local.pos.z],
                            [player.pos.x, player.pos.y, player.pos.z],
                            forward,
                        );

                        if dist < 2000.0 {
                            closest.push((dist, l, r));
                        }
                    }
                }

                closest.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                for (_, l, r) in closest.iter().take(3) {
                    let _ = tx.send(SoundEvent {
                        left: *l,
                        right: *r,
                    });
                }
            }
        }
    }
}
