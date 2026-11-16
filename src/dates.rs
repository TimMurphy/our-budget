//! Minimal date helpers. Dates are ISO `YYYY-MM-DD`, months are `YYYY-MM`.

pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

pub fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (y, m, d) = civil_from_days(secs / 86400);
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn parse_iso(s: &str) -> Option<(i64, i64, i64)> {
    let p: Vec<&str> = s.split('-').collect();
    if p.len() != 3 || p[0].len() != 4 || p[1].len() != 2 || p[2].len() != 2 {
        return None;
    }
    if !p.iter().all(|x| x.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }
    let (y, m, d) = (p[0].parse().ok()?, p[1].parse().ok()?, p[2].parse().ok()?);
    valid(y, m, d).then_some((y, m, d))
}

fn valid(y: i64, m: i64, d: i64) -> bool {
    if !(1..=12).contains(&m) || d < 1 || !(1901..=2999).contains(&y) {
        return false;
    }
    let days = match m {
        2 if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    d <= days
}

pub fn day_number(date: &str) -> Option<i64> {
    parse_iso(date).map(|(y, m, d)| days_from_civil(y, m, d))
}

pub fn month_of(date: &str) -> String {
    date.chars().take(7).collect()
}

/// A canonical `YYYY-MM` (seven characters), so month strings compare and step reliably.
pub fn is_month(s: &str) -> bool {
    s.len() == 7 && parse_iso(&format!("{s}-01")).is_some()
}

pub fn next_month(m: &str) -> String {
    shift(m, 1)
}

pub fn prev_month(m: &str) -> String {
    shift(m, -1)
}

fn shift(m: &str, by: i64) -> String {
    let (y, mo, _) = parse_iso(&format!("{m}-01")).unwrap_or((1970, 1, 1));
    let idx = y * 12 + (mo - 1) + by;
    format!("{:04}-{:02}", idx.div_euclid(12), idx.rem_euclid(12) + 1)
}

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

pub fn month_abbrev(m: i64) -> &'static str {
    MONTHS[(m - 1) as usize]
}

/// Parses `YYYY-MM-DD`, `DD/MM/YYYY`, `D/M/YY` and `DD Mon YY[YY]` into ISO.
pub fn parse_date(s: &str) -> Option<String> {
    let s = s.trim();
    if let Some((y, m, d)) = parse_iso(s) {
        return Some(format!("{y:04}-{m:02}-{d:02}"));
    }
    let year = |t: &str| -> Option<i64> {
        let y: i64 = t.parse().ok()?;
        Some(if t.len() <= 2 { 2000 + y } else { y })
    };
    let parts: Vec<&str> = s.split('/').collect();
    let (d, m, y) = if parts.len() == 3 {
        (parts[0].parse().ok()?, parts[1].parse().ok()?, year(parts[2])?)
    } else {
        let p: Vec<&str> = s.split_whitespace().collect();
        if p.len() != 3 {
            return None;
        }
        let mon = p[1].to_lowercase();
        let m = MONTHS.iter().position(|x| mon.starts_with(x))? as i64 + 1;
        (p[0].parse().ok()?, m, year(p[2])?)
    };
    valid(y, m, d).then(|| format!("{y:04}-{m:02}-{d:02}"))
}
