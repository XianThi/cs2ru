//! Ortak ESP Geometrisi ve Çizim Komutu Katmanı — TEK KAYNAK.
//!
//! Hem ImGui (`app.rs`) hem de Discord V3 CDP Canvas (`discord_overlay.rs`) yolları
//! dünya-ekran izdüşümü (World-to-Screen), sınırlayıcı kutu, sağlık/zırh çubukları,
//! iskelet bağlantıları, taktiksel durum etiketleri (`DEFUSING`, `SCOPED`, `FLASHED`)
//! ve kurulu C4 göstergesi için bu modüldeki `build_esp_draw_list` fonksiyonunu kullanır.

use crate::app::{EspSnapshot, VisualSettings};
use crate::vec3::Vec3;

pub const BONE_PAIRS: &[(usize, usize)] = &[
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorRgba(pub u8, pub u8, pub u8, pub u8);

impl ColorRgba {
    #[inline]
    pub fn to_f32_array(self) -> [f32; 4] {
        [
            self.0 as f32 / 255.0,
            self.1 as f32 / 255.0,
            self.2 as f32 / 255.0,
            self.3 as f32 / 255.0,
        ]
    }

    #[inline]
    pub fn to_css(self) -> String {
        let Self(r, g, b, a) = self;
        if a >= 255 {
            format!("rgb({r},{g},{b})")
        } else {
            format!("rgba({r},{g},{b},{:.2})", a as f32 / 255.0)
        }
    }
}

#[derive(Debug, Clone)]
pub struct RectCmd {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub thickness: f32,
    pub color: ColorRgba,
}

#[derive(Debug, Clone)]
pub struct FilledRectCmd {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub color: ColorRgba,
}

#[derive(Debug, Clone)]
pub struct LineCmd {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub thickness: f32,
    pub color: ColorRgba,
}

#[derive(Debug, Clone)]
pub struct CircleCmd {
    pub cx: f32,
    pub cy: f32,
    pub radius: f32,
    pub thickness: f32,
    pub color: ColorRgba,
}

#[derive(Debug, Clone)]
pub struct TextCmd {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub color: ColorRgba,
}

#[derive(Debug, Clone, Default)]
pub struct EspDrawList {
    pub filled_rects: Vec<FilledRectCmd>,
    pub lines: Vec<LineCmd>,
    pub rects: Vec<RectCmd>,
    pub circles: Vec<CircleCmd>,
    pub texts: Vec<TextCmd>,
}

/// Anlık oyun durumundan (`EspSnapshot`) ve görsel ayarlardan (`VisualSettings`)
/// bağımsız çizim komut listesi üretir.
pub fn build_esp_draw_list(
    snapshot: &EspSnapshot,
    settings: &VisualSettings,
    display_w: f32,
    display_h: f32,
) -> EspDrawList {
    let mut list = EspDrawList::default();

    if display_w <= 0.0 || display_h <= 0.0 {
        return list;
    }

    let is_minimal = settings.is_minimal;

    for player in &snapshot.players {
        if player.health <= 0 {
            continue;
        }

        let is_enemy = snapshot.local_team != 0 && player.team != snapshot.local_team;
        let base_color = if is_enemy {
            if player.defusing {
                ColorRgba(255, 215, 0, 255) // İmha eden düşman: Altın sarısı vurgu
            } else {
                ColorRgba(255, 65, 65, 255) // Düşman: Kırmızı
            }
        } else {
            ColorRgba(65, 160, 255, 255) // Takım arkadaşı: Mavi
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

        if is_minimal {
            if settings.show_box {
                list.rects.push(RectCmd {
                    x: min_x,
                    y: min_y,
                    w: box_width,
                    h: box_height,
                    thickness: 1.0,
                    color: base_color,
                });
            }
            continue;
        }

        // A) Sınırlayıcı Kutu (Kontrastlı dış ve iç siyah kenarlık)
        if settings.show_box {
            list.rects.push(RectCmd {
                x: min_x - 1.0,
                y: min_y - 1.0,
                w: box_width + 2.0,
                h: box_height + 2.0,
                thickness: 1.0,
                color: ColorRgba(0, 0, 0, 255),
            });
            list.rects.push(RectCmd {
                x: min_x,
                y: min_y,
                w: box_width,
                h: box_height,
                thickness: 1.0,
                color: base_color,
            });
            list.rects.push(RectCmd {
                x: min_x + 1.0,
                y: min_y + 1.0,
                w: (box_width - 2.0).max(1.0),
                h: (box_height - 2.0).max(1.0),
                thickness: 1.0,
                color: ColorRgba(0, 0, 0, 255),
            });
        }

        // B) Can Barı (Sol kenarda dikey çubuk) + Zırh Barı (varsa)
        if settings.show_health {
            let bar_w = (box_width * 0.08).clamp(2.0, 4.0);
            let bar_x = min_x - 4.0 - bar_w;

            list.filled_rects.push(FilledRectCmd {
                x: bar_x - 1.0,
                y: min_y - 1.0,
                w: bar_w + 2.0,
                h: box_height + 2.0,
                color: ColorRgba(0, 0, 0, 220),
            });

            let hp_pct = (player.health.clamp(0, 100) as f32) / 100.0;
            let hp_h = box_height * hp_pct;
            let hp_top_y = max_y - hp_h;
            let hp_color = if player.health > 60 {
                ColorRgba(50, 220, 50, 255)
            } else if player.health > 25 {
                ColorRgba(240, 180, 20, 255)
            } else {
                ColorRgba(240, 40, 40, 255)
            };

            list.filled_rects.push(FilledRectCmd {
                x: bar_x,
                y: hp_top_y,
                w: bar_w,
                h: hp_h,
                color: hp_color,
            });

            if player.health < 100 {
                list.texts.push(TextCmd {
                    x: bar_x - 16.0,
                    y: hp_top_y - 2.0,
                    text: player.health.to_string(),
                    color: ColorRgba(255, 255, 255, 255),
                });
            }

            if player.armor > 0 {
                let armor_pct = (player.armor.clamp(0, 100) as f32) / 100.0;
                let armor_w = box_width * armor_pct;
                let armor_y = max_y + 2.0;
                list.filled_rects.push(FilledRectCmd {
                    x: min_x - 1.0,
                    y: armor_y,
                    w: box_width + 2.0,
                    h: 3.0,
                    color: ColorRgba(0, 0, 0, 200),
                });
                list.filled_rects.push(FilledRectCmd {
                    x: min_x,
                    y: armor_y + 0.5,
                    w: armor_w,
                    h: 2.0,
                    color: ColorRgba(70, 170, 255, 255),
                });
            }
        }

        // C) Oyuncu İsmi (Kutunun üstünde)
        if settings.show_name && !player.name.is_empty() {
            list.texts.push(TextCmd {
                x: min_x,
                y: min_y - 14.0,
                text: player.name.clone(),
                color: ColorRgba(255, 255, 255, 255),
            });
        }

        // D) Silah İsmi ve Taktiksel Durum Etiketleri (Kutunun altında)
        if settings.show_weapon {
            let mut bottom_y = if settings.show_health && player.armor > 0 {
                max_y + 6.0
            } else {
                max_y + 2.0
            };

            if !player.weapon.is_empty() {
                list.texts.push(TextCmd {
                    x: min_x,
                    y: bottom_y,
                    text: player.weapon.clone(),
                    color: ColorRgba(210, 210, 210, 255),
                });
                bottom_y += 13.0;
            }

            let mut flags = Vec::new();
            if player.defusing {
                flags.push("DEFUSING");
            }
            if player.scoped {
                flags.push("SCOPED");
            }
            if player.flashed {
                flags.push("FLASHED");
            }
            if !flags.is_empty() {
                list.texts.push(TextCmd {
                    x: min_x,
                    y: bottom_y,
                    text: flags.join(" | "),
                    color: ColorRgba(255, 215, 60, 255),
                });
            }
        }

        // E) İskelet / Kemik Hatları (Skeleton)
        if settings.show_skeleton && player.bone_list.len() >= 28 {
            let skeleton_color = ColorRgba(255, 255, 255, 200);
            for &(b1, b2) in BONE_PAIRS {
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
                        list.lines.push(LineCmd {
                            x1: pt1.0,
                            y1: pt1.1,
                            x2: pt2.0,
                            y2: pt2.1,
                            thickness: 1.2,
                            color: skeleton_color,
                        });
                    }
                }
            }

            if let Some(head_pt) =
                snapshot
                    .view_matrix
                    .world_to_screen(&player.bone_list[6], display_w, display_h)
            {
                let radius = (box_height * 0.07).clamp(2.5, 7.0);
                list.circles.push(CircleCmd {
                    cx: head_pt.0,
                    cy: head_pt.1,
                    radius,
                    thickness: 1.2,
                    color: skeleton_color,
                });
            }
        }
    }

    // F) Kurulu C4 Göstergesi (Planted Bomb ESP)
    if let Some(ref bomb) = snapshot.planted_c4 {
        if bomb.is_planted {
            if let Some((bx, by)) =
                snapshot
                    .view_matrix
                    .world_to_screen(&bomb.pos, display_w, display_h)
            {
                let site_label = match bomb.site {
                    0 => "A",
                    1 => "B",
                    _ => "?",
                };
                let label = format!("[C4 - SITE {site_label}]");
                list.rects.push(RectCmd {
                    x: bx - 6.0,
                    y: by - 6.0,
                    w: 12.0,
                    h: 12.0,
                    thickness: 1.5,
                    color: ColorRgba(255, 180, 30, 255),
                });
                list.texts.push(TextCmd {
                    x: bx + 9.0,
                    y: by - 7.0,
                    text: label,
                    color: ColorRgba(255, 200, 50, 255),
                });
            }
        }
    }

    list
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{PlantedC4Snapshot, PlayerSnapshot};
    use crate::matrix::ViewMatrix;

    fn identity_view() -> ViewMatrix {
        ViewMatrix {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 100.0],
            ],
        }
    }

    #[test]
    fn esp_builds_tactical_flags_and_c4() {
        let snap = EspSnapshot {
            view_matrix: identity_view(),
            local_team: 2,
            has_local: true,
            is_in_game: true,
            map_name: "de_mirage".to_string(),
            players: vec![PlayerSnapshot {
                pos: Vec3::new(0.0, -20.0, 0.0),
                head_pos: Vec3::new(0.0, 20.0, 0.0),
                bone_list: vec![Vec3::zero(); 30],
                health: 75,
                armor: 100,
                team: 3,
                name: "Target1".to_string(),
                weapon: "ak47".to_string(),
                scoped: true,
                defusing: true,
                flashed: false,
            }],
            planted_c4: Some(PlantedC4Snapshot {
                pos: Vec3::new(10.0, 10.0, 0.0),
                site: 0,
                is_planted: true,
            }),
        };

        let settings = VisualSettings::default();
        let cmds = build_esp_draw_list(&snap, &settings, 1920.0, 1080.0);

        assert!(!cmds.rects.is_empty(), "bounding box ve C4 kutusu üretilmeli");
        assert!(!cmds.filled_rects.is_empty(), "HP ve zırh barı üretilmeli");
        assert!(
            cmds.texts.iter().any(|t| t.text.contains("DEFUSING | SCOPED")),
            "taktiksel durum etiketleri üretilmeli"
        );
        assert!(
            cmds.texts.iter().any(|t| t.text.contains("[C4 - SITE A]")),
            "kurulu C4 etiketi üretilmeli"
        );
    }

    #[test]
    fn esp_minimal_mode_suppresses_details() {
        let snap = EspSnapshot {
            view_matrix: identity_view(),
            local_team: 2,
            has_local: true,
            is_in_game: true,
            map_name: "de_mirage".to_string(),
            players: vec![PlayerSnapshot {
                pos: Vec3::new(0.0, -20.0, 0.0),
                head_pos: Vec3::new(0.0, 20.0, 0.0),
                bone_list: vec![Vec3::zero(); 30],
                health: 80,
                armor: 50,
                team: 3,
                name: "Target1".to_string(),
                weapon: "awp".to_string(),
                scoped: true,
                defusing: false,
                flashed: true,
            }],
            planted_c4: None,
        };

        let mut settings = VisualSettings::default();
        settings.is_minimal = true;
        let cmds = build_esp_draw_list(&snap, &settings, 1920.0, 1080.0);

        assert_eq!(cmds.rects.len(), 1, "minimal modda yalnızca tek kutu çizilmeli");
        assert!(cmds.filled_rects.is_empty(), "minimal modda HP/zırh barı olmamalı");
        assert!(cmds.texts.is_empty(), "minimal modda metin olmamalı");
    }
}
