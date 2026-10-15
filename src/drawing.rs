//! Chart drawing tools: the left tool rail, and painting drawings anchored in
//! (candle index, price) space so they follow pan and zoom.

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Id, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui,
    Vec2, pos2, vec2,
};

use crate::{format::price, theme};

pub const RAIL_WIDTH: f32 = 40.0;
const BUTTON: f32 = 30.0;
const LINE: Color32 = theme::TEXT;
const FIB_LEVELS: [f64; 7] = [0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0];

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    #[default]
    Cursor,
    TrendLine,
    Ray,
    ExtendedLine,
    Arrow,
    HorizontalLine,
    HorizontalRay,
    VerticalLine,
    Rectangle,
    Channel,
    PriceRange,
    Fibonacci,
}

impl Tool {
    const ALL: [Tool; 12] = [
        Tool::Cursor,
        Tool::TrendLine,
        Tool::Ray,
        Tool::ExtendedLine,
        Tool::Arrow,
        Tool::HorizontalLine,
        Tool::HorizontalRay,
        Tool::VerticalLine,
        Tool::Rectangle,
        Tool::Channel,
        Tool::PriceRange,
        Tool::Fibonacci,
    ];

    fn name(self) -> &'static str {
        match self {
            Tool::Cursor => "Cursor",
            Tool::TrendLine => "Trend line",
            Tool::Ray => "Ray",
            Tool::ExtendedLine => "Extended line",
            Tool::Arrow => "Arrow",
            Tool::HorizontalLine => "Horizontal line",
            Tool::HorizontalRay => "Horizontal ray",
            Tool::VerticalLine => "Vertical line",
            Tool::Rectangle => "Rectangle",
            Tool::Channel => "Parallel channel",
            Tool::PriceRange => "Price range",
            Tool::Fibonacci => "Fib retracement",
        }
    }

    /// Clicks needed to place one drawing.
    pub fn points(self) -> usize {
        match self {
            Tool::Cursor => 0,
            Tool::HorizontalLine | Tool::HorizontalRay | Tool::VerticalLine => 1,
            Tool::Channel => 3,
            _ => 2,
        }
    }
}

/// A chart position: fractional candle index and price.
#[derive(Clone, Copy)]
pub struct Anchor {
    pub index: f32,
    pub price: f64,
}

pub struct Drawing {
    pub tool: Tool,
    pub anchors: Vec<Anchor>,
}

/// In-progress and finished drawings for one chart.
#[derive(Default)]
pub struct Drawings {
    pub tool: Tool,
    pub pending: Vec<Anchor>,
    pub done: Vec<Drawing>,
}

impl Drawings {
    /// Adds a click at `at`; commits the drawing once the tool has all its points.
    pub fn click(&mut self, at: Anchor) {
        if self.tool == Tool::Cursor {
            return;
        }
        self.pending.push(at);
        if self.pending.len() >= self.tool.points() {
            self.done.push(Drawing { tool: self.tool, anchors: std::mem::take(&mut self.pending) });
            self.tool = Tool::Cursor;
        }
    }

    /// Esc cancels the drawing in progress; Delete/Backspace removes the last drawing.
    pub fn handle_keys(&mut self, ui: &Ui) {
        let (esc, delete) = ui.input(|i| {
            (i.key_pressed(egui::Key::Escape), i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace))
        });
        if esc {
            self.pending.clear();
            self.tool = Tool::Cursor;
        }
        if delete {
            self.done.pop();
        }
    }

    pub fn rail(&mut self, ui: &mut Ui, rect: Rect) {
        let p = ui.painter();
        p.rect_filled(rect, CornerRadius::ZERO, theme::SURFACE);
        p.vline(rect.right() - 0.5, rect.y_range(), Stroke::new(1.0, theme::BORDER));

        // Trash is pinned to the bottom; tools fill from the top while they fit.
        let trash = Rect::from_center_size(pos2(rect.center().x, rect.bottom() - 6.0 - BUTTON / 2.0), Vec2::splat(BUTTON));
        let mut y = rect.top() + 6.0;
        for tool in Tool::ALL {
            let r = Rect::from_min_size(pos2(rect.center().x - BUTTON / 2.0, y), Vec2::splat(BUTTON));
            if r.bottom() > trash.top() - 10.0 {
                break;
            }
            y += BUTTON + 2.0;
            let active = self.tool == tool;
            if rail_button(ui, r, Id::new(("draw_tool", tool.name())), tool.name(), active, |p, r, c| icon(p, r, tool, c)) {
                self.pending.clear();
                self.tool = if active { Tool::Cursor } else { tool };
            }
        }
        let label = "Remove all drawings";
        if rail_button(ui, trash, Id::new("draw_trash"), label, false, |p, r, c| trash_icon(p, r, c)) {
            self.done.clear();
            self.pending.clear();
            self.tool = Tool::Cursor;
        }
    }
}

fn rail_button(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    tooltip: &str,
    active: bool,
    paint: impl FnOnce(&Painter, Rect, Color32),
) -> bool {
    let response = ui.interact(rect, id, Sense::click()).on_hover_text(tooltip);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let p = ui.painter();
    if active {
        p.rect_filled(rect, CornerRadius::same(5), theme::SURFACE_RAISED);
    } else if response.hovered() {
        p.rect_filled(rect, CornerRadius::same(5), theme::HOVER);
    }
    let color = if active || response.hovered() { theme::TEXT } else { theme::TEXT_MUTED };
    paint(p, rect.shrink(8.0), color);
    response.clicked()
}

fn icon(p: &Painter, r: Rect, tool: Tool, c: Color32) {
    let s = Stroke::new(1.3, c);
    let (bl, tr) = (r.left_bottom(), r.right_top());
    match tool {
        Tool::Cursor => {
            p.hline(r.x_range(), r.center().y, s);
            p.vline(r.center().x, r.y_range(), s);
        }
        Tool::TrendLine => {
            p.line_segment([bl, tr], s);
            p.circle_filled(bl, 2.0, c);
            p.circle_filled(tr, 2.0, c);
        }
        Tool::Ray => {
            p.line_segment([bl, tr], s);
            p.circle_filled(bl, 2.0, c);
        }
        Tool::ExtendedLine => {
            p.line_segment([bl, tr], s);
        }
        Tool::Arrow => {
            p.line_segment([bl, tr], s);
            p.line_segment([tr, tr + vec2(-6.0, 0.0)], s);
            p.line_segment([tr, tr + vec2(0.0, 6.0)], s);
        }
        Tool::HorizontalLine => {
            p.hline(r.x_range(), r.center().y, s);
        }
        Tool::HorizontalRay => {
            p.hline(r.x_range(), r.center().y, s);
            p.circle_filled(r.left_center(), 2.0, c);
        }
        Tool::VerticalLine => {
            p.vline(r.center().x, r.y_range(), s);
        }
        Tool::Rectangle => {
            p.rect_stroke(r.shrink2(vec2(0.0, 2.0)), CornerRadius::same(1), s, egui::StrokeKind::Inside);
        }
        Tool::Channel => {
            p.line_segment([bl + vec2(0.0, -5.0), tr + vec2(-5.0, 0.0)], s);
            p.line_segment([bl + vec2(5.0, 0.0), tr + vec2(0.0, 5.0)], s);
        }
        Tool::PriceRange => {
            p.vline(r.center().x, r.y_range(), s);
            p.hline(r.shrink2(vec2(3.0, 0.0)).x_range(), r.top(), s);
            p.hline(r.shrink2(vec2(3.0, 0.0)).x_range(), r.bottom(), s);
        }
        Tool::Fibonacci => {
            for (k, inset) in [0.0, 2.0, 4.0, 6.0].into_iter().enumerate() {
                let y = r.top() + 2.0 + k as f32 * 4.0;
                p.hline(r.left() + inset..=r.right(), y, s);
            }
        }
    }
}

fn trash_icon(p: &Painter, r: Rect, c: Color32) {
    let s = Stroke::new(1.3, c);
    let body = Rect::from_min_max(pos2(r.left() + 2.5, r.top() + 4.0), pos2(r.right() - 2.5, r.bottom()));
    p.rect_stroke(body, CornerRadius::same(2), s, egui::StrokeKind::Inside);
    p.hline(r.x_range(), r.top() + 3.0, s);
    p.hline(r.center().x - 2.5..=r.center().x + 2.5, r.top(), s);
}

/// Paints one drawing. `pts` are the anchors in screen space; `clip` bounds extended lines.
pub fn paint(p: &Painter, tool: Tool, anchors: &[Anchor], pts: &[Pos2], clip: Rect) {
    let s = Stroke::new(1.5, LINE);
    let fill = LINE.gamma_multiply(0.08);
    let far = clip.width() + clip.height();
    let label_font = FontId::proportional(11.5);
    let handle = |p: &Painter, at: Pos2| {
        p.circle_filled(at, 3.5, theme::BG);
        p.circle_stroke(at, 3.5, Stroke::new(1.2, LINE));
    };

    match (tool, pts) {
        (Tool::HorizontalLine, [a, ..]) => {
            p.hline(clip.x_range(), a.y, s);
        }
        (Tool::HorizontalRay, [a, ..]) => {
            p.hline(a.x..=clip.right(), a.y, s);
            handle(p, *a);
        }
        (Tool::VerticalLine, [a, ..]) => {
            p.vline(a.x, clip.y_range(), s);
        }
        (Tool::Rectangle, [a, b, ..]) => {
            p.rect(Rect::from_two_pos(*a, *b), CornerRadius::ZERO, fill, s, egui::StrokeKind::Middle);
        }
        (Tool::Channel, [a, b, rest @ ..]) => {
            // The third click sets the parallel line's vertical offset at its x.
            let offset = rest.first().map_or(Vec2::ZERO, |c| {
                let t = if (b.x - a.x).abs() > f32::EPSILON { (c.x - a.x) / (b.x - a.x) } else { 0.0 };
                vec2(0.0, c.y - (a.y + (b.y - a.y) * t))
            });
            let (a2, b2) = (*a + offset, *b + offset);
            p.add(Shape::convex_polygon(vec![*a, *b, b2, a2], fill, Stroke::NONE));
            p.line_segment([*a, *b], s);
            p.line_segment([a2, b2], s);
            p.extend(Shape::dashed_line(&[a.lerp(a2, 0.5), b.lerp(b2, 0.5)], Stroke::new(1.0, LINE.gamma_multiply(0.5)), 4.0, 4.0));
        }
        (Tool::PriceRange, [a, b, ..]) => {
            let r = Rect::from_two_pos(*a, *b);
            let up = anchors[1].price >= anchors[0].price;
            let color = if up { theme::CANDLE_UP } else { theme::CANDLE_DOWN };
            p.rect(r, CornerRadius::ZERO, color.gamma_multiply(0.15), Stroke::new(1.0, color), egui::StrokeKind::Middle);
            let diff = anchors[1].price - anchors[0].price;
            let pct = diff / anchors[0].price * 100.0;
            let bars = (anchors[1].index - anchors[0].index).round().abs();
            let text = format!("{}{} ({}{:.2}%) · {bars:.0} bars", sign(diff), price(diff.abs()), sign(pct), pct.abs());
            let g = p.layout_no_wrap(text, label_font, Color32::WHITE);
            let at = pos2(r.center().x, if up { r.top() - 6.0 } else { r.bottom() + 6.0 });
            let tag = Rect::from_center_size(at + vec2(0.0, if up { -g.size().y / 2.0 - 3.0 } else { g.size().y / 2.0 + 3.0 }), g.size() + vec2(12.0, 6.0));
            p.rect_filled(tag, CornerRadius::same(3), color);
            p.galley(tag.center() - g.size() / 2.0, g, Color32::WHITE);
        }
        (Tool::Fibonacci, [a, b, ..]) => {
            let (x0, x1) = (a.x.min(b.x), clip.right());
            for level in FIB_LEVELS {
                let y = b.y + (a.y - b.y) * level as f32;
                let value = anchors[1].price + (anchors[0].price - anchors[1].price) * level;
                p.hline(x0..=x1, y, Stroke::new(1.0, LINE.gamma_multiply(0.7)));
                p.text(pos2(x0 + 4.0, y - 2.0), Align2::LEFT_BOTTOM, format!("{level} ({})", price(value)), label_font.clone(), theme::TEXT_MUTED);
            }
            p.extend(Shape::dashed_line(&[*a, *b], Stroke::new(1.0, LINE.gamma_multiply(0.5)), 4.0, 4.0));
        }
        (Tool::TrendLine | Tool::Arrow, [a, b, ..]) => {
            p.line_segment([*a, *b], s);
            if tool == Tool::Arrow && a.distance(*b) > 1.0 {
                let dir = (*b - *a).normalized();
                for angle in [0.45_f32, -0.45] {
                    let back = egui::emath::Rot2::from_angle(angle) * -dir * 12.0;
                    p.line_segment([*b, *b + back], s);
                }
            }
        }
        (Tool::Ray, [a, b, ..]) if a.distance(*b) > 0.5 => {
            p.line_segment([*a, *a + (*b - *a).normalized() * far], s);
        }
        (Tool::ExtendedLine, [a, b, ..]) if a.distance(*b) > 0.5 => {
            let d = (*b - *a).normalized() * far;
            p.line_segment([*a - d, *a + d], s);
        }
        _ => {}
    }

    if !matches!(tool, Tool::HorizontalLine | Tool::VerticalLine) {
        for at in pts {
            handle(p, *at);
        }
    }
}

fn sign(v: f64) -> &'static str {
    if v >= 0.0 { "+" } else { "-" }
}
