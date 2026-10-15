#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod account;
mod bench;
mod dashboard;
mod drawing;
mod format;
mod indicator;
mod tabs;
mod theme;
mod title_bar;

use eframe::egui::{
    self, CornerRadius, CursorIcon, Frame, Id, Margin, ResizeDirection, Sense, Stroke, Ui,
    ViewportCommand, vec2,
};

const APP_NAME: &str = "Unpriced";
const RESIZE_EDGE: f32 = 5.0;
/// eframe's storage key for the saved window geometry.
const WINDOW_KEY: &str = "window";

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(APP_NAME)
            .with_app_id(APP_NAME)
            .with_decorations(false)
            .with_resizable(true)
            .with_inner_size([1000.0, 660.0])
            .with_min_inner_size([520.0, 360.0]),
        ..Default::default()
    };
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
            let last_good_window = cc
                .storage
                .and_then(|s| s.get_string(WINDOW_KEY))
                .filter(|w| !is_minimized_geometry(w));
            Ok(Box::new(App {
                bench: bench::Bench::from_env(),
                ctx: Some(cc.egui_ctx.clone()),
                last_good_window,
                ..Default::default()
            }))
        }),
    )
}

#[derive(Default)]
struct App {
    tab: tabs::Tab,
    /// A page opened from the account menu; shown instead of the active tab.
    page: Option<account::Page>,
    dashboard: dashboard::Dashboard,
    indicator: indicator::IndicatorPage,
    bench: Option<bench::Bench>,
    frame_nr: usize,
    ctx: Option<egui::Context>,
    /// Window geometry from the last save made while the window was not minimized.
    last_good_window: Option<String>,
    /// Geometry seen on the most recent frame while not minimized.
    geometry: Option<Geometry>,
}

#[derive(Clone, Copy)]
struct Geometry {
    inner_px: egui::Pos2,
    outer_px: egui::Pos2,
    inner_size: egui::Vec2,
    maximized: bool,
}

impl Geometry {
    fn capture(ctx: &egui::Context) -> Option<Self> {
        ctx.input(|i| {
            let v = i.viewport();
            if v.minimized == Some(true) {
                return None;
            }
            let ppp = v.native_pixels_per_point?;
            let (inner, outer) = (v.inner_rect?, v.outer_rect?);
            let g = Self {
                inner_px: (inner.min.to_vec2() * ppp).to_pos2(),
                outer_px: (outer.min.to_vec2() * ppp).to_pos2(),
                inner_size: inner.size(),
                maximized: v.maximized == Some(true),
            };
            (g.outer_px.x > -30_000.0).then_some(g)
        })
    }

    /// Same RON shape eframe writes for `egui_winit::WindowSettings`. If it ever drifts,
    /// parsing fails and eframe falls back to the default window, which is safe.
    fn to_ron(self) -> String {
        format!(
            "(inner_position_pixels:Some((x:{:.1},y:{:.1})),outer_position_pixels:Some((x:{:.1},y:{:.1})),fullscreen:false,maximized:{},inner_size_points:Some((x:{:.1},y:{:.1})))",
            self.inner_px.x,
            self.inner_px.y,
            self.outer_px.x,
            self.outer_px.y,
            self.maximized,
            self.inner_size.x,
            self.inner_size.y,
        )
    }
}

/// Windows parks minimized windows at (-32000, -32000) with a tiny size.
fn is_minimized_geometry(window: &str) -> bool {
    window.contains("-32000")
}

impl eframe::App for App {
    /// eframe has already written the window geometry by the time this runs. If the
    /// window is minimized that geometry is bogus, so put back the last good one.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let current = storage.get_string(WINDOW_KEY);
        let minimized = self.ctx.as_ref().is_some_and(|c| c.input(|i| i.viewport().minimized == Some(true)))
            || current.as_deref().is_some_and(is_minimized_geometry);
        if minimized {
            // An empty value fails to parse, so eframe falls back to the default size.
            let good = self.geometry.map(Geometry::to_ron).or_else(|| self.last_good_window.clone());
            storage.set_string(WINDOW_KEY, good.unwrap_or_default());
        } else {
            self.last_good_window = current;
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        theme::BG.to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        if let Some(bench) = &mut self.bench {
            let i = bench.tick(ui.ctx(), frame, self.frame_nr);
            self.tab = tabs::Tab::ALL[i % tabs::Tab::ALL.len()];
        }
        self.frame_nr += 1;
        if let Some(g) = Geometry::capture(ui.ctx()) {
            self.geometry = Some(g);
        }

        egui::Panel::top("title_bar")
            .exact_size(title_bar::HEIGHT)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::NONE)
            .show(ui, |ui| title_bar::show(ui, APP_NAME));

        egui::Panel::top("tab_bar")
            .exact_size(tabs::HEIGHT)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::NONE)
            .show(ui, |ui| match tabs::show(ui, &mut self.tab, self.page.is_none()) {
                Some(tabs::Action::TabClicked) => self.page = None,
                Some(tabs::Action::OpenPage(page)) => self.page = Some(page),
                None => {}
            });

        // The chart page runs edge to edge; everything else gets page padding.
        let full_bleed = self.page.is_none() && self.tab == tabs::Tab::Indicator;
        let margin = if full_bleed { 0 } else { 32 };
        egui::CentralPanel::no_frame()
            .frame(Frame::NONE.fill(theme::BG).inner_margin(Margin::same(margin)))
            .show(ui, |ui| match self.page {
                Some(page) => account_page(ui, page),
                None if self.tab == tabs::Tab::Dashboard => self.dashboard.show(ui),
                None if self.tab == tabs::Tab::Indicator => self.indicator.show(ui),
                None => content(ui, self.tab),
            });

        let maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
        if !maximized {
            let screen = ui.ctx().content_rect();
            ui.painter().rect_stroke(
                screen,
                CornerRadius::ZERO,
                Stroke::new(1.0, theme::BORDER),
                egui::StrokeKind::Inside,
            );
            resize_handles(ui);
        }
    }
}

fn content(ui: &mut Ui, tab: tabs::Tab) {
    ui.label(egui::RichText::new(tab.label()).size(26.0).strong());
    ui.label(egui::RichText::new("A clean slate. Build something here.").color(theme::TEXT_MUTED));
    ui.add_space(16.0);
    Frame::NONE
        .fill(theme::SURFACE)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(20))
        .show(ui, |ui| {
            ui.set_max_width(420.0);
            ui.label(egui::RichText::new("Getting started").strong());
            ui.label(
                egui::RichText::new("Edit src/main.rs to change this view.").color(theme::TEXT_MUTED),
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let _ = ui.button("Primary action");
                let _ = ui.button("Secondary");
            });
        });
}

fn account_page(ui: &mut Ui, page: account::Page) {
    ui.label(egui::RichText::new(page.title()).size(26.0).strong());
    match page {
        account::Page::Settings => {
            ui.label(
                egui::RichText::new("Application preferences will live here.").color(theme::TEXT_MUTED),
            );
        }
        account::Page::Markets => markets_page(ui),
    }
}

fn markets_page(ui: &mut Ui) {
    ui.label(egui::RichText::new("Connect a broker to trade and pull account data.").color(theme::TEXT_MUTED));
    ui.add_space(16.0);
    for broker in account::Broker::ALL {
        Frame::NONE
            .fill(theme::SURFACE)
            .stroke(Stroke::new(1.0, theme::BORDER))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(16))
            .show(ui, |ui| {
                ui.set_width(ui.available_width().min(560.0));
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.label(egui::RichText::new(broker.label()).strong());
                        ui.label(egui::RichText::new(broker.description()).color(theme::TEXT_MUTED));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Broker authentication isn't implemented yet.
                        ui.add_enabled(false, egui::Button::new("Connect"));
                        ui.label(egui::RichText::new("Not connected").color(theme::TEXT_MUTED));
                    });
                });
            });
    }
}

/// Undecorated windows need their own resize grips along the edges.
fn resize_handles(ui: &mut Ui) {
    let r = ui.ctx().content_rect();
    let e = RESIZE_EDGE;
    let handles = [
        (egui::Rect::from_min_size(r.left_top(), vec2(e * 2.0, e * 2.0)), ResizeDirection::NorthWest),
        (egui::Rect::from_min_size(r.right_top() - vec2(e * 2.0, 0.0), vec2(e * 2.0, e * 2.0)), ResizeDirection::NorthEast),
        (egui::Rect::from_min_size(r.left_bottom() - vec2(0.0, e * 2.0), vec2(e * 2.0, e * 2.0)), ResizeDirection::SouthWest),
        (egui::Rect::from_min_size(r.right_bottom() - vec2(e * 2.0, e * 2.0), vec2(e * 2.0, e * 2.0)), ResizeDirection::SouthEast),
        (egui::Rect::from_min_max(r.left_top(), egui::pos2(r.right(), r.top() + e)), ResizeDirection::North),
        (egui::Rect::from_min_max(egui::pos2(r.left(), r.bottom() - e), r.right_bottom()), ResizeDirection::South),
        (egui::Rect::from_min_max(r.left_top(), egui::pos2(r.left() + e, r.bottom())), ResizeDirection::West),
        (egui::Rect::from_min_max(egui::pos2(r.right() - e, r.top()), r.right_bottom()), ResizeDirection::East),
    ];
    for (i, (rect, dir)) in handles.into_iter().enumerate() {
        let response = ui.interact(rect, Id::new(("resize", i)), Sense::drag());
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(cursor_for(dir));
        }
        if response.drag_started() {
            ui.ctx().send_viewport_cmd(ViewportCommand::BeginResize(dir));
        }
    }
}

fn cursor_for(dir: ResizeDirection) -> CursorIcon {
    match dir {
        ResizeDirection::North | ResizeDirection::South => CursorIcon::ResizeVertical,
        ResizeDirection::East | ResizeDirection::West => CursorIcon::ResizeHorizontal,
        ResizeDirection::NorthWest | ResizeDirection::SouthEast => CursorIcon::ResizeNwSe,
        ResizeDirection::NorthEast | ResizeDirection::SouthWest => CursorIcon::ResizeNeSw,
    }
}
