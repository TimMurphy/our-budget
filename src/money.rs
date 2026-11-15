//! Money is integer cents.

/// Parses `1,234.56`, `-$12.3`, `+5` into cents.
pub fn parse_cents(s: &str) -> Option<i64> {
    let mut t: String = s.chars().filter(|c| !matches!(c, '$' | ',' | ' ')).collect();
    let mut neg = false;
    if let Some(rest) = t.strip_prefix('-') {
        neg = true;
        t = rest.to_string();
    } else if let Some(rest) = t.strip_prefix('+') {
        t = rest.to_string();
    }
    if t.is_empty() {
        return None;
    }
    let (whole, frac) = match t.split_once('.') {
        Some((w, f)) => (w, f),
        None => (t.as_str(), ""),
    };
    if frac.len() > 2 || !whole.chars().all(|c| c.is_ascii_digit()) || !frac.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let w: i64 = if whole.is_empty() { 0 } else { whole.parse().ok()? };
    let f: i64 = format!("{frac:0<2}").parse().ok()?;
    let cents = w.checked_mul(100)?.checked_add(f)?;
    Some(if neg { -cents } else { cents })
}

/// Integer division rounded to the nearest, halves away from zero.
pub fn div_round(n: i64, d: i64) -> i64 {
    if n >= 0 {
        (2 * n + d) / (2 * d)
    } else {
        -((2 * -n + d) / (2 * d))
    }
}
