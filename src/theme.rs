use eframe::egui::{self, Color32, CornerRadius, Stroke, Theme, Visuals};

// Near-black surfaces with an off-white gray foreground.
pub const BG: Color32 = Color32::from_rgb(0x13, 0x13, 0x15);
pub const SURFACE: Color32 = Color32::from_rgb(0x18, 0x18, 0x1b);
pub const SURFACE_RAISED: Color32 = Color32::from_rgb(0x1f, 0x1f, 0x23);
pub const BORDER: Color32 = Color32::from_rgb(0x2a, 0x2a, 0x2f);
pub const TEXT: Color32 = Color32::from_rgb(0xe6, 0xe4, 0xdf);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x8c, 0x8a, 0x86);
pub const HOVER: Color32 = Color32::from_rgba_premultiplied(0x0d, 0x0d, 0x0d, 0x0d);
pub const CLOSE_HOVER: Color32 = Color32::from_rgb(0xc4, 0x2b, 0x1c);

// Data colors, stepped for the dark surface.
pub const ACCENT: Color32 = Color32::from_rgb(0x39, 0x87, 0xe5);
pub const GAIN: Color32 = Color32::from_rgb(0x0c, 0xa3, 0x0c);
pub const LOSS: Color32 = Color32::from_rgb(0xe6, 0x67, 0x67);
pub const GRID: Color32 = Color32::from_rgb(0x24, 0x24, 0x28);
/// Categorical series colors, assigned in this fixed order.
pub const SERIES: [Color32; 5] = [
    Color32::from_rgb(0x39, 0x87, 0xe5), // blue
    Color32::from_rgb(0xd9, 0x59, 0x26), // orange
    Color32::from_rgb(0x19, 0x9e, 0x70), // aqua
    Color32::from_rgb(0xc9, 0x85, 0x00), // yellow
    Color32::from_rgb(0xd5, 0x51, 0x81), // magenta
];
pub const CANDLE_UP: Color32 = Color32::from_rgb(0x22, 0xab, 0x94);
pub const CANDLE_DOWN: Color32 = Color32::from_rgb(0xf2, 0x36, 0x45);

pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(Theme::Dark);

    let mut v = Visuals::dark();
    v.override_text_color = Some(TEXT);
    v.panel_fill = BG;
    v.window_fill = SURFACE_RAISED;
    v.extreme_bg_color = SURFACE;
    v.faint_bg_color = SURFACE;
    v.window_stroke = Stroke::new(1.0, BORDER);
    v.window_corner_radius = CornerRadius::same(10);
    v.selection.bg_fill = Color32::from_rgb(0x3a, 0x39, 0x36);
    v.selection.stroke = Stroke::new(1.0, TEXT);
    v.hyperlink_color = TEXT;

    let r = CornerRadius::same(6);
    let w = &mut v.widgets;
    w.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_MUTED);
    for (state, fill) in [
        (&mut w.inactive, SURFACE_RAISED),
        (&mut w.hovered, Color32::from_rgb(0x28, 0x28, 0x2d)),
        (&mut w.active, Color32::from_rgb(0x30, 0x30, 0x36)),
        (&mut w.open, SURFACE_RAISED),
    ] {
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
        state.bg_stroke = Stroke::new(1.0, BORDER);
        state.fg_stroke = Stroke::new(1.0, TEXT);
        state.corner_radius = r;
        state.expansion = 0.0;
    }

    ctx.set_visuals_of(Theme::Dark, v);
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 8.0);
        s.spacing.button_padding = egui::vec2(12.0, 6.0);
    });
}
