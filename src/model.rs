//! Events and the read model rebuilt from them.

use crate::dates::{month_of, next_month, prev_month};
use crate::money::div_round;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    BankAccountRegistered { id: String, name: String, bank: String, file: String, opening_cents: i64 },
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
}

impl Event {
    pub fn kind(&self) -> String {
        serde_json::to_value(self)
            .ok()
            .and_then(|v| v["type"].as_str().map(String::from))
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug)]
pub struct BankAccount {
    pub id: String,
    pub name: String,
    pub bank: String,
    pub file: String,
    pub opening_cents: i64,
}

#[derive(Clone, Debug)]
pub struct Txn {
    pub id: String,
    pub account: String,
    pub date: String,
    pub cents: i64,
    pub description: String,
    pub splits: Vec<Split>,
    pub link: Option<String>,
}

impl Txn {
    pub fn assigned_cents(&self) -> i64 {
        self.splits.iter().map(|s| s.cents).sum()
    }
    /// Linked as a transfer, or fully assigned.
    pub fn is_assigned(&self) -> bool {
        self.link.is_some() || (!self.splits.is_empty() && self.assigned_cents() == self.cents)
    }
}

#[derive(Clone, Debug)]
pub struct Failure {
    pub account: String,
    pub file: String,
    pub row: usize,
    pub reason: String,
}

#[derive(Clone, Debug)]
pub struct Group {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct BudgetAccount {
    pub id: String,
    pub name: String,
    pub group: String,
    pub closed: bool,
    pub auto: Option<(String, i64)>,
}

#[derive(Debug)]
pub struct Model {
    pub bank_accounts: Vec<BankAccount>,
    pub txns: Vec<Txn>,
    pub txn_index: HashMap<String, usize>,
    pub failures: Vec<Failure>,
    pub groups: Vec<Group>,
    pub budget_accounts: Vec<BudgetAccount>,
    pub budgets: HashMap<(String, String), i64>,
}

pub struct AccountRow {
    pub budgeted: i64,
    pub budget_set: bool,
    pub spent: i64,
    pub available: i64,
}

pub struct MonthReport {
    pub rows: HashMap<String, AccountRow>,
    pub prev_income: i64,
    /// Positive amount overspent last month, taken from this month's income.
    pub overspent: i64,
    pub total_budgeted: i64,
    pub left_to_budget: i64,
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

impl Default for Model {
    fn default() -> Self {
        Model {
            bank_accounts: vec![],
            txns: vec![],
            txn_index: HashMap::new(),
            failures: vec![],
            groups: vec![Group { id: UNGROUPED.into(), name: "Ungrouped".into() }],
            budget_accounts: vec![BudgetAccount {
                id: INCOME.into(),
                name: "Income".into(),
                group: String::new(),
                closed: false,
                auto: None,
            }],
            budgets: HashMap::new(),
        }
    }
}

impl Model {
    pub fn apply(&mut self, e: &Event) {
        match e {
            Event::BankAccountRegistered { id, name, bank, file, opening_cents } => {
                self.bank_accounts.push(BankAccount {
                    id: id.clone(),
                    name: name.clone(),
                    bank: bank.clone(),
                    file: file.clone(),
                    opening_cents: *opening_cents,
                });
            }
            Event::TransactionImported { id, account, date, cents, description } => {
                self.txn_index.insert(id.clone(), self.txns.len());
                self.txns.push(Txn {
                    id: id.clone(),
                    account: account.clone(),
                    date: date.clone(),
                    cents: *cents,
                    description: description.clone(),
                    splits: vec![],
                    link: None,
                });
            }
            Event::ImportRowFailed { account, file, row, reason } => self.failures.push(Failure {
                account: account.clone(),
                file: file.clone(),
                row: *row,
                reason: reason.clone(),
            }),
            Event::GroupAdded { id, name } => self.groups.push(Group { id: id.clone(), name: name.clone() }),
            Event::GroupRenamed { id, name } => {
                if let Some(g) = self.groups.iter_mut().find(|g| &g.id == id) {
                    g.name = name.clone();
                }
            }
            Event::GroupRemoved { id } => self.groups.retain(|g| &g.id != id),
            Event::GroupsReordered { ids } => {
                let mut ordered: Vec<Group> = vec![];
                for id in ids {
                    if let Some(g) = self.groups.iter().find(|g| &g.id == id) {
                        ordered.push(g.clone());
                    }
                }
                for g in &self.groups {
                    if !ordered.iter().any(|o| o.id == g.id) {
                        ordered.push(g.clone());
                    }
                }
                self.groups = ordered;
            }
            Event::BudgetAccountCreated { id, name, group } => self.budget_accounts.push(BudgetAccount {
                id: id.clone(),
                name: name.clone(),
                group: group.clone(),
                closed: false,
                auto: None,
            }),
            Event::BudgetAccountRenamed { id, name } => {
                if let Some(a) = self.budget_accounts.iter_mut().find(|a| &a.id == id) {
                    a.name = name.clone();
                }
            }
            Event::BudgetAccountClosed { id } => {
                if let Some(a) = self.budget_accounts.iter_mut().find(|a| &a.id == id) {
                    a.closed = true;
                }
            }
            Event::BudgetAccountMoved { id, group } => {
                if let Some(a) = self.budget_accounts.iter_mut().find(|a| &a.id == id) {
                    a.group = group.clone();
                }
            }
            Event::Assigned { transaction, splits } => {
                if let Some(&i) = self.txn_index.get(transaction) {
                    self.txns[i].splits = splits.clone();
                }
            }
            Event::TransferLinked { a, b } => {
                self.set_link(a, Some(b.clone()));
                self.set_link(b, Some(a.clone()));
            }
            Event::TransferUnlinked { a, b } => {
                self.set_link(a, None);
                self.set_link(b, None);
            }
            Event::BudgetSet { account, month, cents } => {
                self.budgets.insert((account.clone(), month.clone()), *cents);
            }
            Event::AutoMonthlySet { account, period, cents } => {
                if let Some(a) = self.budget_accounts.iter_mut().find(|a| &a.id == account) {
                    a.auto = period.clone().map(|p| (p, *cents));
                }
            }
        }
    }

    fn set_link(&mut self, id: &str, link: Option<String>) {
        if let Some(&i) = self.txn_index.get(id) {
            self.txns[i].link = link;
        }
    }

    pub fn txn(&self, id: &str) -> Option<&Txn> {
        self.txn_index.get(id).map(|&i| &self.txns[i])
    }

    pub fn bank_account(&self, id: &str) -> Option<&BankAccount> {
        self.bank_accounts.iter().find(|a| a.id == id || a.name.eq_ignore_ascii_case(id))
    }

    pub fn group(&self, id: &str) -> Option<&Group> {
        self.groups.iter().find(|g| g.id == id || g.name.eq_ignore_ascii_case(id))
    }

    pub fn budget_account(&self, id: &str) -> Option<&BudgetAccount> {
        self.budget_accounts.iter().find(|a| a.id == id || a.name.eq_ignore_ascii_case(id))
    }

    pub fn account_name(&self, id: &str) -> String {
        self.budget_account(id).map(|a| a.name.clone()).unwrap_or_else(|| id.to_string())
    }

    pub fn balance(&self, account: &str) -> i64 {
        let opening = self.bank_account(account).map(|a| a.opening_cents).unwrap_or(0);
        opening + self.txns.iter().filter(|t| t.account == account).map(|t| t.cents).sum::<i64>()
    }

    fn effective_budget(&self, a: &BudgetAccount, month: &str) -> (i64, bool) {
        if let Some(v) = self.budgets.get(&(a.id.clone(), month.to_string())) {
            return (*v, true);
        }
        match &a.auto {
            Some((period, cents)) => (monthly_cents(period, *cents).unwrap_or(0), false),
            None => (0, false),
        }
    }

    pub fn month_report(&self, month: &str) -> MonthReport {
        let mut spent: HashMap<(String, String), i64> = HashMap::new();
        for t in &self.txns {
            for s in &t.splits {
                *spent.entry((s.account.clone(), month_of(&t.date))).or_default() += s.cents;
            }
        }
        let mut start = month.to_string();
        for (_, m) in spent.keys().chain(self.budgets.keys()) {
            if *m < start {
                start = m.clone();
            }
        }
        let accounts: Vec<&BudgetAccount> = self.budget_accounts.iter().filter(|a| a.id != INCOME).collect();
        let mut prev_avail: HashMap<String, i64> = HashMap::new();
        let mut m = start;
        loop {
            let overspent: i64 = -prev_avail.values().map(|v| (*v).min(0)).sum::<i64>();
            let mut rows = HashMap::new();
            let mut total = 0;
            for a in &accounts {
                let (budgeted, budget_set) = self.effective_budget(a, &m);
                let sp = spent.get(&(a.id.clone(), m.clone())).copied().unwrap_or(0);
                let carry = prev_avail.get(&a.id).copied().unwrap_or(0).max(0);
                total += budgeted;
                rows.insert(a.id.clone(), AccountRow { budgeted, budget_set, spent: sp, available: carry + budgeted + sp });
            }
            let prev_income = spent.get(&(INCOME.to_string(), prev_month(&m))).copied().unwrap_or(0);
            if m == month {
                return MonthReport {
                    rows,
                    prev_income,
                    overspent,
                    total_budgeted: total,
                    left_to_budget: prev_income - overspent - total,
                };
            }
            prev_avail = rows.into_iter().map(|(k, v)| (k, v.available)).collect();
            m = next_month(&m);
        }
    }
}
