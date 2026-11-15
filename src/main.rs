mod app;
mod discord_overlay;
mod matrix;
mod offset;
mod player;
mod sound;
mod utils;
mod vec3;

use crate::app::App;
use crate::offset::GameConfig;
use crate::player::Player;
use std::io::{self, Write};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread::{self, sleep};
use std::time::Duration;
use sysmemkit::{Mode, SysMemKit};

fn select_mode() -> Mode {
    println!("==================================================");
    println!("             SYSMEMKIT ÇALIŞMA MODU               ");
    println!("==================================================");
    println!(" [1] Basic         - Seviye 1: Export Tablosu SSN + Direct Syscall");
    println!(" [2] DirectStealth - Seviye 2: Hook-Aware Resolver + Direct Syscall");
    println!(" [3] Indirect      - Seviye 3: Indirect Syscall Trampoline (Varsayılan)");
    println!(" [4] Rage          - Seviye 4: Stack Unwind Frame / ROP Chain");
    println!("==================================================");
    print!("Çalışma modu seçin [1-4] (Varsayılan: 3): ");
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        match input.trim() {
            "1" => {
                println!("[+] Seçilen mod: Basic");
                Mode::Basic
            }
            "2" => {
                println!("[+] Seçilen mod: DirectStealth");
                Mode::DirectStealth
            }
            "3" => {
                println!("[+] Seçilen mod: Indirect");
                Mode::Indirect
            }
            "4" => {
                println!("[+] Seçilen mod: Rage");
                Mode::Rage
            }
            _ => {
                println!("[*] Varsayılan mod seçildi: Indirect");
                Mode::Indirect
            }
        }
    } else {
        println!("[*] Girdi okunamadı, varsayılan mod seçildi: Indirect");
        Mode::Indirect
    }
}

fn main() {
    if std::env::args().any(|a| a == "--discord-selftest") {
        discord_overlay::selftest();
        return;
    }

    let mode = select_mode();

    println!("[*] Oyun tespiti için bekleniyor...");
    sleep(Duration::from_secs(2)); // Oyun ve hedef sürecin hazır olması için bekle
    let mut config = GameConfig::load("offsets.toml").unwrap_or_else(|_| {
        println!("[!] offsets.toml bulunamadı, varsayılan yapılandırma oluşturuluyor...");
        GameConfig::default()
    });

    println!("[*] Güncel offsetler GitHub üzerinden çekiliyor (a2x/cs2-dumper)...");
    match config.update_from_web() {
        Ok(_) => {
            println!("[+] Offsetler internetten başarıyla güncellendi!");
            if let Err(e) = config.save("offsets.toml") {
                eprintln!("[!] Güncellenen offsetler offsets.toml dosyasına kaydedilemedi: {}", e);
            } else {
                println!("[+] Güncel offsetler offsets.toml dosyasına kaydedildi.");
            }
        }
        Err(e) => {
            eprintln!("[!] İnternetten offset çekilemedi: {}. Mevcut yerel offsetler kullanılacak.", e);
        }
    }

    unsafe {
        let kit: SysMemKit = SysMemKit::with_mode("cs2.exe", false, mode)
            .expect("Oyun bulunamadı veya handle çalınamadı!");
        println!(
            "[+] SysMemKit başarıyla bağlandı! PID: {}, Mod: {:?}",
            kit.get_pid(),
            kit.mode()
        );

        let clientdll = match kit.getmodule("client.dll") {
            Some(dll) => {
                println!("[+] client.dll bulundu: 0x{:016X}", dll);
                dll
            }
            None => {
                println!("[!] client.dll bulunamadı! Yüklü modüller listeleniyor:");
                for (name, base) in kit.list_modules() {
                    println!("  - 0x{:016X}: {}", base, name);
                }
                return;
            }
        };

        let _engine2dll = match kit.getmodule("engine2.dll") {
            Some(dll) => {
                println!("[+] engine2.dll bulundu: 0x{:016X}", dll);
                dll
            }
            None => {
                println!("[!] engine2.dll bulunamadı! Yüklü modüller listeleniyor:");
                for (name, base) in kit.list_modules() {
                    println!("  - 0x{:016X}: {}", base, name);
                }
                return;
            }
        };

        let (tx, rx) = channel();
        thread::spawn(move || sound::StereoSine::audio_thread(rx));

        let app = App::new(config, Arc::new(kit), clientdll, tx);
        app.run();
    }
}

#[allow(dead_code)]
fn fnv1a_hash(data: u32) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xCBF29CE484222325;
    const FNV_PRIME: u64 = 0x100000001B3;

    let bytes = data.to_le_bytes();
    let mut hash = FNV_OFFSET_BASIS;
    for b in bytes {
        hash = hash ^ (b as u64);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[allow(dead_code)]
fn analyze_shared_memory(kit: &SysMemKit, magic_addr: usize) {
    unsafe {
        let magic = kit.read::<u32>(magic_addr);
        println!("Magic: 0x{:X}", magic);

        let frame_count = kit.read::<u32>(magic_addr + 4);
        println!("Frame count: {}", frame_count);

        let width = kit.read::<u32>(magic_addr + 12);
        println!("Width: {}", width);

        let height = kit.read::<u32>(magic_addr + 16);
        println!("Height: {}", height);

        let framebuffer_ptr = kit.read::<usize>(magic_addr + 20);
        println!("Framebuffer pointer: 0x{:X}", framebuffer_ptr);

        if framebuffer_ptr != 0 {
            let first_pixel = kit.read::<u32>(framebuffer_ptr);
            println!("First pixel (BGRA): 0x{:08X}", first_pixel);
        }

        let overlay_info_addr = magic_addr - 32;
        println!("\nTrying to find OverlayInfo at 0x{:X}", overlay_info_addr);

        let message_window = kit.read::<usize>(overlay_info_addr);
        let overlay_window = kit.read::<usize>(overlay_info_addr + 8);
        println!("Message window handle: 0x{:X}", message_window);
        println!("Overlay window handle: 0x{:X}", overlay_window);

        for offset in -0x1000..0x1000 {
            let addr = (magic_addr as isize + offset) as usize;
            if addr < magic_addr - 0x2000 {
                continue;
            }

            let hash = kit.read::<u64>(addr);
            let ptr = kit.read::<usize>(addr + 8);

            if hash > 0x1000000 && ptr > 0x10000 && ptr < 0x7FFFFFFFFFFF {
                println!(
                    "Possible hash entry at 0x{:X}: hash=0x{:X}, ptr=0x{:X}",
                    addr, hash, ptr
                );
            }
        }
    }
}

#[allow(dead_code)]
pub fn find_local_player<'a>(
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
