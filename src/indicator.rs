//! Indicator page: a pannable, zoomable candlestick chart over mock BTC/USD data,
//! with toggleable overlays, oscillator panes, and SMA-crossover entry/exit signals.

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Id, Pos2, Rect, Sense, Shape, Stroke, Ui, pos2,
    vec2,
};

use crate::{
    drawing::{self, Anchor, Drawings, Tool},
    format::{group, iso_date, nice_bounds, price, short_date, today_days},
    theme,
};

const TOOLBAR_HEIGHT: f32 = 36.0;
const PRICE_AXIS: f32 = 84.0;
const TIME_AXIS: f32 = 24.0;
/// Space above the price scale for the OHLC and overlay lines.
const INFO_HEIGHT: f32 = 48.0;
const CANDLES: usize = 320;
const SIGNAL_FAST: usize = 10;
const SIGNAL_SLOW: usize = 30;

const SMA_COLOR: Color32 = theme::SERIES[1];
const EMA_COLOR: Color32 = theme::SERIES[0];
const BB_COLOR: Color32 = theme::SERIES[4];
const RSI_COLOR: Color32 = theme::SERIES[3];
const ENTRY_FILL: Color32 = Color32::from_rgb(0x1a, 0x8f, 0x4c);
const EXIT_FILL: Color32 = Color32::from_rgb(0xd8, 0x2c, 0x44);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Toggle {
    Vol,
    Sma,
    Ema,
    Bb,
    Rsi,
    Macd,
    Signals,
}

impl Toggle {
    const ALL: [Toggle; 7] =
        [Toggle::Vol, Toggle::Sma, Toggle::Ema, Toggle::Bb, Toggle::Rsi, Toggle::Macd, Toggle::Signals];

    fn label(self) -> &'static str {
        match self {
            Toggle::Vol => "VOL",
            Toggle::Sma => "SMA",
            Toggle::Ema => "EMA",
            Toggle::Bb => "BB",
            Toggle::Rsi => "RSI",
            Toggle::Macd => "MACD",
            Toggle::Signals => "Signals",
        }
    }
}

#[derive(Clone, Copy)]
struct Candle {
    day: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Signal {
    Entry,
    Exit,
}

/// Series computed once over the full history. `NAN` marks warm-up gaps.
struct Series {
    sma: Vec<f64>,
    ema: Vec<f64>,
    bb_upper: Vec<f64>,
    bb_lower: Vec<f64>,
    rsi: Vec<f64>,
    macd: Vec<f64>,
    macd_signal: Vec<f64>,
    signals: Vec<(usize, Signal)>,
}

pub struct IndicatorPage {
    candles: Vec<Candle>,
    series: Series,
    enabled: Vec<Toggle>,
    drawings: Drawings,
    /// Index (fractional) of the right edge of the view, and how many candles fit.
    view_end: f32,
    visible: f32,
}

impl Default for IndicatorPage {
    fn default() -> Self {
        let candles = mock_candles();
        let series = Series::compute(&candles);
        Self {
            view_end: candles.len() as f32 + 4.0,
            visible: 110.0,
            candles,
            series,
            enabled: vec![Toggle::Vol, Toggle::Sma, Toggle::Ema, Toggle::Signals],
            drawings: Drawings::default(),
        }
    }
}

/// A horizontal band of the chart with its own value scale.
struct Pane {
    rect: Rect,
    lo: f64,
    hi: f64,
}

impl Pane {
    fn y(&self, v: f64) -> f32 {
        self.rect.bottom() - ((v - self.lo) / (self.hi - self.lo)) as f32 * self.rect.height()
    }

    fn value(&self, y: f32) -> f64 {
        self.lo + f64::from((self.rect.bottom() - y) / self.rect.height()) * (self.hi - self.lo)
    }
}

impl IndicatorPage {
    pub fn show(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        let toolbar = Rect::from_min_size(full.min, vec2(full.width(), TOOLBAR_HEIGHT));
        self.toolbar(ui, toolbar);

        let below = Rect::from_min_max(pos2(full.left(), toolbar.bottom()), full.max);
        let rail = Rect::from_min_max(below.min, pos2(below.left() + drawing::RAIL_WIDTH, below.bottom()));
        let area = Rect::from_min_max(pos2(rail.right(), below.top()), below.max);
        ui.allocate_rect(full, Sense::hover());
        self.drawings.rail(ui, rail);
        self.drawings.handle_keys(ui);
        self.chart(ui, area);
    }

    fn on(&self, t: Toggle) -> bool {
        self.enabled.contains(&t)
    }

    fn toolbar(&mut self, ui: &mut Ui, bar: Rect) {
        let p = ui.painter();
        p.rect_filled(bar, CornerRadius::ZERO, theme::SURFACE);
        p.hline(bar.x_range(), bar.bottom() - 0.5, Stroke::new(1.0, theme::BORDER));

        let font = FontId::proportional(12.5);
        let mut x = bar.left() + 16.0;
        p.text(pos2(x, bar.center().y), Align2::LEFT_CENTER, "BTC/USD · 1D", FontId::proportional(13.0), theme::TEXT);
        x += 104.0;
        p.vline(x, bar.shrink2(vec2(0.0, 10.0)).y_range(), Stroke::new(1.0, theme::BORDER));
        x += 8.0;

        for t in Toggle::ALL {
            let galley = ui.painter().layout_no_wrap(t.label().to_owned(), font.clone(), theme::TEXT);
            let rect = Rect::from_min_size(pos2(x, bar.top() + 6.0), vec2(galley.size().x + 18.0, bar.height() - 12.0));
            x = rect.right() + 2.0;
            let response = ui.interact(rect, Id::new(("indicator_toggle", t.label())), Sense::click());
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if response.clicked() {
                if let Some(i) = self.enabled.iter().position(|e| *e == t) {
                    self.enabled.remove(i);
                } else {
                    self.enabled.push(t);
                }
            }
            let on = self.on(t);
            let p = ui.painter();
            if on {
                p.rect_filled(rect, CornerRadius::same(5), theme::SURFACE_RAISED);
            } else if response.hovered() {
                p.rect_filled(rect, CornerRadius::same(5), theme::HOVER);
            }
            let color = if on { theme::TEXT } else { theme::TEXT_MUTED };
            p.text(rect.center(), Align2::CENTER_CENTER, t.label(), font.clone(), color);
        }
    }

    fn chart(&mut self, ui: &mut Ui, area: Rect) {
        let n = self.candles.len();
        let plot_x = area.left()..=area.right() - PRICE_AXIS;
        let body = Rect::from_x_y_ranges(plot_x.clone(), area.top()..=area.bottom() - TIME_AXIS);

        // Pan by dragging, zoom with the wheel around the pointer.
        let response = ui.interact(body, Id::new("indicator_chart"), Sense::click_and_drag());
        let w = body.width() / self.visible;
        if response.dragged() {
            self.view_end -= response.drag_delta().x / w;
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        }
        if let Some(hover) = response.hover_pos() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let anchor = self.view_end - self.visible + (hover.x - body.left()) / w;
                let frac = (hover.x - body.left()) / body.width();
                self.visible = (self.visible * (1.0 - scroll * 0.0015)).clamp(20.0, n as f32 + 20.0);
                self.view_end = anchor + self.visible * (1.0 - frac);
            }
        }
        self.view_end = self.view_end.clamp(self.visible * 0.25, n as f32 + self.visible * 0.5);

        let w = body.width() / self.visible;
        let start = self.view_end - self.visible;
        let first = start.floor().max(0.0) as usize;
        let last = (self.view_end.ceil().max(0.0) as usize).min(n);
        let x_of = |i: usize| body.left() + (i as f32 - start + 0.5) * w;
        let visible = if first < last { first..last } else { n - 1..n };

        // Split the body into the main pane and any oscillator panes.
        let subs: Vec<Toggle> = [Toggle::Rsi, Toggle::Macd].into_iter().filter(|t| self.on(*t)).collect();
        let sub_h = (body.height() * 0.2).max(70.0);
        let main_bottom = body.bottom() - sub_h * subs.len() as f32;
        let main_rect = Rect::from_x_y_ranges(plot_x.clone(), body.top()..=main_bottom);

        let main = self.main_pane(main_rect, visible.clone());
        let p = ui.painter().with_clip_rect(area);
        let plot_p = ui.painter().with_clip_rect(body);
        p.rect_filled(area, CornerRadius::ZERO, theme::BG);

        // Grid + price axis.
        let (_, _, step) = nice_bounds(main.lo, main.hi, 6);
        let mut v = (main.lo / step).ceil() * step;
        while v < main.hi {
            let y = main.y(v).round() + 0.5;
            plot_p.hline(plot_x.clone(), y, Stroke::new(1.0, theme::GRID));
            p.text(pos2(area.right() - 8.0, y), Align2::RIGHT_CENTER, price(v), FontId::proportional(11.5), theme::TEXT_MUTED);
            v += step;
        }
        p.vline(body.right() + 0.5, area.y_range(), Stroke::new(1.0, theme::BORDER));
        p.hline(area.x_range(), body.bottom() + 0.5, Stroke::new(1.0, theme::BORDER));

        // Time axis: labels at least ~90px apart.
        let every = [1usize, 2, 5, 7, 14, 30, 60, 90].into_iter().find(|s| *s as f32 * w >= 90.0).unwrap_or(180);
        for i in visible.clone().filter(|i| (self.candles[*i].day % every as i64) == 0) {
            let x = x_of(i).round() + 0.5;
            plot_p.vline(x, body.y_range(), Stroke::new(1.0, theme::GRID));
            p.text(pos2(x, body.bottom() + 6.0), Align2::CENTER_TOP, short_date(self.candles[i].day), FontId::proportional(11.5), theme::TEXT_MUTED);
        }

        if self.on(Toggle::Vol) {
            let max_vol = visible.clone().map(|i| self.candles[i].volume).fold(0.0, f64::max);
            let vol_h = main.rect.height() * 0.16;
            for i in visible.clone() {
                let c = &self.candles[i];
                let h = (c.volume / max_vol) as f32 * vol_h;
                let x = x_of(i);
                let bar = Rect::from_min_max(pos2(x - w * 0.35, main.rect.bottom() - h), pos2(x + w * 0.35, main.rect.bottom()));
                plot_p.rect_filled(bar, CornerRadius::ZERO, candle_color(c).gamma_multiply(0.3));
            }
        }

        let s = &self.series;
        if self.on(Toggle::Bb) {
            line(&plot_p, &main, &s.bb_upper, visible.clone(), x_of, BB_COLOR, 1.0);
            line(&plot_p, &main, &s.bb_lower, visible.clone(), x_of, BB_COLOR, 1.0);
        }
        if self.on(Toggle::Sma) {
            line(&plot_p, &main, &s.sma, visible.clone(), x_of, SMA_COLOR, 1.5);
        }
        if self.on(Toggle::Ema) {
            line(&plot_p, &main, &s.ema, visible.clone(), x_of, EMA_COLOR, 1.5);
        }

        // Candles.
        let body_w = (w * 0.7).max(1.0);
        for i in visible.clone() {
            let c = &self.candles[i];
            let color = candle_color(c);
            let x = x_of(i).round() + 0.5;
            plot_p.vline(x, main.y(c.high)..=main.y(c.low), Stroke::new(1.0, color));
            let (top, bottom) = (main.y(c.open.max(c.close)), main.y(c.open.min(c.close)));
            let r = Rect::from_min_max(pos2(x - body_w / 2.0, top), pos2(x + body_w / 2.0, bottom.max(top + 1.0)));
            plot_p.rect_filled(r, CornerRadius::ZERO, color);
        }

        self.extremes(&plot_p, &main, visible.clone(), x_of);
        if self.on(Toggle::Signals) {
            self.signals(&plot_p, &main, visible.clone(), x_of, w);
        }

        // Drawings: place on click, preview the next point under the pointer.
        let to_screen = |a: &Anchor| pos2(body.left() + (a.index - start + 0.5) * w, main.y(a.price));
        let to_anchor = |pt: Pos2| Anchor { index: start + (pt.x - body.left()) / w - 0.5, price: main.value(pt.y) };
        let drawing_p = ui.painter().with_clip_rect(main_rect);
        for d in &self.drawings.done {
            let pts: Vec<Pos2> = d.anchors.iter().map(to_screen).collect();
            drawing::paint(&drawing_p, d.tool, &d.anchors, &pts, main_rect);
        }
        if self.drawings.tool != Tool::Cursor {
            let pointer = response.hover_pos().filter(|h| main_rect.contains(*h));
            if pointer.is_some() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            }
            if response.clicked()
                && let Some(at) = response.interact_pointer_pos().filter(|h| main_rect.contains(*h))
            {
                self.drawings.click(to_anchor(at));
            }
            if let Some(h) = pointer.filter(|_| self.drawings.tool != Tool::Cursor) {
                let mut anchors = self.drawings.pending.clone();
                anchors.push(to_anchor(h));
                let pts: Vec<Pos2> = anchors.iter().map(to_screen).collect();
                drawing::paint(&drawing_p, self.drawings.tool, &anchors, &pts, main_rect);
            }
        }

        // Oscillator panes.
        let mut y = main_bottom;
        let mut panes = Vec::new();
        for t in &subs {
            let rect = Rect::from_x_y_ranges(plot_x.clone(), y..=y + sub_h);
            p.hline(area.x_range(), y + 0.5, Stroke::new(1.0, theme::BORDER));
            panes.push(self.sub_pane(ui, &p, rect, *t, visible.clone(), x_of, w, area.right()));
            y += sub_h;
        }

        // Last price line + axis tag.
        let last_c = self.candles[n - 1];
        let ly = main.y(last_c.close);
        if main.rect.y_range().contains(ly) {
            let color = candle_color(&last_c);
            plot_p.extend(Shape::dashed_line(&[pos2(body.left(), ly), pos2(body.right(), ly)], Stroke::new(1.0, color), 4.0, 3.0));
            axis_tag(&p, area.right(), body.right(), ly, &price(last_c.close), color, Color32::WHITE);
        }

        // Crosshair.
        let hovered = response.hover_pos().filter(|_| !response.dragged());
        let mut info_index = n - 1;
        if let Some(h) = hovered {
            let i = (start + (h.x - body.left()) / w).floor();
            if i >= 0.0 && (i as usize) < n {
                info_index = i as usize;
            }
            let cross = Stroke::new(1.0, theme::TEXT_MUTED.gamma_multiply(0.7));
            let cx = x_of(info_index).round() + 0.5;
            plot_p.extend(Shape::dashed_line(&[pos2(cx, body.top()), pos2(cx, body.bottom())], cross, 3.0, 3.0));
            plot_p.extend(Shape::dashed_line(&[pos2(body.left(), h.y), pos2(body.right(), h.y)], cross, 3.0, 3.0));
            let pane = std::iter::once(&main).chain(panes.iter()).find(|pn| pn.rect.y_range().contains(h.y));
            if let Some(pane) = pane {
                axis_tag(&p, area.right(), body.right(), h.y, &price(pane.value(h.y)), theme::SURFACE_RAISED, theme::TEXT);
            }
            let label = iso_date(self.candles[info_index].day);
            let g = p.layout_no_wrap(label, FontId::proportional(11.5), theme::TEXT);
            let tag = Rect::from_center_size(pos2(cx, body.bottom() + TIME_AXIS / 2.0), g.size() + vec2(12.0, 6.0));
            p.rect_filled(tag, CornerRadius::same(3), theme::SURFACE_RAISED);
            p.galley(tag.center() - g.size() / 2.0, g, theme::TEXT);
        }

        self.info(&p, main_rect, info_index);
    }

    fn main_pane(&self, rect: Rect, visible: std::ops::Range<usize>) -> Pane {
        let mut lo = f64::MAX;
        let mut hi = f64::MIN;
        for i in visible {
            lo = lo.min(self.candles[i].low);
            hi = hi.max(self.candles[i].high);
            if self.on(Toggle::Bb) && !self.series.bb_upper[i].is_nan() {
                lo = lo.min(self.series.bb_lower[i]);
                hi = hi.max(self.series.bb_upper[i]);
            }
        }
        // Prices start below the two info lines; leave room for volume and signal labels.
        let rect = rect.with_min_y((rect.top() + INFO_HEIGHT).min(rect.bottom() - 40.0));
        let span = (hi - lo).max(1.0);
        let bottom_pad = if self.on(Toggle::Vol) { 0.28 } else { 0.14 };
        Pane { rect, lo: lo - span * bottom_pad, hi: hi + span * 0.12 }
    }

    #[allow(clippy::too_many_arguments)]
    fn sub_pane(
        &self,
        ui: &Ui,
        p: &egui::Painter,
        rect: Rect,
        kind: Toggle,
        visible: std::ops::Range<usize>,
        x_of: impl Fn(usize) -> f32 + Copy,
        w: f32,
        axis_right: f32,
    ) -> Pane {
        let s = &self.series;
        let plot_p = ui.painter().with_clip_rect(rect);
        let label_font = FontId::proportional(11.5);
        let inner = rect.shrink2(vec2(0.0, 8.0));
        match kind {
            Toggle::Rsi => {
                let pane = Pane { rect: inner, lo: 0.0, hi: 100.0 };
                for level in [30.0, 70.0] {
                    let y = pane.y(level).round() + 0.5;
                    plot_p.extend(Shape::dashed_line(&[pos2(rect.left(), y), pos2(rect.right(), y)], Stroke::new(1.0, theme::BORDER), 4.0, 3.0));
                    p.text(pos2(axis_right - 8.0, y), Align2::RIGHT_CENTER, format!("{level:.0}"), label_font.clone(), theme::TEXT_MUTED);
                }
                line(&plot_p, &pane, &s.rsi, visible.clone(), x_of, RSI_COLOR, 1.5);
                let last = visible.end.saturating_sub(1);
                pane_label(p, rect, &[("RSI 14", theme::TEXT_MUTED), (&format!("{:.2}", s.rsi[last]), RSI_COLOR)]);
                pane
            }
            _ => {
                let (mut lo, mut hi) = (0.0_f64, 0.0_f64);
                for i in visible.clone().filter(|i| !s.macd_signal[*i].is_nan()) {
                    let hist = s.macd[i] - s.macd_signal[i];
                    lo = lo.min(s.macd[i]).min(s.macd_signal[i]).min(hist);
                    hi = hi.max(s.macd[i]).max(s.macd_signal[i]).max(hist);
                }
                let pad = (hi - lo).max(1.0) * 0.1;
                let pane = Pane { rect: inner, lo: lo - pad, hi: hi + pad };
                let zero = pane.y(0.0);
                plot_p.hline(rect.x_range(), zero.round() + 0.5, Stroke::new(1.0, theme::BORDER));
                p.text(pos2(axis_right - 8.0, zero), Align2::RIGHT_CENTER, "0", label_font, theme::TEXT_MUTED);
                for i in visible.clone().filter(|i| !s.macd_signal[*i].is_nan()) {
                    let hist = s.macd[i] - s.macd_signal[i];
                    let x = x_of(i);
                    let y = pane.y(hist);
                    let color = if hist >= 0.0 { theme::CANDLE_UP } else { theme::CANDLE_DOWN };
                    let bar = Rect::from_two_pos(pos2(x - w * 0.35, zero), pos2(x + w * 0.35, y));
                    plot_p.rect_filled(bar, CornerRadius::ZERO, color.gamma_multiply(0.5));
                }
                line(&plot_p, &pane, &s.macd, visible.clone(), x_of, theme::SERIES[0], 1.5);
                line(&plot_p, &pane, &s.macd_signal, visible.clone(), x_of, theme::SERIES[1], 1.5);
                let last = visible.end.saturating_sub(1);
                pane_label(
                    p,
                    rect,
                    &[
                        ("MACD 12 26 9", theme::TEXT_MUTED),
                        (&format!("{:.2}", s.macd[last]), theme::SERIES[0]),
                        (&format!("{:.2}", s.macd_signal[last]), theme::SERIES[1]),
                    ],
                );
                pane
            }
        }
    }

    /// Small leader labels on the highest high and lowest low in view.
    fn extremes(&self, p: &egui::Painter, pane: &Pane, visible: std::ops::Range<usize>, x_of: impl Fn(usize) -> f32) {
        let hi = visible.clone().max_by(|a, b| self.candles[*a].high.total_cmp(&self.candles[*b].high));
        let lo = visible.min_by(|a, b| self.candles[*a].low.total_cmp(&self.candles[*b].low));
        let font = FontId::proportional(11.0);
        for (i, is_high) in [(hi, true), (lo, false)].into_iter().filter_map(|(i, h)| i.map(|i| (i, h))) {
            let c = &self.candles[i];
            let (v, dy) = if is_high { (c.high, -12.0) } else { (c.low, 12.0) };
            let at = pos2(x_of(i), pane.y(v));
            let end = at + vec2(-14.0, dy);
            p.line_segment([at + vec2(0.0, dy.signum() * 3.0), end], Stroke::new(1.0, theme::TEXT_MUTED));
            p.text(end + vec2(-3.0, 0.0), Align2::RIGHT_CENTER, price(v), font.clone(), theme::TEXT_MUTED);
        }
    }

    fn signals(&self, p: &egui::Painter, pane: &Pane, visible: std::ops::Range<usize>, x_of: impl Fn(usize) -> f32, w: f32) {
        let font = FontId::proportional(12.0);
        let mut placed: Vec<Rect> = Vec::new();
        for (i, kind) in self.series.signals.iter().filter(|(i, _)| visible.contains(i)) {
            let c = &self.candles[*i];
            let x = x_of(*i);
            let (text, fill, dot_y, dir) = match kind {
                Signal::Entry => ("Long Entry", ENTRY_FILL, pane.y(c.low) + 8.0, 1.0),
                Signal::Exit => ("Long Exit", EXIT_FILL, pane.y(c.high) - 8.0, -1.0),
            };
            let dot = pos2(x, dot_y);
            let g = p.layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE);
            let size = g.size() + vec2(16.0, 8.0);
            let mut label = Rect::from_center_size(dot + vec2(0.0, dir * (22.0 + size.y / 2.0)), size);
            // Push away from the candle until it clears earlier labels.
            while let Some(hit) = placed.iter().find(|r| r.expand(2.0).intersects(label)) {
                let shift = if dir > 0.0 { hit.bottom() + 3.0 - label.top() } else { hit.top() - 3.0 - label.bottom() };
                label = label.translate(vec2(0.0, shift));
            }
            placed.push(label);
            p.line_segment([dot, pos2(x, if dir > 0.0 { label.top() } else { label.bottom() })], Stroke::new(1.0, fill));
            p.circle_filled(dot, (w * 0.3).clamp(3.0, 5.0) + 1.5, theme::BG);
            p.circle_filled(dot, (w * 0.3).clamp(3.0, 5.0), fill);
            p.rect_filled(label, CornerRadius::same(3), fill);
            p.galley(label.center() - g.size() / 2.0, g, Color32::WHITE);
        }
    }

    /// OHLCV line for the hovered (or latest) candle, plus overlay values.
    fn info(&self, p: &egui::Painter, rect: Rect, i: usize) {
        let c = &self.candles[i];
        let font = FontId::proportional(12.0);
        let change = (c.close - c.open) / c.open * 100.0;
        let value_color = candle_color(c);
        let mut parts: Vec<(String, Color32)> = vec![
            (iso_date(c.day), theme::TEXT_MUTED),
            ("O".into(), theme::TEXT_MUTED),
            (price(c.open), value_color),
            ("H".into(), theme::TEXT_MUTED),
            (price(c.high), value_color),
            ("L".into(), theme::TEXT_MUTED),
            (price(c.low), value_color),
            ("C".into(), theme::TEXT_MUTED),
            (price(c.close), value_color),
            (format!("{}{:.2}%", if change >= 0.0 { "+" } else { "" }, change), value_color),
            ("Vol".into(), theme::TEXT_MUTED),
            (format!("{}", group(c.volume.round() as u64)), theme::TEXT),
        ];
        let mut x = rect.left() + 12.0;
        let y = rect.top() + 16.0;
        for (text, color) in parts.drain(..) {
            let r = p.text(pos2(x, y), Align2::LEFT_CENTER, text, font.clone(), color);
            x = r.right() + 6.0;
        }

        let s = &self.series;
        let mut overlays = Vec::new();
        if self.on(Toggle::Sma) {
            overlays.push(("SMA 20", s.sma[i], SMA_COLOR));
        }
        if self.on(Toggle::Ema) {
            overlays.push(("EMA 50", s.ema[i], EMA_COLOR));
        }
        if self.on(Toggle::Bb) {
            overlays.push(("BB 20 2 upper", s.bb_upper[i], BB_COLOR));
            overlays.push(("lower", s.bb_lower[i], BB_COLOR));
        }
        let mut x = rect.left() + 12.0;
        for (name, v, color) in overlays {
            let r = p.text(pos2(x, y + 20.0), Align2::LEFT_CENTER, name, font.clone(), theme::TEXT_MUTED);
            let value = if v.is_nan() { "—".to_owned() } else { price(v) };
            let r = p.text(pos2(r.right() + 5.0, y + 20.0), Align2::LEFT_CENTER, value, font.clone(), color);
            x = r.right() + 14.0;
        }
    }
}

impl Series {
    fn compute(c: &[Candle]) -> Self {
        let close: Vec<f64> = c.iter().map(|c| c.close).collect();
        let sma20 = sma(&close, 20);
        let std = rolling_std(&close, 20);
        let bb_upper = sma20.iter().zip(&std).map(|(m, s)| m + 2.0 * s).collect();
        let bb_lower = sma20.iter().zip(&std).map(|(m, s)| m - 2.0 * s).collect();
        let ema12 = ema(&close, 12);
        let ema26 = ema(&close, 26);
        let macd: Vec<f64> = ema12.iter().zip(&ema26).map(|(a, b)| a - b).collect();
        let macd_signal = {
            let valid = macd.iter().position(|v| !v.is_nan()).unwrap_or(0);
            let mut out = vec![f64::NAN; valid];
            out.extend(ema(&macd[valid..], 9));
            out
        };

        // Long while the fast SMA is above the slow one.
        let fast = sma(&close, SIGNAL_FAST);
        let slow = sma(&close, SIGNAL_SLOW);
        let mut signals = Vec::new();
        for i in 1..c.len() {
            if slow[i - 1].is_nan() {
                continue;
            }
            let was = fast[i - 1] > slow[i - 1];
            let is = fast[i] > slow[i];
            if is && !was {
                signals.push((i, Signal::Entry));
            } else if was && !is && !signals.is_empty() {
                signals.push((i, Signal::Exit));
            }
        }

        let ema50 = ema(&close, 50);
        Self { sma: sma20, ema: ema50, bb_upper, bb_lower, rsi: rsi(&close, 14), macd, macd_signal, signals }
    }
}

fn sma(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    let mut sum = 0.0;
    for i in 0..v.len() {
        sum += v[i];
        if i >= n {
            sum -= v[i - n];
        }
        if i + 1 >= n {
            out[i] = sum / n as f64;
        }
    }
    out
}

fn rolling_std(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    for i in n.saturating_sub(1)..v.len() {
        let win = &v[i + 1 - n..=i];
        let mean = win.iter().sum::<f64>() / n as f64;
        out[i] = (win.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64).sqrt();
    }
    out
}

/// EMA seeded with the SMA of the first `n` values.
fn ema(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    if v.len() < n {
        return out;
    }
    let k = 2.0 / (n as f64 + 1.0);
    let mut e = v[..n].iter().sum::<f64>() / n as f64;
    out[n - 1] = e;
    for i in n..v.len() {
        e = v[i] * k + e * (1.0 - k);
        out[i] = e;
    }
    out
}

/// Wilder's RSI.
fn rsi(v: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; v.len()];
    if v.len() <= n {
        return out;
    }
    let (mut gain, mut loss) = (0.0, 0.0);
    for i in 1..=n {
        let d = v[i] - v[i - 1];
        if d > 0.0 { gain += d } else { loss -= d }
    }
    gain /= n as f64;
    loss /= n as f64;
    let value = |g: f64, l: f64| if l == 0.0 { 100.0 } else { 100.0 - 100.0 / (1.0 + g / l) };
    out[n] = value(gain, loss);
    for i in n + 1..v.len() {
        let d = v[i] - v[i - 1];
        gain = (gain * (n as f64 - 1.0) + d.max(0.0)) / n as f64;
        loss = (loss * (n as f64 - 1.0) + (-d).max(0.0)) / n as f64;
        out[i] = value(gain, loss);
    }
    out
}

fn candle_color(c: &Candle) -> Color32 {
    if c.close >= c.open { theme::CANDLE_UP } else { theme::CANDLE_DOWN }
}

/// Polyline through a series' non-NaN values in view.
fn line(
    p: &egui::Painter,
    pane: &Pane,
    values: &[f64],
    visible: std::ops::Range<usize>,
    x_of: impl Fn(usize) -> f32,
    color: Color32,
    width: f32,
) {
    // Extend one candle past each edge so the line runs off-screen instead of stopping short.
    let range = visible.start.saturating_sub(1)..(visible.end + 1).min(values.len());
    let points: Vec<Pos2> = range.filter(|i| !values[*i].is_nan()).map(|i| pos2(x_of(i), pane.y(values[i]))).collect();
    if points.len() > 1 {
        p.add(Shape::line(points, Stroke::new(width, color)));
    }
}

/// A value tag on the price axis, with a short tick into the plot.
fn axis_tag(p: &egui::Painter, axis_right: f32, plot_right: f32, y: f32, text: &str, fill: Color32, fg: Color32) {
    let g = p.layout_no_wrap(text.to_owned(), FontId::proportional(11.5), fg);
    let rect = Rect::from_min_max(pos2(plot_right + 1.0, y - 10.0), pos2(axis_right, y + 10.0));
    p.rect_filled(rect, CornerRadius::same(2), fill);
    p.galley(pos2(axis_right - 8.0 - g.size().x, y - g.size().y / 2.0), g, fg);
}

fn pane_label(p: &egui::Painter, rect: Rect, parts: &[(&str, Color32)]) {
    let mut x = rect.left() + 12.0;
    for (text, color) in parts {
        let r = p.text(pos2(x, rect.top() + 12.0), Align2::LEFT_CENTER, *text, FontId::proportional(11.5), *color);
        x = r.right() + 8.0;
    }
}

/// Seeded daily BTC-like candles ending today.
fn mock_candles() -> Vec<Candle> {
    let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut uniform = move || {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let today = today_days();
    let mut close = 26_500.0;
    let mut drift = 0.002;
    let mut out = Vec::with_capacity(CANDLES);
    for k in 0..CANDLES {
        // Regime shifts every ~40 days keep the trend from looking like pure noise.
        if k % 40 == 0 {
            drift = (uniform() - 0.4) * 0.008;
        }
        let noise = uniform() + uniform() + uniform() - 1.5;
        let open = close * (1.0 + (uniform() - 0.5) * 0.004);
        close = open * (1.0 + drift + noise * 0.035);
        let wick = 0.004 + uniform() * 0.018;
        let high = open.max(close) * (1.0 + wick * uniform());
        let low = open.min(close) * (1.0 - wick * uniform());
        let volume = 8_000.0 + uniform() * 14_000.0 + (close - open).abs() / open * 400_000.0;
        out.push(Candle { day: today - (CANDLES - 1 - k) as i64, open, high, low, close, volume });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Context, Event, PointerButton, RawInput};

    fn frame(ctx: &Context, page: &mut IndicatorPage, events: Vec<Event>) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 700.0))),
            events,
            ..Default::default()
        };
        // No renderer here, so discard the font texture uploads.
        ctx.run_ui(input, |ui| page.show(ui)).textures_delta.clear();
    }

    fn click(ctx: &Context, page: &mut IndicatorPage, at: Pos2) {
        let button = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Default::default() };
        frame(ctx, page, vec![Event::PointerMoved(at)]);
        frame(ctx, page, vec![button(true)]);
        frame(ctx, page, vec![button(false)]);
    }

    #[test]
    fn clicking_toolbar_toggles_indicators() {
        let ctx = Context::default();
        let mut page = IndicatorPage::default();
        frame(&ctx, &mut page, vec![]);

        for t in [Toggle::Rsi, Toggle::Macd, Toggle::Bb] {
            assert!(!page.on(t));
            let rect = ctx.read_response(Id::new(("indicator_toggle", t.label()))).expect("toggle rendered").rect;
            click(&ctx, &mut page, rect.center());
            assert!(page.on(t), "{} should be on after one click", t.label());
        }

        let sma = ctx.read_response(Id::new(("indicator_toggle", "SMA"))).unwrap().rect;
        click(&ctx, &mut page, sma.center());
        assert!(!page.on(Toggle::Sma), "clicking an enabled toggle turns it off");
    }

    #[test]
    fn signals_alternate_entry_then_exit() {
        let page = IndicatorPage::default();
        let kinds: Vec<Signal> = page.series.signals.iter().map(|(_, k)| *k).collect();
        assert!(!kinds.is_empty());
        assert_eq!(kinds[0], Signal::Entry);
        assert!(kinds.windows(2).all(|w| w[0] != w[1]));
    }
}
