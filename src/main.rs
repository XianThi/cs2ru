#[macro_use]
pub mod hash;
mod app;
mod discord_overlay;
pub mod esp;
mod hud;
mod matrix;
mod memory;
pub mod mode;
mod offset;
mod player;
mod sound;
mod utils;
mod vec3;

use crate::app::App;
use crate::memory::MemoryRouter;
use crate::mode::{ModeController, StealthMode};
use crate::offset::GameConfig;
use obfstr::obfstr;
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
    if std::env::args().any(|a| crate::hash::rt_hash(&a) == kh!("--discord-selftest")) {
        discord_overlay::selftest();
        return;
    }

    let mode = select_mode();

    println!("[*] Oyun tespiti için bekleniyor...");
    sleep(Duration::from_secs(2)); // Oyun ve hedef sürecin hazır olması için bekle
    let cfg_path = obfstr!("offsets.toml").to_string();
    let mut config = GameConfig::load(&cfg_path).unwrap_or_else(|_| {
        println!("[!] Yapılandırma dosyası bulunamadı, varsayılan yapılandırma oluşturuluyor...");
        GameConfig::default()
    });

    println!("[*] Güncel offsetler çekiliyor...");
    let web_updated = match config.update_from_web() {
        Ok(_) => {
            println!("[+] Offsetler internetten başarıyla güncellendi!");
            if let Err(e) = config.save(&cfg_path) {
                eprintln!("[!] Güncellenen offsetler kaydedilemedi: {}", e);
            } else {
                println!("[+] Güncel offsetler kaydedildi.");
            }
            true
        }
        Err(e) => {
            eprintln!("[!] İnternetten offset çekilemedi: {}. Signature taraması / yerel offsetler kullanılacak.", e);
            false
        }
    };

    unsafe {
        // Hedef süreç ve modül isimleri derleme zamanında DJB2 hash'e dönüştürülür;
        // "cs2.exe", "client.dll", "engine2.dll" ikili dosyada (.rdata) yer almaz.
        const TARGET_PROC_HASH: u32 = djb2!("cs2.exe");
        const CLIENT_MOD_HASH: u32 = djb2!("client.dll");
        const ENGINE_MOD_HASH: u32 = djb2!("engine2.dll");

        let kit: SysMemKit = match SysMemKit::with_mode_hash(TARGET_PROC_HASH, mode) {
            Some(kit) => kit,
            None if mode == Mode::Rage => {
                eprintln!("[!] Rage (ROP) katmanı kurulamadı; Indirect katmanına düşülüyor.");
                SysMemKit::with_mode_hash(TARGET_PROC_HASH, Mode::Indirect).unwrap_or_else(|| {
                    panic!("Oyun bulunamadı, handle çalınamadı veya syscall katmanı kurulamadı!")
                })
            }
            None => panic!("Oyun bulunamadı, handle çalınamadı veya syscall katmanı kurulamadı!"),
        };

        println!(
            "[+] SysMemKit başarıyla bağlandı! PID: {}, Mod: {:?}",
            kit.get_pid(),
            kit.mode()
        );

        let clientdll = match kit.getmodule_hash(CLIENT_MOD_HASH) {
            Some(dll) => {
                println!("[+] Modül #1 bulundu: 0x{:016X}", dll);
                dll
            }
            None => {
                println!("[!] Modül #1 bulunamadı! Yüklü modüller listeleniyor:");
                for (name, base) in kit.list_modules() {
                    println!("  - 0x{:016X}: {}", base, name);
                }
                return;
            }
        };

        let engine2dll = match kit.getmodule_hash(ENGINE_MOD_HASH) {
            Some(dll) => {
                println!("[+] Modül #2 bulundu: 0x{:016X}", dll);
                dll
            }
            None => {
                println!("[!] Modül #2 bulunamadı! Yüklü modüller listeleniyor:");
                for (name, base) in kit.list_modules() {
                    println!("  - 0x{:016X}: {}", base, name);
                }
                return;
            }
        };

        if !web_updated {
            let count = config.update_from_signatures(&kit, clientdll, engine2dll);
            if count > 0 {
                println!("[+] Signature taraması ile {} ana tablo offseti bellekten güncellendi.", count);
                let _ = config.save(&cfg_path);
            }
        }

        let (tx, rx) = channel();
        thread::spawn(move || sound::StereoSine::audio_thread(rx));

        let stealth_mode = StealthMode::from_sys_mode(kit.mode());
        let mode_controller = ModeController::new(stealth_mode);

        let app = App::new(
            config,
            Arc::new(MemoryRouter::new(kit)),
            clientdll,
            tx,
            mode_controller,
        );
        app.run();
    }
}
