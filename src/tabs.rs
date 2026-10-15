use eframe::egui::{self, Align2, CornerRadius, FontId, Id, Rect, Sense, Stroke, Ui, pos2, vec2};

use crate::{account, theme};

pub const HEIGHT: f32 = 40.0;
const PADDING_X: f32 = 14.0;
const INDICATOR_HEIGHT: f32 = 2.0;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Dashboard,
    Research,
    Indicator,
    Strategy,
    Backtest,
}

impl Tab {
    pub const ALL: [Tab; 5] = [
        Tab::Dashboard,
        Tab::Research,
        Tab::Indicator,
        Tab::Strategy,
        Tab::Backtest,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::Research => "Research",
            Tab::Indicator => "Indicator",
            Tab::Strategy => "Strategy",
            Tab::Backtest => "Backtest",
        }
    }
}

/// What the user did in the tab bar this frame.
pub enum Action {
    TabClicked,
    OpenPage(account::Page),
}

/// Draws the tab bar, with the account menu at its right end.
/// `highlight` is false while a non-tab page (e.g. Markets) is showing, so no tab looks active.
pub fn show(ui: &mut Ui, active: &mut Tab, highlight: bool) -> Option<Action> {
    let bar = ui.max_rect();
    ui.painter().rect_filled(bar, CornerRadius::ZERO, theme::SURFACE);
    ui.painter()
        .hline(bar.x_range(), bar.bottom() - 0.5, Stroke::new(1.0, theme::BORDER));

    let font = FontId::proportional(13.0);
    let mut x = bar.left() + 12.0;
    let mut active_rect = None;
    let mut action = None;

    for tab in Tab::ALL {
        let galley = ui.painter().layout_no_wrap(tab.label().to_owned(), font.clone(), theme::TEXT);
        let rect = Rect::from_min_size(
            pos2(x, bar.top()),
            vec2(galley.size().x + PADDING_X * 2.0, HEIGHT - 1.0),
        );
        x = rect.right();

        let id = Id::new(("tab", tab.label()));
        let response = ui.interact(rect, id, Sense::click());
        if response.clicked() {
            *active = tab;
            action = Some(Action::TabClicked);
        }
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        let is_active = highlight && *active == tab;
        let hover = ui.ctx().animate_bool_with_time(id, response.hovered(), 0.12);
        if hover > 0.0 && !is_active {
            let pill = rect.shrink2(vec2(4.0, 7.0));
            ui.painter()
                .rect_filled(pill, CornerRadius::same(6), theme::HOVER.gamma_multiply(hover));
        }

        let color = if is_active { theme::TEXT } else { theme::TEXT_MUTED };
        ui.painter()
            .text(rect.center(), Align2::CENTER_CENTER, tab.label(), font.clone(), color);

        if is_active {
            active_rect = Some(rect);
        }
    }

    // Underline that slides to the active tab.
    if let Some(target) = active_rect {
        let ctx = ui.ctx();
        let left = ctx.animate_value_with_time(Id::new("tab_indicator_l"), target.left() + PADDING_X, 0.18);
        let right = ctx.animate_value_with_time(Id::new("tab_indicator_r"), target.right() - PADDING_X, 0.18);
        let indicator = Rect::from_min_max(
            pos2(left, bar.bottom() - INDICATOR_HEIGHT - 1.0),
            pos2(right, bar.bottom() - 1.0),
        );
        ui.painter().rect_filled(indicator, CornerRadius::same(1), theme::TEXT);
    }

    let account = Rect::from_min_size(
        pos2(bar.right() - account::BUTTON_WIDTH - 8.0, bar.top()),
        vec2(account::BUTTON_WIDTH, HEIGHT - 1.0),
    );
    if let Some(page) = account::show(ui, account) {
        action = Some(Action::OpenPage(page));
    }

    action
}
