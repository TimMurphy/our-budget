//! Commands and queries. Commands validate against the read model tables, then append events.

use crate::app::App;
use crate::banks;
use crate::dates::{day_number, is_month, month_of};
use crate::mediator::Handler;
use crate::model::{monthly_cents, slug, Event, Split, INCOME, UNGROUPED};
use crate::read;
use rusqlite::Connection;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

type Res = Result<Value, String>;

/// Transfers must be this many days apart or fewer.
pub const TRANSFER_DAYS: i64 = 5;
pub const PERIODS: [&str; 5] = ["Yearly", "Quarterly", "Monthly", "Fortnight", "Weekly"];

pub fn all() -> Vec<(&'static str, Handler)> {
    vec![
        ("RegisterBankAccount", register_bank_account),
        ("ListBankAccounts", list_bank_accounts),
        ("ImportStatements", import_statements),
        ("GetTransactions", get_transactions),
        ("GetTransferCandidates", get_transfer_candidates),
        ("AssignTransactions", assign_transactions),
        ("LinkTransfer", link_transfer),
        ("UnlinkTransfer", unlink_transfer),
        ("GetBudget", get_budget),
        ("AddGroup", add_group),
        ("RenameGroup", rename_group),
        ("RemoveGroup", remove_group),
        ("ReorderGroups", reorder_groups),
        ("CreateBudgetAccount", create_budget_account),
        ("RenameBudgetAccount", rename_budget_account),
        ("CloseBudgetAccount", close_budget_account),
        ("MoveBudgetAccount", move_budget_account),
        ("SetBudgeted", set_budgeted),
        ("SetAutoMonthly", set_auto_monthly),
    ]
}

fn text(v: &Value, k: &str) -> Result<String, String> {
    v.get(k)
        .and_then(|x| x.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("{k} is required"))
}

fn opt_text(v: &Value, k: &str) -> Option<String> {
    text(v, k).ok()
}

fn int(v: &Value, k: &str) -> Result<i64, String> {
    v.get(k).and_then(|x| x.as_i64()).ok_or_else(|| format!("{k} is required"))
}

fn ok() -> Res {
    Ok(json!({}))
}

// ---------- bank accounts ----------

fn register_bank_account(app: &mut App, v: &Value) -> Res {
    let name = text(v, "name")?;
    let bank = text(v, "bank")?;
    if !banks::BANKS.contains(&bank.as_str()) {
        return Err(format!("unknown bank {bank}"));
    }
    let id = slug(&name);
    if read::bank_account(app.store.conn(), &id)?.is_some() {
        return ok();
    }
    let file = opt_text(v, "file").unwrap_or_else(|| format!("{id}.csv"));
    let opening_cents = v.get("opening_cents").and_then(|x| x.as_i64()).unwrap_or(0);
    let mut events = vec![Event::BankAccountRegistered { id: id.clone(), name, bank, file }];
    if opening_cents != 0 {
        events.push(Event::OpeningBalanceSet { account: id.clone(), cents: opening_cents });
    }
    app.record(events)?;
    Ok(json!({ "id": id }))
}

fn list_bank_accounts(app: &mut App, _: &Value) -> Res {
    let c = app.store.conn();
    let txns = read::all_txns(c)?;
    let mut total = 0;
    let mut accounts = vec![];
    for a in read::bank_accounts(c)? {
        let balance = read::balance(c, &a.id)?;
        total += balance;
        let unassigned = txns.iter().filter(|t| t.account == a.id && !t.is_assigned()).count();
        accounts.push(json!({ "id": a.id, "name": a.name, "bank": a.bank, "balance_cents": balance, "unassigned": unassigned }));
    }
    Ok(json!({ "accounts": accounts, "total_cents": total }))
}

fn import_statements(app: &mut App, _: &Value) -> Res {
    let mut imported = 0;
    let from = app.import_from.clone();
    for acct in read::bank_accounts(app.store.conn())? {
        let Ok(content) = std::fs::read_to_string(app.statements_dir.join(&acct.file)) else {
            continue;
        };
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut events = vec![];
        for r in banks::parse(&acct.bank, &content) {
            match r {
                Ok(row) if row.date.as_str() < from.as_str() => {}
                Ok(row) => {
                    let base = format!("{}|{}|{}|{}", acct.id, row.date, row.cents, row.description);
                    let n = seen.entry(base.clone()).or_insert(0);
                    *n += 1;
                    let id = format!("{base}|{n}");
                    if !read::txn_exists(app.store.conn(), &id)? {
                        imported += 1;
                        events.push(Event::TransactionImported {
                            id,
                            account: acct.id.clone(),
                            date: row.date,
                            cents: row.cents,
                            description: row.description,
                        });
                    }
                }
                Err((row, reason)) => {
                    if !read::failure_exists(app.store.conn(), &acct.id, row, &reason)? {
                        events.push(Event::ImportRowFailed { account: acct.id.clone(), file: acct.file.clone(), row, reason });
                    }
                }
            }
        }
        app.record(events)?;
    }
    Ok(json!({ "imported": imported }))
}

fn txn_status(t: &read::Txn) -> &'static str {
    if t.link.is_some() {
        "transfer"
    } else if t.is_assigned() {
        "assigned"
    } else {
        "unassigned"
    }
}

fn get_transactions(app: &mut App, v: &Value) -> Res {
    let c = app.store.conn();
    let id = text(v, "account")?;
    let acct = read::bank_account(c, &id)?.ok_or("no such bank account")?;
    let filter = opt_text(v, "filter").unwrap_or_else(|| "all".into());
    if !["all", "unassigned", "assigned"].contains(&filter.as_str()) {
        return Err(format!("unknown filter {filter}"));
    }
    let search = opt_text(v, "search").unwrap_or_default().to_lowercase();

    let mut mine = read::txns_for_account(c, &acct.id)?;
    mine.sort_by(|a, b| a.date.cmp(&b.date)); // stable: same-day rows keep their import order
    let mut running = acct.opening_cents;
    let mut rows = vec![];
    for t in mine {
        running += t.cents;
        rows.push((t, running));
    }
    let unassigned: Vec<&(read::Txn, i64)> = rows.iter().filter(|(t, _)| !t.is_assigned()).collect();
    let (unassigned_count, unassigned_cents) = (unassigned.len(), unassigned.iter().map(|(t, _)| t.cents).sum::<i64>());

    rows.reverse();
    let mut list = vec![];
    for (t, bal) in &rows {
        let shown = match filter.as_str() {
            "assigned" => t.is_assigned(),
            "unassigned" => !t.is_assigned(),
            _ => true,
        };
        if !shown || (!search.is_empty() && !t.description.to_lowercase().contains(&search)) {
            continue;
        }
        let link = match t.link.as_ref().map(|l| read::txn(c, l)).transpose()?.flatten() {
            Some(o) => Some(json!({
                "id": o.id, "account": read::bank_account(c, &o.account)?.map(|a| a.name), "date": o.date })),
            None => None,
        };
        let mut assignments = vec![];
        for s in &t.splits {
            let account = read::budget_account_by_id(c, &s.account)?;
            let group = match &account {
                Some(a) => read::group(c, &a.group)?.map(|g| g.name),
                None => None,
            };
            assignments.push(json!({
                "budget_account": account.as_ref().map(|a| a.name.clone()).unwrap_or_else(|| s.account.clone()),
                "budget_account_id": s.account, "group": group, "cents": s.cents }));
        }
        list.push(json!({
            "id": t.id, "date": t.date, "description": t.description, "cents": t.cents,
            "balance_cents": bal, "status": txn_status(t), "link": link, "assignments": assignments,
        }));
    }
    let failures: Vec<Value> = read::failures(c, &acct.id)?
        .into_iter()
        .map(|f| json!({ "file": f.file, "row": f.row, "reason": f.reason }))
        .collect();
    Ok(json!({
        "account": { "id": acct.id, "name": acct.name, "bank": acct.bank, "balance_cents": read::balance(c, &acct.id)? },
        "filter": filter, "transactions": list, "failures": failures,
        "unassigned_count": unassigned_count, "unassigned_cents": unassigned_cents,
    }))
}

// ---------- assigning ----------

fn assign_transactions(app: &mut App, v: &Value) -> Res {
    let ids: Vec<String> = v
        .get("transactions")
        .and_then(|x| x.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default();
    if ids.is_empty() {
        return Err("transactions is required".into());
    }
    let c = app.store.conn();
    let mut events = vec![];
    let mut created: HashMap<String, String> = HashMap::new();
    let mut plan: Vec<(String, Vec<(String, i64)>)> = vec![];

    if let Some(name) = opt_text(v, "budget_account") {
        for id in &ids {
            let t = read::txn(c, id)?.ok_or("no such transaction")?;
            plan.push((id.clone(), vec![(name.clone(), t.cents)]));
        }
    } else if let Some(splits) = v.get("splits").and_then(|x| x.as_array()) {
        if ids.len() != 1 {
            return Err("a split applies to one transaction".into());
        }
        let list = splits
            .iter()
            .map(|s| Ok((text(s, "budget_account")?, int(s, "cents")?)))
            .collect::<Result<Vec<_>, String>>()?;
        plan.push((ids[0].clone(), list));
    } else {
        for id in &ids {
            plan.push((id.clone(), vec![]));
        }
    }

    for (id, splits) in plan {
        let t = read::txn(c, &id)?.ok_or("no such transaction")?;
        if t.link.is_some() {
            return Err("transaction is linked as a transfer; unlink it first".into());
        }
        let mut resolved = vec![];
        for (name, cents) in &splits {
            if *cents == 0 || (*cents < 0) != (t.cents < 0) {
                return Err("each assigned amount must have the same sign as the transaction".into());
            }
            let account = resolve_or_create(c, name, &mut created, &mut events)?;
            resolved.push(Split { account, cents: *cents });
        }
        if !resolved.is_empty() && resolved.iter().map(|s| s.cents).sum::<i64>() != t.cents {
            return Err("assigned amounts must add up to the transaction amount".into());
        }
        events.push(Event::Assigned { transaction: id, splits: resolved });
    }
    app.record(events)?;
    ok()
}

fn resolve_or_create(c: &Connection, name: &str, created: &mut HashMap<String, String>, events: &mut Vec<Event>) -> Result<String, String> {
    if let Some(a) = read::budget_account(c, name)? {
        if a.closed {
            return Err(format!("budget account {} is closed", a.name));
        }
        return Ok(a.id);
    }
    let key = name.to_lowercase();
    if let Some(id) = created.get(&key) {
        return Ok(id.clone());
    }
    let taken: HashSet<String> = created.values().cloned().collect();
    let id = unique_id(c, name, UNGROUPED, &taken)?;
    events.push(Event::BudgetAccountCreated { id: id.clone(), name: name.trim().to_string(), group: UNGROUPED.into() });
    created.insert(key, id.clone());
    Ok(id)
}

// ---------- transfers ----------

fn transfer_problem(c: &Connection, a: &str, b: &str) -> Result<Option<String>, String> {
    let (Some(x), Some(y)) = (read::txn(c, a)?, read::txn(c, b)?) else {
        return Ok(Some("no such transaction".into()));
    };
    if x.account == y.account {
        return Ok(Some("a transfer needs two different bank accounts".into()));
    }
    if x.cents + y.cents != 0 || x.cents == 0 {
        return Ok(Some("a transfer needs a matching debit and credit".into()));
    }
    let (Some(dx), Some(dy)) = (day_number(&x.date), day_number(&y.date)) else {
        return Ok(Some("transaction has an invalid date".into()));
    };
    if (dx - dy).abs() > TRANSFER_DAYS {
        return Ok(Some(format!("transfers must be within {TRANSFER_DAYS} days of each other")));
    }
    for t in [&x, &y] {
        if t.link.is_some() {
            return Ok(Some("transaction is already linked".into()));
        }
        if !t.splits.is_empty() {
            return Ok(Some("transaction is assigned; unassign it first".into()));
        }
    }
    Ok(None)
}

fn link_transfer(app: &mut App, v: &Value) -> Res {
    let (a, b) = (text(v, "a")?, text(v, "b")?);
    if let Some(p) = transfer_problem(app.store.conn(), &a, &b)? {
        return Err(p);
    }
    app.record(vec![Event::TransferLinked { a, b }])?;
    ok()
}

fn unlink_transfer(app: &mut App, v: &Value) -> Res {
    let a = text(v, "transaction")?;
    let b = read::txn(app.store.conn(), &a)?.ok_or("no such transaction")?.link.ok_or("transaction is not linked")?;
    app.record(vec![Event::TransferUnlinked { a, b }])?;
    ok()
}

fn get_transfer_candidates(app: &mut App, v: &Value) -> Res {
    let id = text(v, "transaction")?;
    let c = app.store.conn();
    let mut list = vec![];
    for t in read::all_txns(c)? {
        if t.id != id && transfer_problem(c, &id, &t.id)?.is_none() {
            list.push(json!({ "id": t.id, "date": t.date, "description": t.description, "cents": t.cents,
                "account": read::bank_account(c, &t.account)?.map(|a| a.name) }));
        }
    }
    Ok(json!({ "candidates": list }))
}

// ---------- groups and budget accounts ----------

fn add_group(app: &mut App, v: &Value) -> Res {
    let name = text(v, "name")?;
    if read::group(app.store.conn(), &name)?.is_some() {
        return Err("a group with that name already exists".into());
    }
    let id = slug(&name);
    app.record(vec![Event::GroupAdded { id: id.clone(), name }])?;
    Ok(json!({ "id": id }))
}

fn rename_group(app: &mut App, v: &Value) -> Res {
    let c = app.store.conn();
    let id = read::group(c, &text(v, "id")?)?.ok_or("no such group")?.id;
    let name = text(v, "name")?;
    if id == UNGROUPED {
        return Err("the Ungrouped group cannot be renamed".into());
    }
    if read::group(c, &name)?.is_some_and(|g| g.id != id) {
        return Err("a group with that name already exists".into());
    }
    app.record(vec![Event::GroupRenamed { id, name }])?;
    ok()
}

fn remove_group(app: &mut App, v: &Value) -> Res {
    let c = app.store.conn();
    let id = read::group(c, &text(v, "id")?)?.ok_or("no such group")?.id;
    if id == UNGROUPED {
        return Err("the Ungrouped group cannot be removed".into());
    }
    if read::budget_accounts(c)?.iter().any(|a| a.group == id && !a.closed) {
        return Err("a group can be removed only when it has no budget accounts".into());
    }
    app.record(vec![Event::GroupRemoved { id }])?;
    ok()
}

fn reorder_groups(app: &mut App, v: &Value) -> Res {
    let ids = v
        .get("ids")
        .and_then(|x| x.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .ok_or("ids is required")?;
    app.record(vec![Event::GroupsReordered { ids }])?;
    ok()
}

/// An id for a new budget account: the name's slug, made unique with the group and a number if need be.
fn unique_id(c: &Connection, name: &str, group: &str, taken: &HashSet<String>) -> Result<String, String> {
    let base = slug(name);
    if base.is_empty() {
        return Err("invalid budget account name".into());
    }
    let mut candidate = base.clone();
    let mut n = 1;
    while taken.contains(&candidate) || read::budget_account_by_id(c, &candidate)?.is_some() {
        n += 1;
        candidate = if n == 2 { format!("{base}-{}", slug(group)) } else { format!("{base}-{}-{n}", slug(group)) };
    }
    Ok(candidate)
}

/// Names are unique within a group, ignoring case. "Income" is reserved for the fixed Income account.
fn name_taken(c: &Connection, name: &str, group: &str, except: Option<&str>) -> Result<bool, String> {
    if name.eq_ignore_ascii_case("income") {
        return Ok(true);
    }
    Ok(read::budget_accounts(c)?
        .iter()
        .any(|a| !a.closed && a.group == group && a.name.eq_ignore_ascii_case(name) && Some(a.id.as_str()) != except))
}

fn create_budget_account(app: &mut App, v: &Value) -> Res {
    let c = app.store.conn();
    let name = text(v, "name")?;
    let group = match opt_text(v, "group") {
        Some(g) => read::group(c, &g)?.ok_or("no such group")?.id,
        None => UNGROUPED.into(),
    };
    if name_taken(c, &name, &group, None)? {
        return Err("a budget account with that name already exists in that group".into());
    }
    let id = unique_id(c, &name, &group, &HashSet::new())?;
    app.record(vec![Event::BudgetAccountCreated { id: id.clone(), name, group }])?;
    Ok(json!({ "id": id }))
}

fn editable(c: &Connection, key: &str) -> Result<String, String> {
    let a = read::budget_account(c, key)?.ok_or("no such budget account")?;
    if a.id == INCOME {
        return Err("the Income account is fixed".into());
    }
    Ok(a.id)
}

fn rename_budget_account(app: &mut App, v: &Value) -> Res {
    let c = app.store.conn();
    let id = editable(c, &text(v, "id")?)?;
    let name = text(v, "name")?;
    let group = read::budget_account_by_id(c, &id)?.map(|a| a.group).unwrap_or_default();
    if name_taken(c, &name, &group, Some(&id))? {
        return Err("a budget account with that name already exists in that group".into());
    }
    app.record(vec![Event::BudgetAccountRenamed { id, name }])?;
    ok()
}

fn close_budget_account(app: &mut App, v: &Value) -> Res {
    let c = app.store.conn();
    let id = editable(c, &text(v, "id")?)?;
    let report = read::month_report(c, &month_of(&app.today()))?;
    if report.rows.get(&id).is_some_and(|r| r.available != 0) {
        return Err("a budget account can be closed only when its available amount is zero".into());
    }
    app.record(vec![Event::BudgetAccountClosed { id }])?;
    ok()
}

fn move_budget_account(app: &mut App, v: &Value) -> Res {
    let c = app.store.conn();
    let id = editable(c, &text(v, "id")?)?;
    let group = read::group(c, &text(v, "group")?)?.ok_or("no such group")?.id;
    let name = read::budget_account_by_id(c, &id)?.map(|a| a.name).unwrap_or_default();
    if name_taken(c, &name, &group, Some(&id))? {
        return Err("a budget account with that name already exists in that group".into());
    }
    app.record(vec![Event::BudgetAccountMoved { id, group }])?;
    ok()
}

fn set_budgeted(app: &mut App, v: &Value) -> Res {
    let account = editable(app.store.conn(), &text(v, "account")?)?;
    let month = text(v, "month")?;
    if !is_month(&month) {
        return Err("month must be YYYY-MM".into());
    }
    app.record(vec![Event::BudgetSet { account, month, cents: int(v, "cents")? }])?;
    ok()
}

fn set_auto_monthly(app: &mut App, v: &Value) -> Res {
    let account = editable(app.store.conn(), &text(v, "account")?)?;
    let period = opt_text(v, "period");
    let cents = match &period {
        Some(p) => {
            if monthly_cents(p, 0).is_none() {
                return Err(format!("period must be one of {}", PERIODS.join(", ")));
            }
            int(v, "cents")?
        }
        None => 0,
    };
    app.record(vec![Event::AutoMonthlySet { account, period, cents }])?;
    ok()
}

// ---------- budget ----------

fn get_budget(app: &mut App, v: &Value) -> Res {
    let c = app.store.conn();
    let month = opt_text(v, "month").unwrap_or_else(|| month_of(&app.today()));
    if !is_month(&month) {
        return Err("month must be YYYY-MM".into());
    }
    let report = read::month_report(c, &month)?;
    let budget_accounts = read::budget_accounts(c)?;
    let mut groups = vec![];
    for g in read::groups(c)? {
        let accounts: Vec<Value> = budget_accounts
            .iter()
            .filter(|a| a.group == g.id && !a.closed)
            .filter_map(|a| report.rows.get(&a.id).map(|r| (a, r)))
            .map(|(a, r)| {
                json!({
                    "id": a.id, "name": a.name, "budgeted_cents": r.budgeted, "budget_set": r.budget_set,
                    "spent_cents": r.spent, "available_cents": r.available,
                    "auto": a.auto.as_ref().map(|(p, c)| json!({ "period": p, "cents": c })),
                })
            })
            .collect();
        let sum = |k: &str| accounts.iter().map(|a| a[k].as_i64().unwrap_or(0)).sum::<i64>();
        groups.push(json!({ "id": g.id, "name": g.name, "budgeted_cents": sum("budgeted_cents"),
            "spent_cents": sum("spent_cents"), "available_cents": sum("available_cents"), "accounts": accounts }));
    }
    Ok(json!({
        "month": month,
        "income_last_month_cents": report.prev_income,
        "overspent_last_month_cents": report.overspent,
        "total_budgeted_cents": report.total_budgeted,
        "left_to_budget_cents": report.left_to_budget,
        "over_budgeted_cents": (-report.left_to_budget).max(0),
        "groups": groups,
    }))
}
