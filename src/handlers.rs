//! Commands and queries. Commands validate against the read model, then append events.

use crate::app::App;
use crate::banks;
use crate::dates::{day_number, is_month, month_of};
use crate::mediator::Handler;
use crate::model::{monthly_cents, slug, Event, Split, INCOME, UNGROUPED};
use serde_json::{json, Value};
use std::collections::HashMap;

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
    if app.model.bank_account(&id).is_some() {
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
    let m = &app.model;
    let mut total = 0;
    let accounts: Vec<Value> = m
        .bank_accounts
        .iter()
        .map(|a| {
            let balance = m.balance(&a.id);
            total += balance;
            let unassigned = m.txns.iter().filter(|t| t.account == a.id && !t.is_assigned()).count();
            json!({ "id": a.id, "name": a.name, "bank": a.bank, "balance_cents": balance, "unassigned": unassigned })
        })
        .collect();
    Ok(json!({ "accounts": accounts, "total_cents": total }))
}

fn import_statements(app: &mut App, _: &Value) -> Res {
    let mut imported = 0;
    for acct in app.model.bank_accounts.clone() {
        let Ok(content) = std::fs::read_to_string(app.statements_dir.join(&acct.file)) else {
            continue;
        };
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut events = vec![];
        for r in banks::parse(&acct.bank, &content) {
            match r {
                Ok(row) => {
                    let base = format!("{}|{}|{}|{}", acct.id, row.date, row.cents, row.description);
                    let n = seen.entry(base.clone()).or_insert(0);
                    *n += 1;
                    let id = format!("{base}|{n}");
                    if !app.model.txn_index.contains_key(&id) {
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
                    let known = app
                        .model
                        .failures
                        .iter()
                        .any(|f| f.account == acct.id && f.row == row && f.reason == reason);
                    if !known {
                        events.push(Event::ImportRowFailed { account: acct.id.clone(), file: acct.file.clone(), row, reason });
                    }
                }
            }
        }
        app.record(events)?;
    }
    Ok(json!({ "imported": imported }))
}

fn txn_status(t: &crate::model::Txn) -> &'static str {
    if t.link.is_some() {
        "transfer"
    } else if t.is_assigned() {
        "assigned"
    } else {
        "unassigned"
    }
}

fn get_transactions(app: &mut App, v: &Value) -> Res {
    let m = &app.model;
    let id = text(v, "account")?;
    let acct = m.bank_account(&id).ok_or("no such bank account")?;
    let filter = opt_text(v, "filter").unwrap_or_else(|| "all".into());
    if !["all", "unassigned", "assigned"].contains(&filter.as_str()) {
        return Err(format!("unknown filter {filter}"));
    }
    let search = opt_text(v, "search").unwrap_or_default().to_lowercase();

    let mut mine: Vec<(usize, &crate::model::Txn)> = m.txns.iter().enumerate().filter(|(_, t)| t.account == acct.id).collect();
    mine.sort_by(|a, b| (&a.1.date, a.0).cmp(&(&b.1.date, b.0)));
    let mut running = acct.opening_cents;
    let mut rows = vec![];
    for (i, t) in mine {
        running += t.cents;
        rows.push((i, t, running));
    }
    let unassigned: Vec<_> = rows.iter().filter(|(_, t, _)| !t.is_assigned()).collect();
    let (unassigned_count, unassigned_cents) = (unassigned.len(), unassigned.iter().map(|(_, t, _)| t.cents).sum::<i64>());

    rows.reverse();
    let list: Vec<Value> = rows
        .iter()
        .filter(|(_, t, _)| match filter.as_str() {
            "assigned" => t.is_assigned(),
            "unassigned" => !t.is_assigned(),
            _ => true,
        })
        .filter(|(_, t, _)| search.is_empty() || t.description.to_lowercase().contains(&search))
        .map(|(_, t, bal)| {
            let link = t.link.as_ref().and_then(|l| m.txn(l)).map(|o| {
                json!({ "id": o.id, "account": m.bank_account(&o.account).map(|a| a.name.clone()), "date": o.date })
            });
            json!({
                "id": t.id, "date": t.date, "description": t.description, "cents": t.cents,
                "balance_cents": bal, "status": txn_status(t), "link": link,
                "assignments": t.splits.iter().map(|s| json!({ "budget_account": m.account_name(&s.account), "cents": s.cents })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let failures: Vec<Value> = m
        .failures
        .iter()
        .filter(|f| f.account == acct.id)
        .map(|f| json!({ "file": f.file, "row": f.row, "reason": f.reason }))
        .collect();
    Ok(json!({
        "account": { "id": acct.id, "name": acct.name, "bank": acct.bank, "balance_cents": m.balance(&acct.id) },
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
    let mut events = vec![];
    let mut created: HashMap<String, String> = HashMap::new();
    let mut plan: Vec<(String, Vec<(String, i64)>)> = vec![];

    if let Some(name) = opt_text(v, "budget_account") {
        for id in &ids {
            let t = app.model.txn(id).ok_or("no such transaction")?;
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
        let t = app.model.txn(&id).ok_or("no such transaction")?;
        if t.link.is_some() {
            return Err("transaction is linked as a transfer; unlink it first".into());
        }
        let mut resolved = vec![];
        for (name, cents) in &splits {
            if *cents == 0 || (*cents < 0) != (t.cents < 0) {
                return Err("each assigned amount must have the same sign as the transaction".into());
            }
            let account = resolve_or_create(app, name, &mut created, &mut events)?;
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

fn resolve_or_create(app: &App, name: &str, created: &mut HashMap<String, String>, events: &mut Vec<Event>) -> Result<String, String> {
    if let Some(a) = app.model.budget_account(name) {
        if a.closed {
            return Err(format!("budget account {} is closed", a.name));
        }
        return Ok(a.id.clone());
    }
    let key = name.to_lowercase();
    if let Some(id) = created.get(&key) {
        return Ok(id.clone());
    }
    let id = slug(name);
    if id.is_empty() {
        return Err("invalid budget account name".into());
    }
    events.push(Event::BudgetAccountCreated { id: id.clone(), name: name.trim().to_string(), group: UNGROUPED.into() });
    created.insert(key, id.clone());
    Ok(id)
}

// ---------- transfers ----------

fn transfer_problem(app: &App, a: &str, b: &str) -> Option<String> {
    let (Some(x), Some(y)) = (app.model.txn(a), app.model.txn(b)) else {
        return Some("no such transaction".into());
    };
    if x.account == y.account {
        return Some("a transfer needs two different bank accounts".into());
    }
    if x.cents + y.cents != 0 || x.cents == 0 {
        return Some("a transfer needs a matching debit and credit".into());
    }
    let (dx, dy) = (day_number(&x.date)?, day_number(&y.date)?);
    if (dx - dy).abs() > TRANSFER_DAYS {
        return Some(format!("transfers must be within {TRANSFER_DAYS} days of each other"));
    }
    for t in [x, y] {
        if t.link.is_some() {
            return Some("transaction is already linked".into());
        }
        if !t.splits.is_empty() {
            return Some("transaction is assigned; unassign it first".into());
        }
    }
    None
}

fn link_transfer(app: &mut App, v: &Value) -> Res {
    let (a, b) = (text(v, "a")?, text(v, "b")?);
    if let Some(p) = transfer_problem(app, &a, &b) {
        return Err(p);
    }
    app.record(vec![Event::TransferLinked { a, b }])?;
    ok()
}

fn unlink_transfer(app: &mut App, v: &Value) -> Res {
    let a = text(v, "transaction")?;
    let b = app.model.txn(&a).ok_or("no such transaction")?.link.clone().ok_or("transaction is not linked")?;
    app.record(vec![Event::TransferUnlinked { a, b }])?;
    ok()
}

fn get_transfer_candidates(app: &mut App, v: &Value) -> Res {
    let id = text(v, "transaction")?;
    let m = &app.model;
    let list: Vec<Value> = m
        .txns
        .iter()
        .filter(|t| t.id != id && transfer_problem(app, &id, &t.id).is_none())
        .map(|t| json!({ "id": t.id, "date": t.date, "description": t.description, "cents": t.cents,
            "account": m.bank_account(&t.account).map(|a| a.name.clone()) }))
        .collect();
    Ok(json!({ "candidates": list }))
}

// ---------- groups and budget accounts ----------

fn add_group(app: &mut App, v: &Value) -> Res {
    let name = text(v, "name")?;
    if app.model.group(&name).is_some() {
        return Err("a group with that name already exists".into());
    }
    let id = slug(&name);
    app.record(vec![Event::GroupAdded { id: id.clone(), name }])?;
    Ok(json!({ "id": id }))
}

fn rename_group(app: &mut App, v: &Value) -> Res {
    let id = app.model.group(&text(v, "id")?).ok_or("no such group")?.id.clone();
    let name = text(v, "name")?;
    if id == UNGROUPED {
        return Err("the Ungrouped group cannot be renamed".into());
    }
    if app.model.group(&name).is_some_and(|g| g.id != id) {
        return Err("a group with that name already exists".into());
    }
    app.record(vec![Event::GroupRenamed { id, name }])?;
    ok()
}

fn remove_group(app: &mut App, v: &Value) -> Res {
    let id = app.model.group(&text(v, "id")?).ok_or("no such group")?.id.clone();
    if id == UNGROUPED {
        return Err("the Ungrouped group cannot be removed".into());
    }
    if app.model.budget_accounts.iter().any(|a| a.group == id && !a.closed) {
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

fn create_budget_account(app: &mut App, v: &Value) -> Res {
    let name = text(v, "name")?;
    let id = slug(&name);
    if id.is_empty() || app.model.budget_account(&name).is_some() || app.model.budget_account(&id).is_some() {
        return Err("a budget account with that name already exists".into());
    }
    let group = match opt_text(v, "group") {
        Some(g) => app.model.group(&g).ok_or("no such group")?.id.clone(),
        None => UNGROUPED.into(),
    };
    app.record(vec![Event::BudgetAccountCreated { id: id.clone(), name, group }])?;
    Ok(json!({ "id": id }))
}

fn editable(app: &App, key: &str) -> Result<String, String> {
    let a = app.model.budget_account(key).ok_or("no such budget account")?;
    if a.id == INCOME {
        return Err("the Income account is fixed".into());
    }
    Ok(a.id.clone())
}

fn rename_budget_account(app: &mut App, v: &Value) -> Res {
    let id = editable(app, &text(v, "id")?)?;
    let name = text(v, "name")?;
    if app.model.budget_account(&name).is_some_and(|a| a.id != id) {
        return Err("a budget account with that name already exists".into());
    }
    app.record(vec![Event::BudgetAccountRenamed { id, name }])?;
    ok()
}

fn close_budget_account(app: &mut App, v: &Value) -> Res {
    let id = editable(app, &text(v, "id")?)?;
    let month = month_of(&app.today());
    let report = app.model.month_report(&month);
    if report.rows.get(&id).is_some_and(|r| r.available != 0) {
        return Err("a budget account can be closed only when its available amount is zero".into());
    }
    app.record(vec![Event::BudgetAccountClosed { id }])?;
    ok()
}

fn move_budget_account(app: &mut App, v: &Value) -> Res {
    let id = editable(app, &text(v, "id")?)?;
    let group = app.model.group(&text(v, "group")?).ok_or("no such group")?.id.clone();
    app.record(vec![Event::BudgetAccountMoved { id, group }])?;
    ok()
}

fn set_budgeted(app: &mut App, v: &Value) -> Res {
    let account = editable(app, &text(v, "account")?)?;
    let month = text(v, "month")?;
    if !is_month(&month) {
        return Err("month must be YYYY-MM".into());
    }
    app.record(vec![Event::BudgetSet { account, month, cents: int(v, "cents")? }])?;
    ok()
}

fn set_auto_monthly(app: &mut App, v: &Value) -> Res {
    let account = editable(app, &text(v, "account")?)?;
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
    let month = opt_text(v, "month").unwrap_or_else(|| month_of(&app.today()));
    if !is_month(&month) {
        return Err("month must be YYYY-MM".into());
    }
    let m = &app.model;
    let report = m.month_report(&month);
    let groups: Vec<Value> = m
        .groups
        .iter()
        .map(|g| {
            let accounts: Vec<Value> = m
                .budget_accounts
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
            json!({ "id": g.id, "name": g.name, "budgeted_cents": sum("budgeted_cents"),
                "spent_cents": sum("spent_cents"), "available_cents": sum("available_cents"), "accounts": accounts })
        })
        .collect();
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
