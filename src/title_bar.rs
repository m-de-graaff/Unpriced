use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Id, Pos2, Rect, Sense, Stroke, Ui, ViewportCommand,
    pos2, vec2,
};

use crate::theme;

pub const HEIGHT: f32 = 40.0;
const BUTTON_WIDTH: f32 = 46.0;
const ICON: f32 = 10.0;

#[derive(Clone, Copy)]
enum Button {
    Minimize,
    Maximize,
    Close,
}

pub fn show(ui: &mut Ui, title: &str) {
    let ctx = ui.ctx().clone();
    let bar = ui.max_rect();
    let painter = ui.painter();

    painter.rect_filled(bar, CornerRadius::ZERO, theme::SURFACE);
    painter.hline(bar.x_range(), bar.bottom() - 0.5, Stroke::new(1.0, theme::BORDER));

    // Draggable background. Buttons are added afterwards, so they win hit-testing.
    let drag = ui.interact(bar, Id::new("title_bar_drag"), Sense::click_and_drag());
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    if drag.double_clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    } else if drag.drag_started_by(egui::PointerButton::Primary) {
        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    }

    // Logo mark + title.
    let logo = Rect::from_center_size(pos2(bar.left() + 22.0, bar.center().y), vec2(14.0, 14.0));
    painter.rect_filled(logo, CornerRadius::same(4), theme::TEXT);
    painter.rect_filled(logo.shrink(4.0), CornerRadius::same(1), theme::SURFACE);
    painter.text(
        pos2(logo.right() + 10.0, bar.center().y),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(13.0),
        theme::TEXT,
    );

    let mut x = bar.right() - BUTTON_WIDTH;
    for button in [Button::Close, Button::Maximize, Button::Minimize] {
        let rect = Rect::from_min_size(pos2(x, bar.top()), vec2(BUTTON_WIDTH, HEIGHT - 1.0));
        if window_button(ui, rect, button, maximized).clicked() {
            ctx.send_viewport_cmd(match button {
                Button::Minimize => ViewportCommand::Minimized(true),
                Button::Maximize => ViewportCommand::Maximized(!maximized),
                Button::Close => ViewportCommand::Close,
            });
        }
        x -= BUTTON_WIDTH;
    }
}

fn window_button(ui: &mut Ui, rect: Rect, button: Button, maximized: bool) -> egui::Response {
    let id = Id::new(("title_bar_button", button as u8));
    let response = ui.interact(rect, id, Sense::click());
    let t = ui.ctx().animate_bool_with_time(id, response.hovered(), 0.12);

    let (hover_fill, hover_fg) = match button {
        Button::Close => (theme::CLOSE_HOVER, Color32::WHITE),
        _ => (theme::HOVER, theme::TEXT),
    };
    let painter = ui.painter();
    if t > 0.0 {
        painter.rect_filled(rect, CornerRadius::ZERO, hover_fill.gamma_multiply(t));
    }
    let fg = lerp_color(theme::TEXT_MUTED, hover_fg, t);
    let stroke = Stroke::new(1.0, fg);

    let c = snap(rect.center());
    let h = ICON / 2.0;
    match button {
        Button::Minimize => {
            painter.hline((c.x - h)..=(c.x + h), c.y, stroke);
        }
        Button::Maximize if maximized => {
            // Restore: two offset squares.
            let back = Rect::from_min_size(pos2(c.x - h + 2.0, c.y - h), vec2(ICON - 2.0, ICON - 2.0));
            let front = back.translate(vec2(-2.0, 2.0));
            painter.line_segment([pos2(back.left(), back.top()), back.right_top()], stroke);
            painter.line_segment([back.right_top(), pos2(back.right(), back.bottom())], stroke);
            painter.rect_stroke(front, CornerRadius::same(1), stroke, egui::StrokeKind::Inside);
        }
        Button::Maximize => {
            let r = Rect::from_center_size(c, vec2(ICON, ICON));
            painter.rect_stroke(r, CornerRadius::same(1), stroke, egui::StrokeKind::Inside);
        }
        Button::Close => {
            painter.line_segment([pos2(c.x - h, c.y - h), pos2(c.x + h, c.y + h)], stroke);
            painter.line_segment([pos2(c.x + h, c.y - h), pos2(c.x - h, c.y + h)], stroke);
        }
    }
    response
}

fn snap(p: Pos2) -> Pos2 {
    pos2(p.x.round() + 0.5, p.y.round() + 0.5)
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}
