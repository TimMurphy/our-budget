//! Events, and the small pure helpers shared by the projection and the queries.

use crate::money::div_round;
use serde::{Deserialize, Serialize};

pub const INCOME: &str = "income";
pub const UNGROUPED: &str = "ungrouped";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Split {
    pub account: String,
    pub cents: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    /// `opening_cents` is only read, from events stored before opening balances became their own event.
    BankAccountRegistered {
        id: String,
        name: String,
        bank: String,
        file: String,
        #[serde(default, skip_serializing)]
        opening_cents: i64,
    },
    OpeningBalanceSet { account: String, cents: i64 },
    TransactionImported { id: String, account: String, date: String, cents: i64, description: String },
    ImportRowFailed { account: String, file: String, row: usize, reason: String },
    GroupAdded { id: String, name: String },
    GroupRenamed { id: String, name: String },
    GroupRemoved { id: String },
    GroupsReordered { ids: Vec<String> },
    BudgetAccountCreated { id: String, name: String, group: String },
    BudgetAccountRenamed { id: String, name: String },
    BudgetAccountClosed { id: String },
    BudgetAccountMoved { id: String, group: String },
    Assigned { transaction: String, splits: Vec<Split> },
    TransferLinked { a: String, b: String },
    TransferUnlinked { a: String, b: String },
    BudgetSet { account: String, month: String, cents: i64 },
    AutoMonthlySet { account: String, period: Option<String>, cents: i64 },
    AutoAssignRuleCreated { id: String, phrase: String, account: String },
    AutoAssignRuleRemoved { id: String },
    /// The whole transaction assigned to `account` by a rule. It stays pending approval until approved.
    AutoAssigned { transaction: String, account: String, cents: i64, rule: String },
    TransactionApproved { transaction: String },
}

impl Event {
    pub fn kind(&self) -> String {
        serde_json::to_value(self)
            .ok()
            .and_then(|v| v["type"].as_str().map(String::from))
            .unwrap_or_default()
    }
}

pub fn slug(name: &str) -> String {
    let s: String = name.to_lowercase().chars().map(|c| if c.is_alphanumeric() { c } else { '-' }).collect();
    s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-")
}

/// Converts an auto monthly amount for a period into a monthly amount.
pub fn monthly_cents(period: &str, cents: i64) -> Option<i64> {
    Some(match period {
        "Yearly" => div_round(cents, 12),
        "Quarterly" => div_round(cents, 3),
        "Monthly" => cents,
        "Fortnight" => div_round(cents * 26, 12),
        "Weekly" => div_round(cents * 52, 12),
        _ => return None,
    })
}
