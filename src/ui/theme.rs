//! Тема оформления: палитра с проверенным контрастом, стиль egui, шрифты.

use crate::state::progress::{Progress, MAX_FONT_SCALE, MIN_FONT_SCALE};
use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, Visuals};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Монохромный Noto Emoji (OFL): в шрифтах egui нет эмодзи после Unicode 11 — 🩸 🧪 🥋 рисовались квадратами.
const EMOJI_FONT: &[u8] = include_bytes!("../../assets/fonts/NotoEmoji-Regular.ttf");

pub const DARK_PANEL: Color32 = Color32::from_rgb(18, 20, 26);
pub const LIGHT_PANEL: Color32 = Color32::from_rgb(245, 245, 248);

static DARK: AtomicBool = AtomicBool::new(true);

pub fn is_dark() -> bool {
    DARK.load(Ordering::Relaxed)
}

fn pick(dark: (u8, u8, u8), light: (u8, u8, u8)) -> Color32 {
    let (r, g, b) = if is_dark() { dark } else { light };
    Color32::from_rgb(r, g, b)
}

pub fn accent() -> Color32 {
    pick((240, 90, 100), (176, 24, 40))
}

pub fn good() -> Color32 {
    pick((90, 200, 120), (20, 110, 55))
}

pub fn warn() -> Color32 {
    pick((240, 180, 70), (138, 82, 0))
}

pub fn info() -> Color32 {
    pick((120, 170, 230), (25, 85, 165))
}

pub fn purple() -> Color32 {
    pick((200, 140, 255), (105, 45, 165))
}

/// Фон блоков с кодом и дизассемблером.
pub fn code_bg() -> Color32 {
    pick((12, 13, 18), (232, 232, 238))
}

/// Тонировка карточки: в тёмной теме — цвет как есть, в светлой — бледный оттенок того же тона.
pub fn soft(r: u8, g: u8, b: u8) -> Color32 {
    if is_dark() {
        Color32::from_rgb(r, g, b)
    } else {
        let lift = |c: u8| (255.0 - (255.0 - f32::from(c)) * 0.14).round() as u8;
        Color32::from_rgb(lift(r), lift(g), lift(b))
    }
}

fn gray(v: u8) -> Color32 {
    Color32::from_gray(v)
}

/// Полная установка: шрифты и стиль. Вызывается один раз при создании окна.
pub fn install(ctx: &egui::Context, progress: &Progress) {
    install_fonts(ctx);
    apply(ctx, progress);
}

/// Шрифты приложения: стандартные egui + Noto Emoji + Hack как запасной для стрелок и рамок.
pub fn font_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "noto-emoji".into(),
        Arc::new(FontData::from_static(EMOJI_FONT)),
    );
    // Ubuntu-Light (основной шрифт) не содержит стрелок, рамок и геометрических фигур; они есть в Hack,
    // который egui уже поставляет, но подключает только для моноширинного текста.
    for (family, extra) in [
        (FontFamily::Proportional, vec!["Hack", "noto-emoji"]),
        (FontFamily::Monospace, vec!["noto-emoji"]),
    ] {
        let list = fonts.families.entry(family).or_default();
        for (offset, name) in extra.into_iter().enumerate() {
            if !list.iter().any(|n| n == name) {
                let at = (1 + offset).min(list.len());
                list.insert(at, name.into());
            }
        }
    }
    fonts
}

pub fn install_fonts(ctx: &egui::Context) {
    ctx.set_fonts(font_definitions());
}

/// Тема и размер шрифта. Всегда строится от стандартного стиля, поэтому повторный вызов идемпотентен
/// (раньше каждый вызов умножал уже увеличенные шрифты, и «A+» уменьшал текст).
pub fn apply(ctx: &egui::Context, progress: &Progress) {
    let light = progress.is_light();
    DARK.store(!light, Ordering::Relaxed);
    let mut v = if light {
        Visuals::light()
    } else {
        Visuals::dark()
    };
    if light {
        v.panel_fill = LIGHT_PANEL;
        v.window_fill = Color32::WHITE;
        v.extreme_bg_color = Color32::from_rgb(232, 232, 238);
        v.widgets.noninteractive.fg_stroke.color = gray(35);
        v.weak_text_color = Some(gray(100));
        v.hyperlink_color = Color32::from_rgb(25, 85, 165);
    } else {
        v.panel_fill = DARK_PANEL;
        v.window_fill = Color32::from_rgb(22, 24, 32);
        v.extreme_bg_color = Color32::from_rgb(12, 13, 18);
        v.widgets.noninteractive.fg_stroke.color = gray(205);
        v.weak_text_color = Some(gray(150));
        v.hyperlink_color = Color32::from_rgb(255, 110, 110);
    }
    v.selection.bg_fill = Color32::from_rgb(200, 40, 60);
    let mut style = egui::Style {
        visuals: v,
        ..egui::Style::default()
    };
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    let scale = progress.font_scale.clamp(MIN_FONT_SCALE, MAX_FONT_SCALE);
    for font in style.text_styles.values_mut() {
        font.size = (font.size * scale).clamp(8.0, 48.0);
    }
    ctx.set_global_style(style);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(c: Color32) -> f64 {
        let lin = |v: u8| {
            let s = f64::from(v) / 255.0;
            if s <= 0.03928 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
    }

    fn contrast(a: Color32, b: Color32) -> f64 {
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    /// Единственный тест, меняющий глобальный флаг темы: проверяет обе темы последовательно.
    #[test]
    fn palette_is_readable_in_both_themes() {
        for light in [false, true] {
            let ctx = egui::Context::default();
            let progress = Progress {
                theme: if light { "light" } else { "dark" }.into(),
                ..Progress::default()
            };
            apply(&ctx, &progress);
            let panel = if light { LIGHT_PANEL } else { DARK_PANEL };
            let style = ctx.global_style();
            for (name, color) in [
                ("accent", accent()),
                ("good", good()),
                ("warn", warn()),
                ("info", info()),
                ("purple", purple()),
            ] {
                let c = contrast(color, panel);
                assert!(
                    c >= 4.5,
                    "{name} в {} теме: контраст {c:.2} < 4.5",
                    if light {
                        "светлой"
                    } else {
                        "тёмной"
                    }
                );
            }
            let text = style.visuals.widgets.noninteractive.fg_stroke.color;
            assert!(
                contrast(text, panel) >= 7.0,
                "основной текст ({light}): {:.2}",
                contrast(text, panel)
            );
            let weak = style
                .visuals
                .weak_text_color
                .expect("слабый текст задан явно");
            assert!(
                contrast(weak, panel) >= 4.5,
                "слабый текст ({light}): {:.2}",
                contrast(weak, panel)
            );
            assert!(
                contrast(style.visuals.hyperlink_color, panel) >= 4.5,
                "ссылки ({light})"
            );
            assert!(
                contrast(accent(), soft(38, 30, 40)) >= 4.5,
                "акцент на тонированной карточке ({light})"
            );
            assert!(
                contrast(text, code_bg()) >= 7.0,
                "текст на блоке кода ({light})"
            );
        }
        apply(&egui::Context::default(), &Progress::default());
    }

    #[test]
    fn applying_twice_does_not_compound_the_font_scale() {
        // регресс: apply_style брал текущий стиль и умножал размеры ещё раз
        let ctx = egui::Context::default();
        let progress = Progress {
            font_scale: 1.5,
            ..Progress::default()
        };
        let base = egui::Style::default().text_styles[&egui::TextStyle::Body].size;
        apply(&ctx, &progress);
        let once = ctx.global_style().text_styles[&egui::TextStyle::Body].size;
        apply(&ctx, &progress);
        apply(&ctx, &progress);
        let thrice = ctx.global_style().text_styles[&egui::TextStyle::Body].size;
        assert_eq!(once, base * 1.5);
        assert_eq!(once, thrice);
    }

    #[test]
    fn font_scale_is_clamped() {
        let ctx = egui::Context::default();
        let base = egui::Style::default().text_styles[&egui::TextStyle::Body].size;
        apply(
            &ctx,
            &Progress {
                font_scale: 40.0,
                ..Progress::default()
            },
        );
        assert_eq!(
            ctx.global_style().text_styles[&egui::TextStyle::Body].size,
            base * MAX_FONT_SCALE
        );
        apply(
            &ctx,
            &Progress {
                font_scale: 0.0,
                ..Progress::default()
            },
        );
        assert_eq!(
            ctx.global_style().text_styles[&egui::TextStyle::Body].size,
            base * MIN_FONT_SCALE
        );
    }
}
