//! Perceptually uniform OKLCH color system and egui styling.

use eframe::egui::{self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Margin, Stroke, Style, Visuals};

/// A color specified in perceptual OKLCH space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklch {
    /// Lightness in `0.0..=1.0`.
    pub l: f32,
    /// Chroma in `0.0..=~0.4`.
    pub c: f32,
    /// Hue angle in degrees (`0.0..360.0`).
    pub h: f32,
}

impl Oklch {
    /// Create a new OKLCH color.
    pub const fn new(l: f32, c: f32, h: f32) -> Self {
        Self { l, c, h }
    }

    /// Convert to an opaque `egui::Color32` with sRGB gamut clamping.
    pub fn to_color32(self) -> Color32 {
        self.with_alpha(1.0)
    }

    /// Convert to an `egui::Color32` with the given opacity (`0.0..=1.0`).
    pub fn with_alpha(self, alpha: f32) -> Color32 {
        let [r, g, b] = oklch_to_srgb_clamped(self.l, self.c, self.h);
        let a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
        Color32::from_rgba_unmultiplied(r, g, b, a)
    }
}

/// Studio color tokens authored in OKLCH.
#[derive(Debug, Clone, Copy)]
pub struct ThemeTokens {
    pub bg_canvas: Oklch,
    pub bg_panel: Oklch,
    pub bg_elevated: Oklch,
    pub border_subtle: Oklch,
    pub text_primary: Oklch,
    pub text_muted: Oklch,
    pub accent: Oklch,
    pub accent_hover: Oklch,
    pub success: Oklch,
    pub warning: Oklch,
    pub danger: Oklch,
}

impl Default for ThemeTokens {
    fn default() -> Self {
        Self {
            bg_canvas: Oklch::new(0.145, 0.012, 260.0),
            bg_panel: Oklch::new(0.185, 0.014, 260.0),
            bg_elevated: Oklch::new(0.230, 0.016, 260.0),
            border_subtle: Oklch::new(0.310, 0.018, 260.0),
            text_primary: Oklch::new(0.930, 0.008, 260.0),
            text_muted: Oklch::new(0.680, 0.015, 260.0),
            accent: Oklch::new(0.680, 0.175, 250.0),
            accent_hover: Oklch::new(0.730, 0.175, 250.0),
            success: Oklch::new(0.720, 0.150, 150.0),
            warning: Oklch::new(0.780, 0.145, 75.0),
            danger: Oklch::new(0.660, 0.185, 25.0),
        }
    }
}

impl ThemeTokens {
    /// Apply this OKLCH token set to the global `egui::Context` style.
    pub fn apply_to_egui(&self, ctx: &egui::Context) {
        let mut visuals = Visuals::dark();
        let canvas = self.bg_canvas.to_color32();
        let panel = self.bg_panel.to_color32();
        let elevated = self.bg_elevated.to_color32();
        let border = self.border_subtle.to_color32();
        let text = self.text_primary.to_color32();
        let accent = self.accent.to_color32();
        let accent_hover = self.accent_hover.to_color32();

        visuals.dark_mode = true;
        visuals.override_text_color = Some(text);
        visuals.panel_fill = panel;
        visuals.window_fill = panel;
        visuals.extreme_bg_color = canvas;
        visuals.faint_bg_color = elevated;
        visuals.selection.bg_fill = accent.gamma_multiply(0.35);
        visuals.selection.stroke = Stroke::new(1.0, accent);
        visuals.window_stroke = Stroke::new(1.0, border);
        visuals.window_corner_radius = CornerRadius::same(8);

        visuals.widgets.noninteractive.bg_fill = panel;
        visuals.widgets.noninteractive.weak_bg_fill = panel;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, border);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, text);
        visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);

        visuals.widgets.inactive.bg_fill = elevated;
        visuals.widgets.inactive.weak_bg_fill = elevated;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, border);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, text);
        visuals.widgets.inactive.corner_radius = CornerRadius::same(6);

        visuals.widgets.hovered.bg_fill = self.bg_elevated.with_alpha(1.0);
        visuals.widgets.hovered.weak_bg_fill = elevated;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, accent_hover);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.5, text);
        visuals.widgets.hovered.corner_radius = CornerRadius::same(6);

        visuals.widgets.active.bg_fill = accent;
        visuals.widgets.active.weak_bg_fill = accent;
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, accent_hover);
        visuals.widgets.active.fg_stroke = Stroke::new(1.5, text);
        visuals.widgets.active.corner_radius = CornerRadius::same(6);

        visuals.widgets.open.bg_fill = elevated;
        visuals.widgets.open.bg_stroke = Stroke::new(1.0, accent);
        visuals.widgets.open.corner_radius = CornerRadius::same(6);

        let mut style = Style {
            visuals,
            ..Default::default()
        };
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        style.spacing.window_margin = Margin::same(12);
        style.spacing.slider_width = 160.0;

        ctx.set_style(style);
    }

    /// Register system CJK fallback fonts (Chinese, Japanese, Korean) on Windows, macOS, and Linux.
    pub fn install_cjk_font_fallback(&self, ctx: &egui::Context) {
        #[cfg(target_os = "windows")]
        let script_groups: [(&str, &[&str]); 3] = [
            (
                "cjk_sc",
                &[
                    "C:\\Windows\\Fonts\\msyh.ttc",
                    "C:\\Windows\\Fonts\\simsun.ttc",
                ],
            ),
            (
                "cjk_jp",
                &[
                    "C:\\Windows\\Fonts\\YuGothR.ttc",
                    "C:\\Windows\\Fonts\\meiryo.ttc",
                    "C:\\Windows\\Fonts\\msgothic.ttc",
                ],
            ),
            ("cjk_kr", &["C:\\Windows\\Fonts\\malgun.ttf"]),
        ];

        #[cfg(target_os = "macos")]
        let script_groups: [(&str, &[&str]); 3] = [
            (
                "cjk_sc",
                &[
                    "/System/Library/Fonts/PingFang.ttc",
                    "/System/Library/Fonts/STHeiti Light.ttc",
                ],
            ),
            (
                "cjk_jp",
                &[
                    "/System/Library/Fonts/Hiragino Sans GB.ttc",
                    "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc",
                ],
            ),
            (
                "cjk_kr",
                &["/System/Library/Fonts/AppleSDGothicNeo.ttc"],
            ),
        ];

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        let script_groups: [(&str, &[&str]); 3] = [
            (
                "cjk_sc",
                &[
                    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
                    "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
                ],
            ),
            (
                "cjk_jp",
                &[
                    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
                ],
            ),
            (
                "cjk_kr",
                &["/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc"],
            ),
        ];

        let mut fonts = FontDefinitions::default();
        let mut added = false;

        for (key, paths) in script_groups {
            for path in paths {
                if let Ok(bytes) = std::fs::read(path) {
                    fonts.font_data.insert(
                        key.to_owned(),
                        std::sync::Arc::new(FontData::from_owned(bytes)),
                    );
                    if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
                        family.push(key.to_owned());
                    }
                    if let Some(family) = fonts.families.get_mut(&FontFamily::Monospace) {
                        family.push(key.to_owned());
                    }
                    added = true;
                    break;
                }
            }
        }

        if added {
            ctx.set_fonts(fonts);
        }
    }
}

/// Convert OKLCH `(l, c, h_deg)` to linear sRGB `[r, g, b]`.
fn oklch_to_linear_srgb(l: f32, c: f32, h_deg: f32) -> [f32; 3] {
    let h_rad = h_deg.to_radians();
    let a = c * h_rad.cos();
    let b = c * h_rad.sin();

    let l_ = l + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_ = l - 0.105_561_346 * a - 0.063_854_17 * b;
    let s_ = l - 0.089_484_18 * a - 1.291_485_5 * b;

    let l3 = l_ * l_ * l_;
    let m3 = m_ * m_ * m_;
    let s3 = s_ * s_ * s_;

    let r = 4.076_741_7 * l3 - 3.307_711_6 * m3 + 0.230_969_94 * s3;
    let g = -1.268_438 * l3 + 2.609_757_4 * m3 - 0.341_319_38 * s3;
    let b_lin = -0.004_196_086_3 * l3 - 0.703_418_6 * m3 + 1.707_614_7 * s3;

    [r, g, b_lin]
}

fn in_srgb_gamut(rgb: [f32; 3]) -> bool {
    rgb.iter().all(|&v| (0.0..=1.0).contains(&v))
}

fn linear_to_srgb_u8(x: f32) -> u8 {
    let clamped = x.clamp(0.0, 1.0);
    let encoded = if clamped <= 0.003_130_8 {
        12.92 * clamped
    } else {
        1.055 * clamped.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round().clamp(0.0, 255.0) as u8
}

/// Convert OKLCH to sRGB `[u8; 3]`, clamping chroma by binary search if out of gamut.
pub fn oklch_to_srgb_clamped(l: f32, c: f32, h_deg: f32) -> [u8; 3] {
    let l = l.clamp(0.0, 1.0);
    let c = c.max(0.0);

    let direct = oklch_to_linear_srgb(l, c, h_deg);
    let lin = if in_srgb_gamut(direct) {
        direct
    } else {
        let mut lo = 0.0_f32;
        let mut hi = c;
        for _ in 0..16 {
            let mid = 0.5 * (lo + hi);
            if in_srgb_gamut(oklch_to_linear_srgb(l, mid, h_deg)) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        oklch_to_linear_srgb(l, lo, h_deg)
    };

    [
        linear_to_srgb_u8(lin[0]),
        linear_to_srgb_u8(lin[1]),
        linear_to_srgb_u8(lin[2]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_and_white_convert_accurately() {
        assert_eq!(oklch_to_srgb_clamped(0.0, 0.0, 0.0), [0, 0, 0]);
        assert_eq!(oklch_to_srgb_clamped(1.0, 0.0, 0.0), [255, 255, 255]);
    }

    #[test]
    fn out_of_gamut_chroma_clamps_without_overflow() {
        let rgb = oklch_to_srgb_clamped(0.7, 0.45, 145.0);
        assert!(rgb.iter().any(|&v| v > 0));
    }
}
