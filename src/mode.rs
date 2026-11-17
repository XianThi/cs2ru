//! Stealth 4-Mode Denetleyicisi ve Overlay Yönlendirici Durumu
//!
//! CS2RU mimarisinde 4 stealth modu bulunur:
//! - Mode 1: Basic (Direct Syscall - saf export byte) -> ImGui Overlay
//! - Mode 2: DirectStealth (Direct Syscall - Halo/Tartarus Gate) -> ImGui Overlay
//! - Mode 3: Indirect (Indirect Syscall - ntdll trampoline) -> Discord V3 Overlay
//! - Mode 4: Rage (Indirect ROP Gadget Syscall) -> Discord V3 Overlay
//!
//! Kural:
//! - Mod 1 & 2'de ImGui penceresi aktiftir, Discord overlay dormant kalır.
//! - Mod 3 & 4'te ImGui penceresi kesinlikle çizim yapmaz ve gizlidir (`SW_HIDE`),
//!   yalnızca Discord overlay (CDP canvas) çalışır.
//! - İki overlay ASLA aynı anda çizim yapmaz.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;

use sysmemkit::Mode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StealthMode {
    /// Seviye 1: Standart PEB okuma + ntdll Export tablosundan doğrudan byte taraması + Direct Syscall.
    Basic = 1,
    /// Seviye 2: Hook farkındalıklı SSN çözümleme (Halo's/Tartarus' Gate + RVA) + Direct Syscall.
    DirectStealth = 2,
    /// Seviye 3: Indirect Syscall (ntdll fonksiyonunun kendi içindeki `syscall; ret` talimatına atlama).
    Indirect = 3,
    /// Seviye 4: Indirect ROP Gadget Syscall + Stack Trampoline.
    Rage = 4,
}

impl StealthMode {
    pub fn from_u8(val: u8) -> Self {
        match val {
            1 => StealthMode::Basic,
            2 => StealthMode::DirectStealth,
            3 => StealthMode::Indirect,
            4 => StealthMode::Rage,
            _ => StealthMode::Basic,
        }
    }

    pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    pub fn name(&self) -> &'static str {
        match self {
            StealthMode::Basic => "Mode 1/4: Basic (Direct Syscall)",
            StealthMode::DirectStealth => "Mode 2/4: DirectStealth (Direct Syscall)",
            StealthMode::Indirect => "Mode 3/4: Indirect (Syscall Trampoline)",
            StealthMode::Rage => "Mode 4/4: Rage (ROP Gadget Syscall)",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            StealthMode::Basic => "Basic (Direct)",
            StealthMode::DirectStealth => "DirectStealth",
            StealthMode::Indirect => "Indirect Syscall",
            StealthMode::Rage => "Rage (ROP Gadget)",
        }
    }

    pub fn overlay_name(&self) -> &'static str {
        match self {
            StealthMode::Basic | StealthMode::DirectStealth => "ImGui Overlay",
            StealthMode::Indirect | StealthMode::Rage => "Discord V3 CDP",
        }
    }

    #[inline]
    pub fn uses_imgui(&self) -> bool {
        matches!(self, StealthMode::Basic | StealthMode::DirectStealth)
    }

    #[inline]
    pub fn uses_discord(&self) -> bool {
        matches!(self, StealthMode::Indirect | StealthMode::Rage)
    }

    pub fn next(&self) -> Self {
        match self {
            StealthMode::Basic => StealthMode::DirectStealth,
            StealthMode::DirectStealth => StealthMode::Indirect,
            StealthMode::Indirect => StealthMode::Rage,
            StealthMode::Rage => StealthMode::Basic,
        }
    }

    /// Bu modun sysmemkit tarafındaki karşılığı.
    ///
    /// Mod geçişi yalnızca overlay'i değil, syscall katmanını da değiştirmelidir;
    /// aksi halde "mod" sadece kozmetik bir etiket olur.
    pub fn to_sys_mode(&self) -> Mode {
        match self {
            StealthMode::Basic => Mode::Basic,
            StealthMode::DirectStealth => Mode::DirectStealth,
            StealthMode::Indirect => Mode::Indirect,
            StealthMode::Rage => Mode::Rage,
        }
    }

    /// sysmemkit modundan denetleyici moduna eşleme (başlangıç seçimi için).
    pub fn from_sys_mode(mode: Mode) -> Self {
        match mode {
            Mode::Basic => StealthMode::Basic,
            Mode::DirectStealth => StealthMode::DirectStealth,
            Mode::Indirect => StealthMode::Indirect,
            Mode::Rage => StealthMode::Rage,
        }
    }
}

/// Tüm thread'ler (okuyucu, ImGui, Discord CDP) arasında paylaşılan mod denetleyicisi
#[derive(Clone)]
pub struct ModeController {
    active_mode: Arc<AtomicU8>,
    hud_visible: Arc<AtomicBool>,
}

impl ModeController {
    pub fn new(initial_mode: StealthMode) -> Self {
        Self {
            active_mode: Arc::new(AtomicU8::new(initial_mode.as_u8())),
            hud_visible: Arc::new(AtomicBool::new(true)), // Varsayılan olarak açık
        }
    }

    #[inline]
    pub fn mode(&self) -> StealthMode {
        StealthMode::from_u8(self.active_mode.load(Ordering::Relaxed))
    }

    #[inline]
    pub fn set_mode(&self, mode: StealthMode) {
        self.active_mode.store(mode.as_u8(), Ordering::Relaxed);
    }

    pub fn cycle_mode(&self) -> StealthMode {
        let current = self.mode();
        let next = current.next();
        self.set_mode(next);
        next
    }

    #[inline]
    pub fn is_hud_visible(&self) -> bool {
        self.hud_visible.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_hud_visible(&self, visible: bool) {
        self.hud_visible.store(visible, Ordering::Relaxed);
    }

    pub fn toggle_hud(&self) -> bool {
        let cur = self.is_hud_visible();
        self.set_hud_visible(!cur);
        !cur
    }

    #[inline]
    pub fn is_imgui_active(&self) -> bool {
        self.mode().uses_imgui()
    }

    #[inline]
    pub fn is_discord_active(&self) -> bool {
        self.mode().uses_discord()
    }

    pub fn active_mode_handle(&self) -> Arc<AtomicU8> {
        self.active_mode.clone()
    }

    pub fn hud_visible_handle(&self) -> Arc<AtomicBool> {
        self.hud_visible.clone()
    }
}
