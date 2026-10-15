//! Number and date formatting shared across pages.

pub fn group(int: u64) -> String {
    let s = int.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

pub fn money(v: f64) -> String {
    if v < 0.0 { format!("-${}", price(-v)) } else { format!("${}", price(v)) }
}

pub fn signed_money(v: f64) -> String {
    if v >= 0.0 { format!("+{}", money(v)) } else { money(v) }
}

pub fn signed_pct(v: f64) -> String {
    format!("{}{:.2}%", if v >= 0.0 { "+" } else { "-" }, v.abs())
}

pub fn compact_money(v: f64) -> String {
    if v.abs() >= 1000.0 { format!("${:.0}k", v / 1000.0) } else { format!("${v:.0}") }
}

pub fn trim_qty(q: f64) -> String {
    if q.fract() == 0.0 { format!("{q:.0}") } else { format!("{q}") }
}

/// Rounds the range out to `ticks` evenly spaced "nice" gridlines.
pub fn nice_bounds(lo: f64, hi: f64, ticks: usize) -> (f64, f64, f64) {
    let raw = ((hi - lo).max(1.0)) / ticks as f64;
    let mag = 10f64.powf(raw.log10().floor());
    let step = [1.0, 2.0, 2.5, 5.0, 10.0].iter().map(|m| m * mag).find(|s| *s >= raw).unwrap_or(10.0 * mag);
    ((lo / step).floor() * step, (hi / step).ceil() * step, step)
}

// Dates are days since 1970-01-01.

pub fn today_days() -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    (secs / 86_400) as i64
}

/// (year, month 1-12, day) for a day count. Howard Hinnant's civil_from_days.
pub fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

pub fn short_date(days: i64) -> String {
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let (_, m, d) = civil(days);
    format!("{} {d}", MONTHS[(m - 1) as usize])
}

pub fn iso_date(days: i64) -> String {
    let (y, m, d) = civil(days);
    format!("{y}-{m:02}-{d:02}")
}

/// Price with thousands separators and two decimals, no currency sign.
pub fn price(v: f64) -> String {
    let cents = (v.abs() * 100.0).round() as u64;
    let sign = if v < 0.0 { "-" } else { "" };
    format!("{sign}{}.{:02}", group(cents / 100), cents % 100)
}
