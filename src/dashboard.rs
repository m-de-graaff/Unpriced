//! Mock portfolio dashboard. All figures are generated locally from a fixed seed.

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Frame, Layout, Margin, Mesh, Pos2, Rect,
    RichText, Sense, Shape, Stroke, Ui, pos2, vec2,
};

use crate::{
    account::Broker,
    format::{compact_money, money, nice_bounds, short_date, signed_money, signed_pct, today_days, trim_qty},
    theme,
};

const GAP: f32 = 16.0;
/// Below this width, side-by-side rows stack vertically.
const WIDE: f32 = 860.0;
const HISTORY_DAYS: usize = 365;
const ENDING_BALANCE: f64 = 128_450.32;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Range {
    Month,
    Quarter,
    Half,
    Year,
}

impl Range {
    const ALL: [Range; 4] = [Range::Month, Range::Quarter, Range::Half, Range::Year];

    fn label(self) -> &'static str {
        match self {
            Range::Month => "1M",
            Range::Quarter => "3M",
            Range::Half => "6M",
            Range::Year => "1Y",
        }
    }

    fn days(self) -> usize {
        match self {
            Range::Month => 30,
            Range::Quarter => 90,
            Range::Half => 180,
            Range::Year => HISTORY_DAYS,
        }
    }
}

struct Position {
    symbol: &'static str,
    name: &'static str,
    qty: f64,
    price: f64,
    day_pct: f64,
}

impl Position {
    fn value(&self) -> f64 {
        self.qty * self.price
    }
}

const POSITIONS: [Position; 7] = [
    Position { symbol: "AAPL", name: "Apple", qty: 120.0, price: 247.10, day_pct: 0.82 },
    Position { symbol: "NVDA", name: "NVIDIA", qty: 150.0, price: 138.45, day_pct: 2.41 },
    Position { symbol: "MSFT", name: "Microsoft", qty: 40.0, price: 431.20, day_pct: -0.37 },
    Position { symbol: "SPY", name: "S&P 500 ETF", qty: 25.0, price: 584.30, day_pct: 0.54 },
    Position { symbol: "ASML", name: "ASML Holding", qty: 12.0, price: 712.80, day_pct: -1.18 },
    Position { symbol: "BTC", name: "Bitcoin", qty: 0.12, price: 67_350.0, day_pct: 3.06 },
    Position { symbol: "TSLA", name: "Tesla", qty: 20.0, price: 219.60, day_pct: -2.24 },
];

/// (days ago, description, broker, signed amount)
const ACTIVITY: [(i64, &str, Broker, f64); 6] = [
    (0, "Bought 10 NVDA @ 138.45", Broker::Alpaca, -1_384.50),
    (1, "Dividend AAPL", Broker::InteractiveBrokers, 30.00),
    (2, "Sold 5 TSLA @ 224.10", Broker::Etoro, 1_120.50),
    (4, "Deposit", Broker::Alpaca, 5_000.00),
    (6, "Bought 0.02 BTC @ 66,120", Broker::Etoro, -1_322.40),
    (9, "Bought 4 ASML @ 698.00", Broker::InteractiveBrokers, -2_792.00),
];

pub struct Dashboard {
    range: Range,
    /// Daily closing equity, oldest first; the last entry is today.
    equity: Vec<f64>,
    today: i64,
}

impl Default for Dashboard {
    fn default() -> Self {
        Self { range: Range::Quarter, equity: mock_equity(), today: today_days() }
    }
}

impl Dashboard {
    pub fn show(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Dashboard").size(26.0).strong());
                ui.add_space(4.0);
                badge(ui, "Mock data");
            });
            ui.label(RichText::new("All accounts, combined.").color(theme::TEXT_MUTED));
            ui.add_space(GAP);

            self.kpis(ui);
            ui.add_space(GAP);
            let total = self.total();
            row(ui, 0.66, |ui| self.equity_card(ui), |ui| accounts_card(ui, total));
            ui.add_space(GAP);
            row(ui, 0.66, positions_card, |ui| self.stats_card(ui));
            ui.add_space(GAP);
            self.activity_card(ui);
            ui.add_space(GAP);
        });
    }

    fn total(&self) -> f64 {
        *self.equity.last().unwrap_or(&0.0)
    }

    fn window(&self) -> &[f64] {
        let n = (self.range.days() + 1).min(self.equity.len());
        &self.equity[self.equity.len() - n..]
    }

    fn kpis(&self, ui: &mut Ui) {
        let total = self.total();
        let prev = self.equity[self.equity.len() - 2];
        let start = self.equity[0];
        let invested: f64 = POSITIONS.iter().map(Position::value).sum();
        let cash = total - invested;

        let tiles = [
            ("Total balance", money(total), Some((total - prev) / prev * 100.0), "today"),
            ("Today's P&L", signed_money(total - prev), None, ""),
            ("1Y return", signed_money(total - start), Some((total - start) / start * 100.0), "past year"),
            ("Buying power", money(cash), None, ""),
        ];

        let per_row = if ui.available_width() < WIDE { 2 } else { 4 };
        let w = (ui.available_width() - GAP * (per_row - 1) as f32) / per_row as f32;
        for chunk in tiles.chunks(per_row) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = GAP;
                for (label, value, pct, period) in chunk {
                    ui.allocate_ui_with_layout(vec2(w, 0.0), Layout::top_down(Align::Min), |ui| {
                        card(ui, 0.0, |ui| {
                            ui.spacing_mut().item_spacing.y = 4.0;
                            ui.label(RichText::new(*label).size(12.0).color(theme::TEXT_MUTED));
                            ui.label(RichText::new(value).size(22.0).strong());
                            match pct {
                                Some(p) => {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;
                                        ui.label(RichText::new(signed_pct(*p)).size(12.0).color(delta_color(*p)));
                                        ui.label(RichText::new(*period).size(12.0).color(theme::TEXT_MUTED));
                                    });
                                }
                                None => {
                                    let note = if *label == "Buying power" { "Uninvested cash" } else { "Across all accounts" };
                                    ui.label(RichText::new(note).size(12.0).color(theme::TEXT_MUTED));
                                }
                            }
                        });
                    });
                }
            });
            ui.add_space(GAP);
        }
        ui.add_space(-GAP);
    }

    fn equity_card(&mut self, ui: &mut Ui) {
        card(ui, 0.0, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Portfolio value").strong());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for range in Range::ALL.into_iter().rev() {
                        ui.selectable_value(&mut self.range, range, range.label());
                    }
                });
            });
            ui.add_space(8.0);
            let (rect, response) =
                ui.allocate_exact_size(vec2(ui.available_width(), 240.0), Sense::hover());
            equity_chart(ui, rect, response.hover_pos(), self.window(), self.today);
        });
    }

    fn stats_card(&self, ui: &mut Ui) {
        let s = Stats::of(self.window());
        card(ui, 0.0, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Statistics").strong());
                ui.label(RichText::new(self.range.label()).color(theme::TEXT_MUTED));
            });
            ui.add_space(8.0);
            let rows = [
                ("Win rate", format!("{:.0}%", s.win_rate * 100.0), None),
                ("Sharpe ratio", format!("{:.2}", s.sharpe), None),
                ("Volatility (ann.)", format!("{:.1}%", s.volatility * 100.0), None),
                ("Max drawdown", format!("{:.1}%", s.max_drawdown * 100.0), Some(s.max_drawdown)),
                ("Best day", signed_pct(s.best * 100.0), Some(s.best)),
                ("Worst day", signed_pct(s.worst * 100.0), Some(s.worst)),
            ];
            for (label, value, signed) in rows {
                let color = signed.map_or(theme::TEXT, delta_color);
                kv_row(ui, label, &value, color);
            }
        });
    }

    fn activity_card(&self, ui: &mut Ui) {
        card(ui, 0.0, |ui| {
            ui.label(RichText::new("Recent activity").strong());
            ui.add_space(4.0);
            for (i, (ago, text, broker, amount)) in ACTIVITY.iter().enumerate() {
                if i > 0 {
                    hairline(ui);
                }
                let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::hover());
                let p = ui.painter();
                let y = rect.center().y;
                let date = short_date(self.today - ago);
                p.text(pos2(rect.left(), y), Align2::LEFT_CENTER, date, FontId::proportional(12.5), theme::TEXT_MUTED);
                p.text(pos2(rect.left() + 64.0, y), Align2::LEFT_CENTER, *text, FontId::proportional(13.0), theme::TEXT);
                let amount_color = if *amount >= 0.0 { theme::GAIN } else { theme::TEXT };
                p.text(pos2(rect.right(), y), Align2::RIGHT_CENTER, signed_money(*amount), FontId::proportional(13.0), amount_color);
                if rect.width() > 520.0 {
                    p.text(
                        pos2(rect.right() - 120.0, y),
                        Align2::RIGHT_CENTER,
                        broker.label(),
                        FontId::proportional(12.5),
                        theme::TEXT_MUTED,
                    );
                }
            }
        });
    }
}

fn accounts_card(ui: &mut Ui, total: f64) {
    // Fixed split of the combined balance across brokers.
    let shares = [(Broker::Alpaca, 0.52, 0.94), (Broker::Etoro, 0.31, 1.87), (Broker::InteractiveBrokers, 0.17, 0.41)];
    card(ui, 0.0, |ui| {
        ui.label(RichText::new("Accounts").strong());
        ui.add_space(8.0);

        // Allocation bar: one segment per account, 2px surface gap between fills.
        let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 10.0), Sense::hover());
        let mut x = bar.left();
        for (i, (_, share, _)) in shares.iter().enumerate() {
            let w = bar.width() * *share as f32;
            let seg = Rect::from_min_max(pos2(x, bar.top()), pos2(x + w, bar.bottom()));
            let seg = if i + 1 < shares.len() { seg.with_max_x(seg.right() - 2.0) } else { seg };
            ui.painter().rect_filled(seg, CornerRadius::same(3), theme::SERIES[i]);
            x += w;
        }
        ui.add_space(10.0);

        for (i, (broker, share, day_pct)) in shares.iter().enumerate() {
            if i > 0 {
                hairline(ui);
            }
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::hover());
            let p = ui.painter();
            let swatch = Rect::from_center_size(pos2(rect.left() + 5.0, rect.center().y - 8.0), vec2(10.0, 10.0));
            p.rect_filled(swatch, CornerRadius::same(3), theme::SERIES[i]);
            p.text(pos2(rect.left() + 18.0, rect.center().y - 8.0), Align2::LEFT_CENTER, broker.label(), FontId::proportional(13.0), theme::TEXT);
            p.text(
                pos2(rect.left() + 18.0, rect.center().y + 9.0),
                Align2::LEFT_CENTER,
                format!("{:.0}% of total", share * 100.0),
                FontId::proportional(12.0),
                theme::TEXT_MUTED,
            );
            p.text(pos2(rect.right(), rect.center().y - 8.0), Align2::RIGHT_CENTER, money(total * share), FontId::proportional(13.0), theme::TEXT);
            p.text(
                pos2(rect.right(), rect.center().y + 9.0),
                Align2::RIGHT_CENTER,
                signed_pct(*day_pct),
                FontId::proportional(12.0),
                delta_color(*day_pct),
            );
        }
    });
}

fn positions_card(ui: &mut Ui) {
    card(ui, 0.0, |ui| {
        ui.label(RichText::new("Top positions").strong());
        ui.add_space(4.0);
        let total: f64 = POSITIONS.iter().map(Position::value).sum();
        let w = ui.available_width();
        let narrow = w < 480.0;
        // Right edges of the numeric columns.
        let cols: &[(&str, f32)] = if narrow {
            &[("Value", w - 70.0), ("Day", w)]
        } else {
            &[("Qty", w * 0.45), ("Price", w * 0.62), ("Value", w * 0.82), ("Day", w)]
        };

        let (head, _) = ui.allocate_exact_size(vec2(w, 26.0), Sense::hover());
        let header_font = FontId::proportional(12.0);
        ui.painter().text(pos2(head.left(), head.center().y), Align2::LEFT_CENTER, "Symbol", header_font.clone(), theme::TEXT_MUTED);
        for (label, x) in cols {
            ui.painter().text(pos2(head.left() + x, head.center().y), Align2::RIGHT_CENTER, *label, header_font.clone(), theme::TEXT_MUTED);
        }

        let mut sorted: Vec<&Position> = POSITIONS.iter().collect();
        sorted.sort_by(|a, b| b.value().total_cmp(&a.value()));
        for pos in sorted {
            hairline(ui);
            let (rect, response) = ui.allocate_exact_size(vec2(w, 40.0), Sense::hover());
            let p = ui.painter();
            if response.hovered() {
                p.rect_filled(rect.expand2(vec2(6.0, 0.0)), CornerRadius::same(5), theme::SURFACE_RAISED);
            }
            let y = rect.center().y;
            p.text(pos2(rect.left(), y - 7.0), Align2::LEFT_CENTER, pos.symbol, FontId::proportional(13.0), theme::TEXT);
            p.text(
                pos2(rect.left(), y + 9.0),
                Align2::LEFT_CENTER,
                format!("{} · {:.1}%", pos.name, pos.value() / total * 100.0),
                FontId::proportional(11.5),
                theme::TEXT_MUTED,
            );
            for (label, x) in cols {
                let (text, color) = match *label {
                    "Qty" => (trim_qty(pos.qty), theme::TEXT),
                    "Price" => (money(pos.price), theme::TEXT),
                    "Value" => (money(pos.value()), theme::TEXT),
                    _ => (signed_pct(pos.day_pct), delta_color(pos.day_pct)),
                };
                p.text(pos2(rect.left() + x, y), Align2::RIGHT_CENTER, text, FontId::proportional(13.0), color);
            }
        }
    });
}

/// Line + area chart of equity with a recessive grid and a hover crosshair.
fn equity_chart(ui: &Ui, rect: Rect, hover: Option<Pos2>, data: &[f64], today: i64) {
    let p = ui.painter();
    let label_font = FontId::proportional(11.5);
    let plot = Rect::from_min_max(pos2(rect.left() + 52.0, rect.top() + 6.0), pos2(rect.right() - 4.0, rect.bottom() - 22.0));

    let (lo, hi) = data.iter().fold((f64::MAX, f64::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
    let (lo, hi, step) = nice_bounds(lo, hi, 4);
    let y_of = |v: f64| plot.bottom() - ((v - lo) / (hi - lo)) as f32 * plot.height();
    let x_of = |i: usize| plot.left() + i as f32 / (data.len() - 1).max(1) as f32 * plot.width();

    // Grid + y labels.
    let mut v = lo;
    while v <= hi + step * 0.5 {
        let y = y_of(v).round() + 0.5;
        p.hline(plot.x_range(), y, Stroke::new(1.0, theme::GRID));
        p.text(pos2(rect.left(), y), Align2::LEFT_CENTER, compact_money(v), label_font.clone(), theme::TEXT_MUTED);
        v += step;
    }

    // X labels: five evenly spaced dates.
    let n = data.len();
    for k in 0..5 {
        let i = k * (n - 1) / 4;
        let align = match k {
            0 => Align2::LEFT_TOP,
            4 => Align2::RIGHT_TOP,
            _ => Align2::CENTER_TOP,
        };
        let day = today - (n - 1 - i) as i64;
        p.text(pos2(x_of(i), plot.bottom() + 6.0), align, short_date(day), label_font.clone(), theme::TEXT_MUTED);
    }

    let points: Vec<Pos2> = data.iter().enumerate().map(|(i, v)| pos2(x_of(i), y_of(*v))).collect();

    // Area: vertical gradient from the line down to the baseline.
    let mut mesh = Mesh::default();
    let top = theme::ACCENT.gamma_multiply(0.22);
    let bottom = theme::ACCENT.gamma_multiply(0.0);
    for (i, pt) in points.iter().enumerate() {
        mesh.colored_vertex(*pt, top);
        mesh.colored_vertex(pos2(pt.x, plot.bottom()), bottom);
        if i > 0 {
            let b = (i as u32) * 2;
            mesh.add_triangle(b - 2, b - 1, b);
            mesh.add_triangle(b - 1, b, b + 1);
        }
    }
    p.add(Shape::mesh(mesh));
    p.add(Shape::line(points.clone(), Stroke::new(2.0, theme::ACCENT)));

    // Hover: crosshair, marker with a surface ring, tooltip.
    let Some(h) = hover.filter(|h| plot.expand2(vec2(0.0, 6.0)).contains(*h)) else { return };
    let i = (((h.x - plot.left()) / plot.width()) * (n - 1) as f32).round().clamp(0.0, (n - 1) as f32) as usize;
    let pt = points[i];
    p.vline(pt.x.round() + 0.5, plot.y_range(), Stroke::new(1.0, theme::TEXT_MUTED.gamma_multiply(0.6)));
    p.circle_filled(pt, 6.0, theme::SURFACE);
    p.circle_filled(pt, 4.0, theme::ACCENT);

    let change = (data[i] - data[0]) / data[0] * 100.0;
    let lines = [
        (short_date(today - (n - 1 - i) as i64), theme::TEXT_MUTED, 11.5),
        (money(data[i]), theme::TEXT, 13.0),
        (format!("{} vs start", signed_pct(change)), delta_color(change), 11.5),
    ];
    let galleys: Vec<_> = lines
        .iter()
        .map(|(t, c, s)| p.layout_no_wrap(t.clone(), FontId::proportional(*s), *c))
        .collect();
    let w = galleys.iter().map(|g| g.size().x).fold(0.0, f32::max) + 20.0;
    let hgt = galleys.iter().map(|g| g.size().y + 2.0).sum::<f32>() + 14.0;
    let mut x = pt.x + 12.0;
    if x + w > plot.right() {
        x = pt.x - 12.0 - w;
    }
    let y = (pt.y - hgt / 2.0).clamp(plot.top(), plot.bottom() - hgt);
    let tip = Rect::from_min_size(pos2(x, y), vec2(w, hgt));
    p.rect(tip, CornerRadius::same(6), theme::SURFACE_RAISED, Stroke::new(1.0, theme::BORDER), egui::StrokeKind::Inside);
    let mut ty = tip.top() + 7.0;
    for g in galleys {
        let gh = g.size().y;
        p.galley(pos2(tip.left() + 10.0, ty), g, theme::TEXT);
        ty += gh + 2.0;
    }
}

struct Stats {
    win_rate: f64,
    sharpe: f64,
    volatility: f64,
    max_drawdown: f64,
    best: f64,
    worst: f64,
}

impl Stats {
    fn of(data: &[f64]) -> Self {
        let returns: Vec<f64> = data.windows(2).map(|w| w[1] / w[0] - 1.0).collect();
        let n = returns.len().max(1) as f64;
        let mean = returns.iter().sum::<f64>() / n;
        let std = (returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n).sqrt();
        let mut peak = f64::MIN;
        let mut max_drawdown = 0.0_f64;
        for v in data {
            peak = peak.max(*v);
            max_drawdown = max_drawdown.min(v / peak - 1.0);
        }
        Self {
            win_rate: returns.iter().filter(|r| **r > 0.0).count() as f64 / n,
            sharpe: if std > 0.0 { mean / std * 252f64.sqrt() } else { 0.0 },
            volatility: std * 252f64.sqrt(),
            max_drawdown,
            best: returns.iter().copied().fold(f64::MIN, f64::max),
            worst: returns.iter().copied().fold(f64::MAX, f64::min),
        }
    }
}

// ---- layout helpers ----

fn card<R>(ui: &mut Ui, min_height: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::NONE
        .fill(theme::SURFACE)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(min_height);
            add(ui)
        })
        .inner
}

/// Two cards side by side (`ratio` of the width to the left), stacked when narrow.
fn row(ui: &mut Ui, ratio: f32, left: impl FnOnce(&mut Ui), right: impl FnOnce(&mut Ui)) {
    let w = ui.available_width();
    if w < WIDE {
        left(ui);
        ui.add_space(GAP);
        right(ui);
        return;
    }
    let lw = ((w - GAP) * ratio).floor();
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        ui.allocate_ui_with_layout(vec2(lw, 0.0), Layout::top_down(Align::Min), left);
        ui.allocate_ui_with_layout(vec2(w - GAP - lw, 0.0), Layout::top_down(Align::Min), right);
    });
}

fn kv_row(ui: &mut Ui, label: &str, value: &str, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::hover());
    let p = ui.painter();
    p.text(pos2(rect.left(), rect.center().y), Align2::LEFT_CENTER, label, FontId::proportional(13.0), theme::TEXT_MUTED);
    p.text(pos2(rect.right(), rect.center().y), Align2::RIGHT_CENTER, value, FontId::proportional(13.0), color);
}

fn hairline(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().hline(rect.x_range(), rect.center().y.round() + 0.5, Stroke::new(1.0, theme::BORDER));
}

fn badge(ui: &mut Ui, text: &str) {
    Frame::NONE
        .fill(theme::SURFACE_RAISED)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::symmetric(7, 2))
        .show(ui, |ui| ui.label(RichText::new(text).size(11.5).color(theme::TEXT_MUTED)));
}

fn delta_color(v: f64) -> Color32 {
    if v >= 0.0 { theme::GAIN } else { theme::LOSS }
}

// ---- mock data ----

/// Seeded random walk with upward drift, scaled so today's value is `ENDING_BALANCE`.
fn mock_equity() -> Vec<f64> {
    let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
    let mut uniform = move || {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut v = 100.0;
    let mut out = Vec::with_capacity(HISTORY_DAYS + 1);
    out.push(v);
    for _ in 0..HISTORY_DAYS {
        // Sum of uniforms ≈ normal noise.
        let noise = uniform() + uniform() + uniform() - 1.5;
        v *= 1.0 + 0.0006 + noise * 0.018;
        out.push(v);
    }
    let scale = ENDING_BALANCE / v;
    out.iter_mut().for_each(|x| *x *= scale);
    out
}
