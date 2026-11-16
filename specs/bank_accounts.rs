//! Steps for bank-accounts.feature.

use crate::{cents, table, BudgetWorld};
use cucumber::{gherkin::Step, given, then, when};
use serde_json::{json, Value};

pub fn txn_id(w: &BudgetWorld, account: &str, description: &str) -> String {
    let c = w.app.store.conn();
    let acct = our_budget::read::bank_account(c, account).unwrap().expect("bank account").id;
    our_budget::read::txns_for_account(c, &acct)
        .unwrap()
        .into_iter()
        .find(|t| t.description == description)
        .unwrap_or_else(|| panic!("no transaction {description} in {account}"))
        .id
}

fn transactions(w: &mut BudgetWorld, account: &str, filter: &str, search: &str) -> Value {
    w.ok("GetTransactions", json!({ "account": account, "filter": filter, "search": search }))
}

fn descriptions(v: &Value) -> Vec<String> {
    v["transactions"].as_array().unwrap().iter().map(|t| t["description"].as_str().unwrap().to_string()).collect()
}

fn find(w: &mut BudgetWorld, account: &str, description: &str) -> Value {
    let v = transactions(w, account, "all", "");
    v["transactions"].as_array().unwrap().iter().find(|t| t["description"] == description).cloned().expect("transaction")
}

#[then(expr = "{string} has balance {string}")]
fn balance(w: &mut BudgetWorld, account: String, expected: String) {
    let v = transactions(w, &account, "all", "");
    assert_eq!(v["account"]["balance_cents"].as_i64().unwrap(), cents(&expected));
}

#[then(expr = "the total of all bank accounts is {string}")]
fn total(w: &mut BudgetWorld, expected: String) {
    let v = w.ok("ListBankAccounts", json!({}));
    assert_eq!(v["total_cents"].as_i64().unwrap(), cents(&expected));
}

#[then(expr = "{string} has {int} transaction(s)")]
fn count(w: &mut BudgetWorld, account: String, n: usize) {
    assert_eq!(descriptions(&transactions(w, &account, "all", "")).len(), n);
}

#[then(expr = "{string} has {int} import failure(s)")]
fn failures(w: &mut BudgetWorld, account: String, n: usize) {
    assert_eq!(transactions(w, &account, "all", "")["failures"].as_array().unwrap().len(), n);
}

#[then(expr = "{string} filtered by {string} lists {string}")]
fn filtered(w: &mut BudgetWorld, account: String, filter: String, expected: String) {
    assert_eq!(descriptions(&transactions(w, &account, &filter, "")).join(", "), expected);
}

#[then(expr = "{string} searched for {string} lists {string}")]
fn searched(w: &mut BudgetWorld, account: String, search: String, expected: String) {
    assert_eq!(descriptions(&transactions(w, &account, "all", &search)).join(", "), expected);
}

#[then(expr = "{string} shows {int} unassigned transaction(s) totalling {string}")]
fn unassigned(w: &mut BudgetWorld, account: String, n: usize, total: String) {
    let v = transactions(w, &account, "all", "");
    assert_eq!(v["unassigned_count"].as_u64().unwrap() as usize, n);
    assert_eq!(v["unassigned_cents"].as_i64().unwrap(), cents(&total));
}

#[given(expr = "I assign {string} in {string} to {string}")]
#[when(expr = "I assign {string} in {string} to {string}")]
fn assign(w: &mut BudgetWorld, description: String, account: String, budget_account: String) {
    let id = txn_id(w, &account, &description);
    w.send("AssignTransactions", json!({ "transactions": [id], "budget_account": budget_account }));
}

#[given(expr = "I unassign {string} in {string}")]
#[when(expr = "I unassign {string} in {string}")]
fn unassign(w: &mut BudgetWorld, description: String, account: String) {
    let id = txn_id(w, &account, &description);
    w.ok("AssignTransactions", json!({ "transactions": [id] }));
}

#[when(expr = "I assign every transaction in {string} matching {string} to {string}")]
fn assign_many(w: &mut BudgetWorld, account: String, search: String, budget_account: String) {
    let ids: Vec<Value> = transactions(w, &account, "all", &search)["transactions"].as_array().unwrap().iter().map(|t| t["id"].clone()).collect();
    w.ok("AssignTransactions", json!({ "transactions": ids, "budget_account": budget_account }));
}

#[when(expr = "I split {string} in {string} into:")]
fn split(w: &mut BudgetWorld, step: &Step, description: String, account: String) {
    let id = txn_id(w, &account, &description);
    let splits: Vec<Value> = table(step).iter().map(|r| json!({ "budget_account": r[0], "cents": cents(&r[1]) })).collect();
    w.send("AssignTransactions", json!({ "transactions": [id], "splits": splits }));
}

#[then(expr = "{string} in {string} is {string}")]
fn status(w: &mut BudgetWorld, description: String, account: String, expected: String) {
    assert_eq!(find(w, &account, &description)["status"], expected.as_str());
}

#[then(expr = "{string} in {string} is assigned {string} to {string}")]
fn assigned_to(w: &mut BudgetWorld, description: String, account: String, amount: String, budget_account: String) {
    let t = find(w, &account, &description);
    let found = t["assignments"]
        .as_array()
        .unwrap()
        .iter()
        .any(|a| a["budget_account"] == budget_account.as_str() && a["cents"].as_i64() == Some(cents(&amount)));
    assert!(found, "{description} assignments were {}", t["assignments"]);
}

#[then(expr = "budget account {string} is in group {string}")]
fn in_group(w: &mut BudgetWorld, name: String, group: String) {
    let b = w.ok("GetBudget", json!({}));
    let g = b["groups"].as_array().unwrap().iter().find(|g| g["name"] == group.as_str()).expect("group");
    assert!(g["accounts"].as_array().unwrap().iter().any(|a| a["name"] == name.as_str()), "{name} not in {group}");
}

#[when(expr = "I link {string} in {string} with {string} in {string}")]
#[given(expr = "I link {string} in {string} with {string} in {string}")]
fn link(w: &mut BudgetWorld, d1: String, a1: String, d2: String, a2: String) {
    let (a, b) = (txn_id(w, &a1, &d1), txn_id(w, &a2, &d2));
    w.send("LinkTransfer", json!({ "a": a, "b": b }));
}

#[when(expr = "I unlink {string} in {string}")]
fn unlink(w: &mut BudgetWorld, description: String, account: String) {
    let id = txn_id(w, &account, &description);
    w.ok("UnlinkTransfer", json!({ "transaction": id }));
}

#[given(expr = "transactions before {string} are not imported")]
fn import_from(w: &mut BudgetWorld, date: String) {
    w.app.import_from = date;
}

#[when(expr = "I assign {string} in {string} to {string} in group {string}")]
fn assign_in_group(w: &mut BudgetWorld, description: String, account: String, budget_account: String, group: String) {
    let account_id = crate::budgets::account_id(w, &budget_account, &group);
    let id = txn_id(w, &account, &description);
    w.send("AssignTransactions", json!({ "transactions": [id], "budget_account": account_id }));
}
