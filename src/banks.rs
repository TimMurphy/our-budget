//! Known banks. Each parses its own downloaded CSV format.

use crate::dates::parse_date;
use crate::money::parse_cents;

pub const BANKS: [&str; 2] = ["NAB", "Westpac"];

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub date: String,
    pub cents: i64,
    pub description: String,
}

pub type RowResult = Result<Row, (usize, String)>;

pub fn split_csv_line(line: &str) -> Vec<String> {
    let mut fields = vec![];
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => fields.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    fields.push(cur);
    fields.into_iter().map(|f| f.trim().to_string()).collect()
}

struct Cols {
    date: usize,
    amount: Option<usize>,
    debit: Option<usize>,
    credit: Option<usize>,
    desc: usize,
}

fn positional(bank: &str) -> Cols {
    if bank == "NAB" {
        Cols { date: 0, amount: Some(1), debit: None, credit: None, desc: 5 }
    } else {
        Cols { date: 1, amount: None, debit: Some(3), credit: Some(4), desc: 2 }
    }
}

fn from_header(bank: &str, header: &[String]) -> Cols {
    let find = |names: &[&str]| header.iter().position(|h| names.contains(&h.to_lowercase().as_str()));
    let base = positional(bank);
    let nab = bank == "NAB";
    Cols {
        date: find(&["date"]).unwrap_or(base.date),
        amount: if nab { find(&["amount"]).or(base.amount) } else { None },
        debit: if nab { None } else { find(&["debit amount", "debit"]).or(base.debit) },
        credit: if nab { None } else { find(&["credit amount", "credit"]).or(base.credit) },
        desc: find(&["transaction details", "narrative", "description"]).unwrap_or(base.desc),
    }
}

/// Parses a downloaded statement. Rows that cannot be parsed come back as
/// `Err((line number, reason))` so the caller can carry on with the rest.
pub fn parse(bank: &str, text: &str) -> Vec<RowResult> {
    let mut cols = positional(bank);
    let mut out = vec![];
    let mut first = true;
    for (i, line) in text.lines().enumerate() {
        let row = i + 1;
        if line.trim().is_empty() {
            continue;
        }
        let f = split_csv_line(line);
        if first {
            first = false;
            if !f.iter().any(|x| parse_date(x).is_some()) {
                cols = from_header(bank, &f);
                continue;
            }
        }
        out.push(parse_row(&cols, &f).map_err(|e| (row, e)));
    }
    out
}

fn parse_row(c: &Cols, f: &[String]) -> Result<Row, String> {
    let get = |i: usize| f.get(i).map(|s| s.as_str()).unwrap_or("");
    let date = parse_date(get(c.date)).ok_or("bad date")?;
    let cents = match c.amount {
        Some(i) => parse_cents(get(i)).ok_or("bad amount")?,
        None => {
            let side = |i: Option<usize>| -> Result<Option<i64>, String> {
                let s = i.map(get).unwrap_or("");
                if s.is_empty() {
                    Ok(None)
                } else {
                    parse_cents(s).map(Some).ok_or_else(|| "bad amount".to_string())
                }
            };
            match (side(c.debit)?, side(c.credit)?) {
                (None, None) => return Err("no amount".into()),
                (debit, credit) => credit.unwrap_or(0) - debit.unwrap_or(0),
            }
        }
    };
    let description = get(c.desc).split_whitespace().collect::<Vec<_>>().join(" ");
    Ok(Row { date, cents, description })
}
