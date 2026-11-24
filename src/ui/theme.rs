//! Colors, fonts and widget style: a deep violet-black theme with violet/pink and cyan accents.

use eframe::egui::{
    self, epaint::Shadow, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId,
    Stroke, TextStyle, Vec2,
};
use std::sync::Arc;

pub const BG0: Color32 = Color32::from_rgb(0x0e, 0x0c, 0x15); // graph / app background
pub const BG1: Color32 = Color32::from_rgb(0x14, 0x11, 0x1e); // side panels
pub const BG2: Color32 = Color32::from_rgb(0x1a, 0x16, 0x27); // toolbars, headers
pub const BG3: Color32 = Color32::from_rgb(0x23, 0x1e, 0x35); // inputs, hovered items
pub const BG4: Color32 = Color32::from_rgb(0x2e, 0x27, 0x45); // pressed, strong surfaces
pub const BORDER: Color32 = Color32::from_rgb(0x2a, 0x24, 0x3d);
pub const BORDER_SOFT: Color32 = Color32::from_rgb(0x1f, 0x1a, 0x2e);
/// Cartoon ink: the dark outline and hard drop shadow around stickers, buttons and dots.
pub const INK: Color32 = Color32::from_rgb(0x05, 0x03, 0x0a);
/// The visible 2 px outline of panels and inputs.
pub const OUTLINE: Color32 = Color32::from_rgb(0x2e, 0x26, 0x46);

pub const TEXT: Color32 = Color32::from_rgb(0xee, 0xea, 0xf8);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0xa8, 0xa0, 0xc2);
pub const TEXT_FAINT: Color32 = Color32::from_rgb(0x6e, 0x66, 0x8a);

pub const VIOLET: Color32 = Color32::from_rgb(0xa8, 0x55, 0xf7);
pub const VIOLET_DEEP: Color32 = Color32::from_rgb(0x7c, 0x3a, 0xed);
pub const PINK: Color32 = Color32::from_rgb(0xec, 0x48, 0x99);
pub const CYAN: Color32 = Color32::from_rgb(0x22, 0xd3, 0xee);

pub const ADDED: Color32 = Color32::from_rgb(0x34, 0xd3, 0x99);
pub const REMOVED: Color32 = Color32::from_rgb(0xf4, 0x3f, 0x5e);
pub const MODIFIED: Color32 = Color32::from_rgb(0xfb, 0xbf, 0x24);
pub const RENAMED: Color32 = CYAN;
pub const WARNING: Color32 = Color32::from_rgb(0xfb, 0x92, 0x3c);

/// Ten lane colors, one per graph column,
/// re-tuned to the requested neon violet / pink / cyan family.
pub const LANES: [Color32; 10] = [
    Color32::from_rgb(0x22, 0xd3, 0xee), // cyan
    Color32::from_rgb(0xa8, 0x55, 0xf7), // violet
    Color32::from_rgb(0xec, 0x48, 0x99), // pink
    Color32::from_rgb(0x3b, 0x82, 0xf6), // blue
    Color32::from_rgb(0xd9, 0x46, 0xef), // magenta
    Color32::from_rgb(0x2d, 0xd4, 0xbf), // teal
    Color32::from_rgb(0xfb, 0x71, 0x85), // rose
    Color32::from_rgb(0x81, 0x8c, 0xf8), // indigo
    Color32::from_rgb(0xfa, 0xcc, 0x15), // gold
    Color32::from_rgb(0x4a, 0xde, 0x80), // green
];

pub fn lane(i: usize) -> Color32 {
    LANES[i % LANES.len()]
}

pub fn alpha(c: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (a.clamp(0.0, 1.0) * 255.0) as u8)
}

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}
pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}
pub fn regular(size: f32) -> FontId {
    FontId::proportional(size)
}
pub fn mono(size: f32) -> FontId {
    FontId::monospace(size)
}

fn load_font(path: &str) -> Option<Vec<u8>> {
    std::fs::read(path).ok()
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let dir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into()) + "\\Fonts\\";
    let candidates: [(&str, &[&str]); 4] = [
        ("ui", &["segoeui.ttf"]),
        ("ui-semibold", &["seguisb.ttf", "segoeuib.ttf"]),
        ("ui-bold", &["segoeuib.ttf"]),
        ("ui-mono", &["CascadiaMono.ttf", "consola.ttf"]),
    ];
    let mut loaded = std::collections::HashSet::new();
    for (name, files) in candidates {
        for f in files {
            if let Some(bytes) = load_font(&format!("{dir}{f}")) {
                fonts
                    .font_data
                    .insert(name.to_string(), Arc::new(FontData::from_owned(bytes)));
                loaded.insert(name);
                break;
            }
        }
    }
    let defaults_prop = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let defaults_mono = fonts
        .families
        .get(&FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();

    let family = |first: &str, fallback: &[String]| -> Vec<String> {
        let mut v = Vec::new();
        if loaded.contains(first) {
            v.push(first.to_string());
        }
        if first != "ui" && loaded.contains("ui") {
            v.push("ui".into());
        }
        v.extend(fallback.iter().cloned());
        v
    };
    fonts
        .families
        .insert(FontFamily::Proportional, family("ui", &defaults_prop));
    fonts
        .families
        .insert(FontFamily::Monospace, family("ui-mono", &defaults_mono));
    fonts.families.insert(
        FontFamily::Name("semibold".into()),
        family("ui-semibold", &defaults_prop),
    );
    fonts.families.insert(
        FontFamily::Name("bold".into()),
        family("ui-bold", &defaults_prop),
    );
    ctx.set_fonts(fonts);
}

pub fn apply_style(ctx: &egui::Context) {
    ctx.style_mut(|style| {
        style.text_styles = [
            (TextStyle::Small, regular(11.0)),
            (TextStyle::Body, regular(13.5)),
            (TextStyle::Button, regular(13.0)),
            (TextStyle::Heading, semibold(18.0)),
            (TextStyle::Monospace, mono(12.5)),
        ]
        .into();
        let s = &mut style.spacing;
        s.item_spacing = Vec2::new(10.0, 8.0);
        s.button_padding = Vec2::new(12.0, 6.0);
        s.interact_size = Vec2::new(24.0, 26.0);
        s.menu_margin = egui::Margin::same(8);
        s.window_margin = egui::Margin::same(16);
        s.scroll = egui::style::ScrollStyle::floating();
        s.scroll.bar_width = 8.0;
        s.text_edit_width = 240.0;

        let v = &mut style.visuals;
        *v = egui::Visuals::dark();
        v.override_text_color = None;
        v.panel_fill = BG1;
        v.window_fill = BG2;
        v.window_stroke = Stroke::new(1.0, OUTLINE);
        v.extreme_bg_color = Color32::from_rgb(0x0b, 0x09, 0x12);
        v.faint_bg_color = BG2;
        v.code_bg_color = BG3;
        v.hyperlink_color = CYAN;
        v.warn_fg_color = WARNING;
        v.error_fg_color = REMOVED;
        v.window_corner_radius = CornerRadius::same(18);
        v.menu_corner_radius = CornerRadius::same(14);
        v.window_shadow = Shadow {
            offset: [0, 10],
            blur: 30,
            spread: 0,
            color: Color32::from_black_alpha(140),
        };
        v.popup_shadow = Shadow {
            offset: [0, 8],
            blur: 22,
            spread: 0,
            color: Color32::from_black_alpha(130),
        };
        v.selection.bg_fill = alpha(VIOLET, 0.45);
        v.selection.stroke = Stroke::new(1.0, TEXT);
        v.text_cursor.stroke = Stroke::new(2.0, CYAN);
        v.striped = false;
        v.slider_trailing_fill = true;

        let r = CornerRadius::same(11);
        v.widgets.noninteractive.bg_fill = BG2;
        v.widgets.noninteractive.weak_bg_fill = BG2;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER_SOFT);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_DIM);
        v.widgets.noninteractive.corner_radius = r;

        v.widgets.inactive.bg_fill = BG3;
        v.widgets.inactive.weak_bg_fill = BG3;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, OUTLINE);
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
        v.widgets.inactive.corner_radius = r;

        v.widgets.hovered.bg_fill = BG4;
        v.widgets.hovered.weak_bg_fill = BG4;
        v.widgets.hovered.bg_stroke = Stroke::new(1.5, alpha(VIOLET, 0.8));
        v.widgets.hovered.fg_stroke = Stroke::new(1.5, TEXT);
        v.widgets.hovered.corner_radius = r;
        v.widgets.hovered.expansion = 0.0;

        v.widgets.active.bg_fill = mix(BG4, VIOLET, 0.25);
        v.widgets.active.weak_bg_fill = mix(BG4, VIOLET, 0.25);
        v.widgets.active.bg_stroke = Stroke::new(1.5, PINK);
        v.widgets.active.fg_stroke = Stroke::new(1.5, Color32::WHITE);
        v.widgets.active.corner_radius = r;
        v.widgets.active.expansion = 0.0;

        v.widgets.open.bg_fill = BG4;
        v.widgets.open.weak_bg_fill = BG4;
        v.widgets.open.bg_stroke = Stroke::new(1.5, alpha(VIOLET, 0.8));
        v.widgets.open.fg_stroke = Stroke::new(1.0, TEXT);
        v.widgets.open.corner_radius = r;
    });
}
