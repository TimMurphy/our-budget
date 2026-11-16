//! Registers the household's bank accounts, reading opening balances from the uncommitted secrets file.

use crate::app::App;
use crate::mediator::Mediator;
use crate::money::parse_cents;
use serde_json::json;
use std::path::Path;

pub const ACCOUNTS: [(&str, &str, &str); 3] = [
    ("NAB - Home Loan", "NAB", "NAB - Home Loan.csv"),
    ("NAB - Nicole's Account", "NAB", "NAB - Nicole's Account.csv"),
    ("Westpac - Our Account", "Westpac", "Westpac - Our Account.csv"),
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
    let secrets = std::fs::read_to_string(secrets_path).unwrap_or_default();
    for (name, bank, file) in ACCOUNTS {
        let body = json!({ "name": name, "bank": bank, "file": file });
        mediator.send(app, "RegisterBankAccount", &body)?;
        // An account's opening balance is recorded once, the first time the secrets file has one for it.
        // After that the events are the truth, and the file is not read for that account again.
        let cents = opening_balance(&secrets, name);
        if cents != 0 {
            match mediator.send(app, "SetOpeningBalance", &json!({ "account": name, "cents": cents })) {
                Ok(_) => {}
                Err(e) if e.contains("already set") => {}
                Err(e) => return Err(e),
            }
        }
    }
    Ok(())
}
