use sysmemkit::SysMemKit;
mod matrix;
mod offset;
mod player;
mod sound;
mod utils;
mod vec3;
use matrix::ViewMatrix;
use offset::GameConfig;
use player::Player;
use rodio::OutputStream;
use rodio::{OutputStreamHandle, Source, source::SineWave};
use sound::StereoSine;
use std::f32::consts::PI;
use std::sync::mpsc::channel;
use std::thread;
use std::time::{Duration};

fn main() {
    let config = GameConfig::load("offsets.toml").expect("Config yüklenemedi!");
    unsafe {
        let kit =
            SysMemKit::new("cs2.exe", false).expect("Oyun bulunamadı veya handle çalınamadı!");
        let clientdll = match kit.getmodule("client.dll") {
            Some(dll) => dll,
            None => {
                println!("client.dll bulunamadı");
                return;
            }
        };
        let engine2dll = match kit.getmodule("engine2.dll") {
            Some(dll) => dll,
            None => {
                println!("engine2.dll bulunamadı");
                return;
            }
        };
        if clientdll != 0 && engine2dll != 0 {
            //let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let (tx, rx) = channel();
            std::thread::spawn(move || StereoSine::audio_thread(rx));
            loop {
                let start_time = std::time::Instant::now();
                let entitylist: usize = kit.read(clientdll + config.offsets.entityList as usize);
                let list_entry: usize = kit.read(entitylist as usize + 0x10);
                let local_player = find_local_player(&kit, &config, list_entry, clientdll);
                let view_matrix_bytes =
                    kit.read_buffer(clientdll + config.offsets.viewMatrix as usize, 64);
                let view_matrix = ViewMatrix::from_bytes(&view_matrix_bytes);
                if let Some(local) = local_player {
                    //println!("local player bulundu {:?}", local.name);
                    let forward = [
                        view_matrix.matrix[0][2],
                        view_matrix.matrix[1][2],
                        view_matrix.matrix[2][2],
                    ];
                    let mut closest = vec![];
                    for i in 0..64 {
                        let mut player =
                            Player::new(i, list_entry, &kit, &config.offsets, clientdll);
                        //println!("player bulundu {:?}", player.name);

                        if player.update(false) && player.alive && !player.localplayer {
                            if player.team == local.team {
                                continue;
                            }
                            let (l, r, dist) = utils::Utils::calculate_3d_audio(
                                [local.pos.x, local.pos.y, local.pos.z],
                                [player.pos.x, player.pos.y, player.pos.z],
                                forward,
                            );
                            if dist < 2000.0 {
                                closest.push((dist, l, r));
                            }
                            // if (player.pos.x - local.pos.x).abs() > 3000.0 {
                            //     continue;
                            // }
                            // let delta_x = player.pos.x - local.pos.x;
                            // let delta_y = player.pos.y - local.pos.y;
                            // let delta_z = player.pos.z - local.pos.z;
                            // let distance =
                            //     (delta_x * delta_x + delta_y * delta_y + delta_z * delta_z).sqrt();
                            // let angle = delta_y.atan2(delta_x);
                            // if distance < 2000.0 {
                            //     let volume = (1.0 - (distance / 2000.0)).max(0.1).min(1.0);
                            //     // let pan = angle.sin();
                            //     let frequency = 200.0 + (800.0 * volume);
                            //     let handle = stream_handle.clone();

                            //     let (pan, volume) = calculate_3d_pan(angle);
                            //     thread::spawn(move || {
                            //         play_sound(&handle, frequency, volume, pan);
                            //     });

                            //     // println!(
                            //     //     "[SOUND ESP] {} - Distance: {:.0} - Angle: {:.0}°",
                            //     //     player.name,
                            //     //     distance,
                            //     //     angle.to_degrees()
                            //     // );
                            // }
                        }
                    }
                    closest.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                    for (_, l, r) in closest.iter().take(3) {
                        let _ = tx.send(sound::SoundEvent {
                            left: *l,
                            right: *r,
                        });
                    }
                } else {
                    println!("local player bulunamadı");
                }
                let elapsed = start_time.elapsed();
                if elapsed < Duration::from_millis(16) {
                    thread::sleep(Duration::from_millis(16) - elapsed);
                }
            }
        }
    }
}

fn find_local_player<'a>(
    kit: &'a SysMemKit,
    config: &'a GameConfig,
    list_entry: usize,
    clientbase: usize,
) -> Option<Player<'a>> {
    for i in 0..64 {
        let mut player = Player::new(i, list_entry, kit, &config.offsets, clientbase);
        if player.update(true) && player.localplayer {
            return Some(player);
        }
    }
    None
}

// fn calculate_3d_pan(angle_degrees: f32) -> (f32, f32) {
//     let angle_rad = angle_degrees.to_radians();
//     let angle = angle_rad % (2.0 * PI);
//     let left_right = angle.cos();
//     let pan = left_right;
//     let front_back = angle.sin().abs();
//     let volume_factor = (0.7 + (front_back * 0.3)) * 0.5;

//     (pan, volume_factor)
// }

// fn play_sound(handle: &OutputStreamHandle, frequency: f32, volume: f32, pan: f32) {
//     let source = SineWave::new(frequency)
//         .take_duration(Duration::from_millis(150))
//         .amplify(volume);
//     let _ = handle.play_raw(source);
// }
pub const fn dbj2_hash(s: &str) -> u32 {
    let mut hash: u32 = 5381;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        hash = (hash << 5).wrapping_add(hash).wrapping_add(bytes[i] as u32);
        i += 1;
    }
    hash
}
