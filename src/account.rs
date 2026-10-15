use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Frame, Id, Margin, Popup, PopupCloseBehavior,
    Rect, RectAlign, Sense, Stroke, Ui, pos2, vec2,
};

use crate::theme;

pub const BUTTON_WIDTH: f32 = 40.0;
const AVATAR: f32 = 24.0;
const MENU_WIDTH: f32 = 240.0;
const ROW_HEIGHT: f32 = 30.0;

/// A page reachable from the account menu.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Settings,
    Markets,
}

impl Page {
    pub fn title(self) -> &'static str {
        match self {
            Page::Settings => "Settings",
            Page::Markets => "Markets",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Broker {
    Alpaca,
    Etoro,
    InteractiveBrokers,
}

impl Broker {
    pub const ALL: [Broker; 3] = [Broker::Alpaca, Broker::Etoro, Broker::InteractiveBrokers];

    pub fn label(self) -> &'static str {
        match self {
            Broker::Alpaca => "Alpaca",
            Broker::Etoro => "eToro",
            Broker::InteractiveBrokers => "Interactive Brokers",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Broker::Alpaca => "US stocks, ETFs, options and crypto",
            Broker::Etoro => "Stocks, ETFs, crypto and CFDs",
            Broker::InteractiveBrokers => "Global stocks, options, futures and forex",
        }
    }
}

/// Avatar button that opens the account dropdown. Returns the page picked, if any.
pub fn show(ui: &mut Ui, rect: Rect) -> Option<Page> {
    let id = Id::new("account_button");
    let response = ui.interact(rect, id, Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    let popup = Popup::from_toggle_button_response(&response)
        .align(RectAlign::BOTTOM_END)
        .gap(4.0)
        .width(MENU_WIDTH)
        .close_behavior(PopupCloseBehavior::CloseOnClick)
        .frame(
            Frame::NONE
                .fill(theme::SURFACE_RAISED)
                .stroke(Stroke::new(1.0, theme::BORDER))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(6)),
        );
    let open = popup.is_open();

    let t = ui.ctx().animate_bool_with_time(id, response.hovered() || open, 0.12);
    let painter = ui.painter();
    if t > 0.0 {
        painter.rect_filled(rect, CornerRadius::ZERO, theme::HOVER.gamma_multiply(t));
    }
    let avatar = Rect::from_center_size(rect.center(), vec2(AVATAR, AVATAR));
    painter.circle_filled(avatar.center(), AVATAR / 2.0, theme::BORDER);
    painter.circle_stroke(avatar.center(), AVATAR / 2.0, Stroke::new(1.0, theme::TEXT_MUTED.gamma_multiply(0.4)));
    person_glyph(painter, avatar, theme::TEXT);

    popup.show(menu).and_then(|r| r.inner)
}

fn menu(ui: &mut Ui) -> Option<Page> {
    ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
    let mut picked = None;

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
            ui.label(egui::RichText::new("Account").size(13.0).strong());
            ui.label(egui::RichText::new("Not signed in").size(12.0).color(theme::TEXT_MUTED));
        });
    });
    ui.add_space(6.0);
    separator(ui);

    for page in [Page::Settings, Page::Markets] {
        if row(ui, page.title()).clicked() {
            picked = Some(page);
        }
    }

    picked
}

/// A full-width clickable menu row.
fn row(ui: &mut Ui, label: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let t = ui.ctx().animate_bool_with_time(response.id, response.hovered(), 0.08);
    let painter = ui.painter();
    if t > 0.0 {
        painter.rect_filled(rect, CornerRadius::same(5), hover_fill().gamma_multiply(t));
    }
    let font = FontId::proportional(13.0);
    painter.text(pos2(rect.left() + 8.0, rect.center().y), Align2::LEFT_CENTER, label, font, theme::TEXT);
    response
}

fn separator(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 9.0), Sense::hover());
    ui.painter()
        .hline(rect.x_range(), rect.center().y.round() + 0.5, Stroke::new(1.0, theme::BORDER));
}

fn hover_fill() -> Color32 {
    Color32::from_rgb(0x28, 0x28, 0x2d)
}

/// Generic head-and-shoulders silhouette, clipped to the avatar circle.
fn person_glyph(painter: &egui::Painter, avatar: Rect, color: Color32) {
    let c = avatar.center();
    let s = avatar.width();
    painter.circle_filled(pos2(c.x, c.y - s * 0.12), s * 0.17, color);
    let shoulders = Rect::from_center_size(pos2(c.x, c.y + s * 0.32), vec2(s * 0.56, s * 0.36));
    painter
        .with_clip_rect(avatar.shrink(1.5))
        .rect_filled(shoulders, CornerRadius::same((s * 0.18) as u8), color);
}
