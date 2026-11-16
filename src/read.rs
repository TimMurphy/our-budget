//! Queries over the read model tables.

use crate::dates::{next_month, prev_month};
use crate::model::{monthly_cents, Split, INCOME};
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::collections::HashMap;

pub type R<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
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
    /// Assigned by an auto assign rule and not yet approved.
    pub pending: bool,
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
    pub file: String,
    pub row: i64,
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

#[derive(Clone, Debug)]
pub struct Rule {
    pub id: String,
    pub phrase: String,
    pub account: String,
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

// ---------- bank accounts ----------

const BANK_COLS: &str = "SELECT id, name, bank, file, opening_cents FROM bank_accounts";

fn bank_row(r: &Row) -> rusqlite::Result<BankAccount> {
    Ok(BankAccount { id: r.get(0)?, name: r.get(1)?, bank: r.get(2)?, file: r.get(3)?, opening_cents: r.get(4)? })
}

pub fn bank_accounts(c: &Connection) -> R<Vec<BankAccount>> {
    let mut s = c.prepare(&format!("{BANK_COLS} ORDER BY rowid")).map_err(err)?;
    let rows = s.query_map([], bank_row).map_err(err)?;
    rows.collect::<Result<_, _>>().map_err(err)
}

/// Finds a bank account by id or by name, ignoring case.
pub fn bank_account(c: &Connection, key: &str) -> R<Option<BankAccount>> {
    c.query_row(&format!("{BANK_COLS} WHERE id = ?1 OR lower(name) = lower(?1)"), params![key], bank_row)
        .optional()
        .map_err(err)
}

pub fn balance(c: &Connection, account: &str) -> R<i64> {
    c.query_row(
        "SELECT COALESCE((SELECT opening_cents FROM bank_accounts WHERE id = ?1), 0)
              + COALESCE((SELECT SUM(cents) FROM transactions WHERE account = ?1), 0)",
        params![account],
        |r| r.get(0),
    )
    .map_err(err)
}

// ---------- transactions ----------

fn load_txns(c: &Connection, account: Option<&str>, id: Option<&str>) -> R<Vec<Txn>> {
    let mut s = c
        .prepare(
            "SELECT id, account, date, cents, description, link, pending FROM transactions
             WHERE (?1 IS NULL OR account = ?1) AND (?2 IS NULL OR id = ?2) ORDER BY rowid",
        )
        .map_err(err)?;
    let rows = s
        .query_map(params![account, id], |r| {
            Ok(Txn {
                id: r.get(0)?,
                account: r.get(1)?,
                date: r.get(2)?,
                cents: r.get(3)?,
                description: r.get(4)?,
                splits: vec![],
                link: r.get(5)?,
                pending: r.get::<_, i64>(6)? != 0,
            })
        })
        .map_err(err)?;
    let mut txns: Vec<Txn> = rows.collect::<Result<_, _>>().map_err(err)?;

    let mut s = c
        .prepare(
            "SELECT a.transaction_id, a.budget_account, a.cents FROM assignments a
             JOIN transactions t ON t.id = a.transaction_id
             WHERE (?1 IS NULL OR t.account = ?1) AND (?2 IS NULL OR t.id = ?2) ORDER BY a.rowid",
        )
        .map_err(err)?;
    let rows = s
        .query_map(params![account, id], |r| Ok((r.get::<_, String>(0)?, Split { account: r.get(1)?, cents: r.get(2)? })))
        .map_err(err)?;
    let mut by_txn: HashMap<String, Vec<Split>> = HashMap::new();
    for r in rows {
        let (txn, split) = r.map_err(err)?;
        by_txn.entry(txn).or_default().push(split);
    }
    for t in &mut txns {
        t.splits = by_txn.remove(&t.id).unwrap_or_default();
    }
    Ok(txns)
}

pub fn txn(c: &Connection, id: &str) -> R<Option<Txn>> {
    Ok(load_txns(c, None, Some(id))?.into_iter().next())
}

/// In the order they were imported.
pub fn txns_for_account(c: &Connection, account: &str) -> R<Vec<Txn>> {
    load_txns(c, Some(account), None)
}

pub fn all_txns(c: &Connection) -> R<Vec<Txn>> {
    load_txns(c, None, None)
}

pub fn txn_exists(c: &Connection, id: &str) -> R<bool> {
    c.query_row("SELECT EXISTS (SELECT 1 FROM transactions WHERE id = ?1)", params![id], |r| r.get(0)).map_err(err)
}

pub fn failures(c: &Connection, account: &str) -> R<Vec<Failure>> {
    let mut s = c.prepare("SELECT file, row, reason FROM import_failures WHERE account = ?1 ORDER BY rowid").map_err(err)?;
    let rows = s.query_map(params![account], |r| Ok(Failure { file: r.get(0)?, row: r.get(1)?, reason: r.get(2)? })).map_err(err)?;
    rows.collect::<Result<_, _>>().map_err(err)
}

pub fn failure_exists(c: &Connection, account: &str, row: usize, reason: &str) -> R<bool> {
    c.query_row(
        "SELECT EXISTS (SELECT 1 FROM import_failures WHERE account = ?1 AND row = ?2 AND reason = ?3)",
        params![account, row as i64, reason],
        |r| r.get(0),
    )
    .map_err(err)
}

// ---------- groups and budget accounts ----------

pub fn groups(c: &Connection) -> R<Vec<Group>> {
    let mut s = c.prepare("SELECT id, name FROM groups ORDER BY position, rowid").map_err(err)?;
    let rows = s.query_map([], |r| Ok(Group { id: r.get(0)?, name: r.get(1)? })).map_err(err)?;
    rows.collect::<Result<_, _>>().map_err(err)
}

/// Finds a group by id or by name, ignoring case.
pub fn group(c: &Connection, key: &str) -> R<Option<Group>> {
    c.query_row(
        "SELECT id, name FROM groups WHERE id = ?1 OR lower(name) = lower(?1)",
        params![key],
        |r| Ok(Group { id: r.get(0)?, name: r.get(1)? }),
    )
    .optional()
    .map_err(err)
}

pub fn group_by_id(c: &Connection, id: &str) -> R<Option<Group>> {
    c.query_row("SELECT id, name FROM groups WHERE id = ?1", params![id], |r| Ok(Group { id: r.get(0)?, name: r.get(1)? }))
        .optional()
        .map_err(err)
}

const BUDGET_COLS: &str = "SELECT id, name, group_id, closed, auto_period, auto_cents FROM budget_accounts";

fn budget_row(r: &Row) -> rusqlite::Result<BudgetAccount> {
    let period: Option<String> = r.get(4)?;
    let cents: i64 = r.get(5)?;
    Ok(BudgetAccount {
        id: r.get(0)?,
        name: r.get(1)?,
        group: r.get(2)?,
        closed: r.get::<_, i64>(3)? != 0,
        auto: period.map(|p| (p, cents)),
    })
}

pub fn budget_accounts(c: &Connection) -> R<Vec<BudgetAccount>> {
    let mut s = c.prepare(&format!("{BUDGET_COLS} ORDER BY rowid")).map_err(err)?;
    let rows = s.query_map([], budget_row).map_err(err)?;
    rows.collect::<Result<_, _>>().map_err(err)
}

fn budget_matches(c: &Connection, key: &str) -> R<Vec<BudgetAccount>> {
    let mut s = c
        .prepare(&format!("{BUDGET_COLS} WHERE id = ?1 OR lower(name) = lower(?1) ORDER BY rowid"))
        .map_err(err)?;
    let rows = s.query_map(params![key], budget_row).map_err(err)?;
    rows.collect::<Result<_, _>>().map_err(err)
}

/// Finds a budget account by id, or else by name ignoring case. Names are only unique within a group,
/// so a name used by more than one open account is an error: the caller must use the id.
pub fn budget_account(c: &Connection, key: &str) -> R<Option<BudgetAccount>> {
    let matches = budget_matches(c, key)?;
    if let Some(a) = matches.iter().find(|a| a.id == key) {
        return Ok(Some(a.clone()));
    }
    let open: Vec<&BudgetAccount> = matches.iter().filter(|a| !a.closed).collect();
    match open.len() {
        0 => Ok(matches.into_iter().next()),
        1 => Ok(Some(open[0].clone())),
        _ => Err(format!("more than one budget account is named {key}; choose the one in the group you want")),
    }
}

pub fn budget_account_by_id(c: &Connection, id: &str) -> R<Option<BudgetAccount>> {
    c.query_row(&format!("{BUDGET_COLS} WHERE id = ?1"), params![id], budget_row).optional().map_err(err)
}

pub fn account_name(c: &Connection, id: &str) -> R<String> {
    Ok(budget_account_by_id(c, id)?.map(|a| a.name).unwrap_or_else(|| id.to_string()))
}

// ---------- auto assign rules ----------

/// In the order they were made. The first rule that matches a transaction wins.
/// How many rules have ever been made, including removed ones. Events are never deleted, so this only
/// grows, which makes `rule-{n}` a unique id.
pub fn rules_ever_created(c: &Connection) -> R<i64> {
    c.query_row("SELECT COUNT(*) FROM events WHERE type = 'AutoAssignRuleCreated'", [], |r| r.get(0)).map_err(err)
}

/// Whether an opening balance has been recorded for the bank account.
pub fn opening_balance_set(c: &Connection, account: &str) -> R<bool> {
    c.query_row(
        "SELECT EXISTS (SELECT 1 FROM events WHERE type = 'OpeningBalanceSet' AND json_extract(data, '$.account') = ?1)",
        params![account],
        |r| r.get(0),
    )
    .map_err(err)
}

/// (unassigned, pending approval) transaction counts for each bank account, counted in SQL.
pub fn account_counts(c: &Connection) -> R<HashMap<String, (i64, i64)>> {
    let mut s = c
        .prepare(
            "SELECT t.account,
                    SUM(CASE WHEN t.link IS NULL AND NOT (
                            EXISTS (SELECT 1 FROM assignments a WHERE a.transaction_id = t.id)
                            AND (SELECT SUM(a.cents) FROM assignments a WHERE a.transaction_id = t.id) = t.cents
                        ) THEN 1 ELSE 0 END),
                    SUM(t.pending)
             FROM transactions t GROUP BY t.account",
        )
        .map_err(err)?;
    let rows = s.query_map([], |r| Ok((r.get::<_, String>(0)?, (r.get::<_, i64>(1)?, r.get::<_, i64>(2)?)))).map_err(err)?;
    rows.collect::<Result<_, _>>().map_err(err)
}

pub fn auto_rules(c: &Connection) -> R<Vec<Rule>> {
    let mut s = c.prepare("SELECT id, phrase, account FROM auto_assign_rules ORDER BY rowid").map_err(err)?;
    let rows = s.query_map([], |r| Ok(Rule { id: r.get(0)?, phrase: r.get(1)?, account: r.get(2)? })).map_err(err)?;
    rows.collect::<Result<_, _>>().map_err(err)
}

// ---------- the budget ----------

fn effective_budget(a: &BudgetAccount, budgets: &HashMap<(String, String), i64>, month: &str) -> (i64, bool) {
    if let Some(v) = budgets.get(&(a.id.clone(), month.to_string())) {
        return (*v, true);
    }
    match &a.auto {
        Some((period, cents)) => (monthly_cents(period, *cents).unwrap_or(0), false),
        None => (0, false),
    }
}

pub fn month_report(c: &Connection, month: &str) -> R<MonthReport> {
    let mut spent: HashMap<(String, String), i64> = HashMap::new();
    let mut s = c
        .prepare(
            "SELECT a.budget_account, substr(t.date, 1, 7), SUM(a.cents) FROM assignments a
             JOIN transactions t ON t.id = a.transaction_id GROUP BY 1, 2",
        )
        .map_err(err)?;
    for r in s.query_map([], |r| Ok(((r.get::<_, String>(0)?, r.get::<_, String>(1)?), r.get::<_, i64>(2)?))).map_err(err)? {
        let (k, v) = r.map_err(err)?;
        spent.insert(k, v);
    }
    let mut budgets: HashMap<(String, String), i64> = HashMap::new();
    let mut s = c.prepare("SELECT account, month, cents FROM budgets").map_err(err)?;
    for r in s.query_map([], |r| Ok(((r.get::<_, String>(0)?, r.get::<_, String>(1)?), r.get::<_, i64>(2)?))).map_err(err)? {
        let (k, v) = r.map_err(err)?;
        budgets.insert(k, v);
    }

    let mut start = month.to_string();
    for (_, m) in spent.keys().chain(budgets.keys()) {
        if *m < start {
            start = m.clone();
        }
    }
    let accounts: Vec<BudgetAccount> = budget_accounts(c)?.into_iter().filter(|a| a.id != INCOME && !a.closed).collect();
    let mut prev_avail: HashMap<String, i64> = HashMap::new();
    let mut m = start;
    loop {
        let overspent: i64 = -prev_avail.values().map(|v| (*v).min(0)).sum::<i64>();
        let mut rows = HashMap::new();
        let mut total = 0;
        for a in &accounts {
            let (budgeted, budget_set) = effective_budget(a, &budgets, &m);
            let sp = spent.get(&(a.id.clone(), m.clone())).copied().unwrap_or(0);
            let carry = prev_avail.get(&a.id).copied().unwrap_or(0).max(0);
            total += budgeted;
            rows.insert(a.id.clone(), AccountRow { budgeted, budget_set, spent: sp, available: carry + budgeted + sp });
        }
        let prev_income = spent.get(&(INCOME.to_string(), prev_month(&m))).copied().unwrap_or(0);
        if m == month {
            return Ok(MonthReport { rows, prev_income, overspent, total_budgeted: total, left_to_budget: prev_income - overspent - total });
        }
        prev_avail = rows.into_iter().map(|(k, v)| (k, v.available)).collect();
        m = next_month(&m);
    }
}
