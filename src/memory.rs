//! Syscall Katmanı Yönlendiricisi (Faz 4)
//!
//! `SysMemKit` moduna göre farklı `SsnResolver` + `SyscallInvoker` bileşenleri barındırır.
//! Bu yönlendirici, mod değiştiğinde kit'i **yeni** bir bileşen setiyle kurar ve tüm okuma
//! yollarının anlık (snapshot) bir `Arc<SysMemKit>` üzerinden çalışmasını sağlar.
//!
//! Kurallar:
//! - Mod geçişi syscall katmanını da değiştirir; etiket değişimi yeterli değildir.
//! - Mevcut süreç handle'ı korunur (`from_handle`), yeniden handle hijack yapılmaz.
//! - Okuma thread'i, mod geçişi sırasında yarım kalmış bir bileşen seti görmez:
//!   her turda `kit()` ile tutarlı bir snapshot alır.

use std::sync::{Arc, RwLock};

use sysmemkit::{Mode, SysMemKit};

/// Farklı thread'lerin paylaştığı, moda göre yeniden kurulabilen bellek erişim yönlendiricisi.
pub struct MemoryRouter {
    inner: RwLock<Arc<SysMemKit>>,
}

impl MemoryRouter {
    pub fn new(kit: SysMemKit) -> Self {
        Self {
            inner: RwLock::new(Arc::new(kit)),
        }
    }

    /// Tutarlı bir kit anlık görüntüsü döner.
    ///
    /// Okuma sıcak yolunda kilit tutulmaz: `Arc` klonlanır, kilit bırakılır.
    pub fn kit(&self) -> Arc<SysMemKit> {
        Arc::clone(&self.inner.read().unwrap_or_else(|e| e.into_inner()))
    }

    #[inline]
    pub fn mode(&self) -> Mode {
        self.kit().mode()
    }

    #[inline]
    pub fn get_pid(&self) -> u32 {
        self.kit().get_pid()
    }

    /// Aktif modu değiştirir ve syscall katmanını yeniden kurar.
    ///
    /// `Mode::Rage` için ROP gadget taraması başarısız olursa `Mode::Indirect`
    /// katmanına düşer — sessizce eski (yanlış) katmanda kalmaz.
    pub fn apply_mode(&self, target: Mode) -> Result<Mode, String> {
        let current = self.kit();
        if current.mode() == target {
            return Ok(target);
        }

        let handle = current.get_handle();
        let pid = current.get_pid();

        let mut effective = target;
        let mut rebuilt = unsafe { SysMemKit::from_handle(handle, pid, effective) };

        if rebuilt.is_none() && effective == Mode::Rage {
            eprintln!("[!] ROP gadget katmanı kurulamadı, Indirect katmanına düşülüyor.");
            effective = Mode::Indirect;
            rebuilt = unsafe { SysMemKit::from_handle(handle, pid, effective) };
        }

        match rebuilt {
            Some(kit) => {
                let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());
                *guard = Arc::new(kit);
                Ok(effective)
            }
            None => Err(format!(
                "{:?} modu için syscall katmanı kurulamadı (mevcut katman korunuyor)",
                target
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::StealthMode;

    #[test]
    fn mode_mapping_is_symmetric() {
        for m in [
            StealthMode::Basic,
            StealthMode::DirectStealth,
            StealthMode::Indirect,
            StealthMode::Rage,
        ] {
            assert_eq!(StealthMode::from_sys_mode(m.to_sys_mode()), m);
        }
        for m in [Mode::Basic, Mode::DirectStealth, Mode::Indirect, Mode::Rage] {
            assert_eq!(StealthMode::from_sys_mode(m).to_sys_mode(), m);
        }
    }

    #[test]
    fn overlay_and_syscall_layers_stay_coupled() {
        // Aynı mod hem doğru overlay'i hem doğru syscall katmanını seçmeli.
        let imgui_modes = [StealthMode::Basic, StealthMode::DirectStealth];
        let discord_modes = [StealthMode::Indirect, StealthMode::Rage];

        for m in imgui_modes {
            assert!(m.uses_imgui() && !m.uses_discord());
            assert!(matches!(m.to_sys_mode(), Mode::Basic | Mode::DirectStealth));
        }
        for m in discord_modes {
            assert!(m.uses_discord() && !m.uses_imgui());
            assert!(matches!(m.to_sys_mode(), Mode::Indirect | Mode::Rage));
        }
    }
}
