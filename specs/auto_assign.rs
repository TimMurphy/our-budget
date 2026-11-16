//! Steps for auto_assign.feature.

use crate::bank_accounts::txn_id;
use crate::BudgetWorld;
use cucumber::{given, then, when};
use serde_json::{json, Value};

#[given(expr = "I add an auto assign rule {string} for {string}")]
#[when(expr = "I add an auto assign rule {string} for {string}")]
fn add_rule(w: &mut BudgetWorld, phrase: String, account: String) {
    w.send("CreateAutoAssignRule", json!({ "phrase": phrase, "budget_account": account }));
}

#[then(expr = "the rule assigned {int} transaction(s)")]
fn rule_assigned(w: &mut BudgetWorld, n: u64) {
    let v = w.last.clone().expect("the rule was created");
    assert_eq!(v["assigned"].as_u64().unwrap(), n);
}

#[then(expr = "the import auto assigned {int} transaction(s)")]
fn import_assigned(w: &mut BudgetWorld, n: u64) {
    let v = w.last.clone().expect("the import succeeded");
    assert_eq!(v["auto_assigned"].as_u64().unwrap(), n);
}

#[when(expr = "I approve {string} in {string}")]
fn approve(w: &mut BudgetWorld, description: String, account: String) {
    let id = txn_id(w, &account, &description);
    w.ok("ApproveTransactions", json!({ "transactions": [id] }));
}

#[when(expr = "I approve every pending transaction in {string}")]
fn approve_all(w: &mut BudgetWorld, account: String) {
    let v = w.ok("GetTransactions", json!({ "account": account, "filter": "pending" }));
    let ids: Vec<Value> = v["transactions"].as_array().unwrap().iter().map(|t| t["id"].clone()).collect();
    w.ok("ApproveTransactions", json!({ "transactions": ids }));
}

fn pending(w: &mut BudgetWorld, description: &str, account: &str) -> bool {
    let v = w.ok("GetTransactions", json!({ "account": account, "filter": "all" }));
    v["transactions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["description"] == description)
        .unwrap_or_else(|| panic!("no transaction {description}"))["pending"]
        .as_bool()
        .unwrap()
}

#[then(expr = "{string} in {string} is pending approval")]
fn is_pending(w: &mut BudgetWorld, description: String, account: String) {
    assert!(pending(w, &description, &account), "{description} is not pending approval");
}

#[then(expr = "{string} in {string} is not pending approval")]
fn is_not_pending(w: &mut BudgetWorld, description: String, account: String) {
    assert!(!pending(w, &description, &account), "{description} is pending approval");
}

#[then(expr = "{string} has {int} transaction(s) pending approval")]
fn pending_count(w: &mut BudgetWorld, account: String, n: u64) {
    let v = w.ok("GetTransactions", json!({ "account": account }));
    assert_eq!(v["pending_count"].as_u64().unwrap(), n);
}

#[when(expr = "I remove the auto assign rule {string}")]
fn remove_rule(w: &mut BudgetWorld, phrase: String) {
    let v = w.ok("ListAutoAssignRules", json!({}));
    let id = v["rules"].as_array().unwrap().iter().find(|r| r["phrase"] == phrase.as_str()).expect("rule")["id"].clone();
    w.ok("RemoveAutoAssignRule", json!({ "id": id }));
}

#[then(expr = "there are {int} auto assign rule(s)")]
fn rule_count(w: &mut BudgetWorld, n: usize) {
    let v = w.ok("ListAutoAssignRules", json!({}));
    assert_eq!(v["rules"].as_array().unwrap().len(), n);
}
