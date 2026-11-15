//! Registers the household's bank accounts, reading opening balances from the uncommitted secrets file.

use crate::app::App;
use crate::mediator::Mediator;
use crate::money::parse_cents;
use serde_json::json;
use std::path::Path;

pub const ACCOUNTS: [(&str, &str, &str); 3] = [
    ("NAB - Home Loan", "NAB", "home-loan.csv"),
    ("NAB - Nicole's Account", "NAB", "nicole.csv"),
    ("Westpac - Our Joint Account", "Westpac", "joint.csv"),
];

/// Looks up `- <account name>: <amount>` in the secrets file.
fn opening_balance(secrets: &str, name: &str) -> i64 {
    secrets
        .lines()
        .filter_map(|l| l.trim().strip_prefix('-'))
        .filter_map(|l| l.split_once(':'))
        .find(|(n, _)| n.trim().eq_ignore_ascii_case(name))
        .and_then(|(_, amount)| parse_cents(amount.trim()))
        .unwrap_or(0)
}

pub fn register_accounts(app: &mut App, mediator: &Mediator, secrets_path: &Path) -> Result<(), String> {
    // Opening balances are recorded only into an empty database. After that the events are the truth.
    let secrets = if app.store.is_empty()? { std::fs::read_to_string(secrets_path).unwrap_or_default() } else { String::new() };
    for (name, bank, file) in ACCOUNTS {
        let body = json!({ "name": name, "bank": bank, "file": file, "opening_cents": opening_balance(&secrets, name) });
        mediator.send(app, "RegisterBankAccount", &body)?;
    }
    Ok(())
}
