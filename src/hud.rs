//! HUD gösterge paneli içeriği — TEK KAYNAK.
//!
//! Mod 1/2'de ImGui draw list, Mod 3/4'te Discord CDP canvas bu modelin aynısını
//! çizer. Böylece iki overlay farklı bilgi göstermez; içerik burada tanımlanır.
//!
//! Panel yalnızca `hud_visible` açıkken ve maç içindeyken üretilir (çağıran taraf
//! in-game kontrolünü yapar).

use crate::app::VisualSettings;
use crate::mode::ModeController;

/// Panel genişliği (piksel, CSS piksel = ImGui piksel).
pub const PANEL_WIDTH: f32 = 286.0;
/// Tek satır yüksekliği.
pub const ROW_HEIGHT: f32 = 17.0;
/// Panel iç boşluğu.
pub const PADDING: f32 = 9.0;
/// Panel sol-üst konumu.
pub const ORIGIN: [f32; 2] = [18.0, 18.0];

/// Paneldeki tek bir özellik satırı.
pub struct HudRow {
    pub label: &'static str,
    pub detail: String,
    pub on: bool,
}

/// Çizime hazır HUD modeli.
pub struct HudModel {
    pub header: String,
    pub footer: &'static str,
    pub rows: Vec<HudRow>,
}

impl HudModel {
    /// Panelin toplam yüksekliği: header + satırlar + footer + iç boşluklar.
    pub fn height(&self) -> f32 {
        PADDING * 2.0 + ROW_HEIGHT * (self.rows.len() as f32 + 2.0)
    }
}

fn flag(on: bool) -> String {
    if on { "ON".to_string() } else { "OFF".to_string() }
}

/// HUD modelini üretir. `hud_visible` kapalıysa `None` döner.
pub fn build(
    mode_controller: &ModeController,
    settings: &VisualSettings,
    visual_esp: bool,
    sound_esp: bool,
) -> Option<HudModel> {
    if !mode_controller.is_hud_visible() {
        return None;
    }

    let mode = mode_controller.mode();
    let rows = vec![
        HudRow {
            label: "MODE",
            detail: format!("{}/4 {}", mode.as_u8(), mode.short_name()),
            on: true,
        },
        HudRow {
            label: "OVERLAY",
            detail: mode.overlay_name().to_string(),
            on: true,
        },
        HudRow {
            label: "VISUAL ESP",
            detail: flag(visual_esp),
            on: visual_esp,
        },
        HudRow {
            label: "SOUND ESP",
            detail: flag(sound_esp),
            on: sound_esp,
        },
        HudRow {
            label: "BOX",
            detail: flag(settings.show_box),
            on: settings.show_box,
        },
        HudRow {
            label: "SKELETON",
            detail: flag(settings.show_skeleton),
            on: settings.show_skeleton,
        },
        HudRow {
            label: "HEALTH",
            detail: flag(settings.show_health),
            on: settings.show_health,
        },
        HudRow {
            label: "NAME",
            detail: flag(settings.show_name),
            on: settings.show_name,
        },
        HudRow {
            label: "WEAPON",
            detail: flag(settings.show_weapon),
            on: settings.show_weapon,
        },
        HudRow {
            label: "DRAW STYLE",
            detail: if settings.is_minimal { "MINIMAL" } else { "DETAILED" }.to_string(),
            on: !settings.is_minimal,
        },
    ];

    Some(HudModel {
        header: format!("cs2ru // {}", mode.name()),
        footer: "F9: mod  |  F11 / INS: HUD",
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::StealthMode;

    #[test]
    fn hud_hidden_returns_none() {
        let ctrl = ModeController::new(StealthMode::Basic);
        ctrl.set_hud_visible(false);
        assert!(build(&ctrl, &VisualSettings::default(), true, true).is_none());
    }

    #[test]
    fn hud_reports_mode_and_feature_flags() {
        let ctrl = ModeController::new(StealthMode::Indirect);
        ctrl.set_hud_visible(true);

        let mut settings = VisualSettings::default();
        settings.show_skeleton = false;

        let hud = build(&ctrl, &settings, true, false).expect("HUD açıkken model üretilmeli");

        assert!(hud.header.contains("Mode 3/4"), "header: {}", hud.header);

        let row = |label: &str| {
            hud.rows
                .iter()
                .find(|r| r.label == label)
                .unwrap_or_else(|| panic!("{label} satırı yok"))
        };
        assert_eq!(row("VISUAL ESP").detail, "ON");
        assert_eq!(row("OVERLAY").detail, "Discord V3 CDP");
        assert!(!row("SOUND ESP").on);
        assert_eq!(row("SKELETON").detail, "OFF");
        assert!(row("BOX").on);
        assert!((hud.height() - (PADDING * 2.0 + ROW_HEIGHT * (hud.rows.len() as f32 + 2.0))).abs() < 0.001);
    }
}
