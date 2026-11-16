//! Steps for budgets.feature.

use crate::{cents, BudgetWorld};
use cucumber::{given, then, when};
use serde_json::{json, Value};

pub fn budget(w: &mut BudgetWorld, month: &str) -> Value {
    w.ok("GetBudget", json!({ "month": month }))
}

#[given(expr = "I add group {string}")]
#[when(expr = "I add group {string}")]
fn add_group(w: &mut BudgetWorld, name: String) {
    w.send("AddGroup", json!({ "name": name }));
}

#[when(expr = "I remove group {string}")]
fn remove_group(w: &mut BudgetWorld, name: String) {
    w.send("RemoveGroup", json!({ "id": name }));
}

#[when(expr = "I rename group {string} to {string}")]
fn rename_group(w: &mut BudgetWorld, name: String, to: String) {
    w.send("RenameGroup", json!({ "id": name, "name": to }));
}

#[given(expr = "I create budget account {string}")]
#[when(expr = "I create budget account {string}")]
fn create_account(w: &mut BudgetWorld, name: String) {
    w.send("CreateBudgetAccount", json!({ "name": name }));
}

#[given(expr = "I create budget account {string} in group {string}")]
#[when(expr = "I create budget account {string} in group {string}")]
fn create_account_in_group(w: &mut BudgetWorld, name: String, group: String) {
    w.send("CreateBudgetAccount", json!({ "name": name, "group": group }));
}

#[when(expr = "I move budget account {string} to group {string}")]
fn move_account(w: &mut BudgetWorld, name: String, group: String) {
    w.ok("MoveBudgetAccount", json!({ "id": name, "group": group }));
}

#[when(expr = "I rename budget account {string} to {string}")]
fn rename_account(w: &mut BudgetWorld, name: String, to: String) {
    w.send("RenameBudgetAccount", json!({ "id": name, "name": to }));
}

#[when(expr = "I close budget account {string}")]
fn close_account(w: &mut BudgetWorld, name: String) {
    w.send("CloseBudgetAccount", json!({ "id": name }));
}

#[given(expr = "I budget {string} for {string} in {string}")]
#[when(expr = "I budget {string} for {string} in {string}")]
fn set_budgeted(w: &mut BudgetWorld, amount: String, account: String, month: String) {
    w.ok("SetBudgeted", json!({ "account": account, "month": month, "cents": cents(&amount) }));
}

#[given(expr = "budget account {string} is set to auto monthly {string} {string}")]
#[when(expr = "budget account {string} is set to auto monthly {string} {string}")]
fn auto_monthly(w: &mut BudgetWorld, account: String, period: String, amount: String) {
    w.ok("SetAutoMonthly", json!({ "account": account, "period": period, "cents": cents(&amount) }));
}

#[when(expr = "auto monthly is turned off for {string}")]
fn auto_off(w: &mut BudgetWorld, account: String) {
    w.ok("SetAutoMonthly", json!({ "account": account }));
}

#[then(expr = "group {string} has budget accounts {string}")]
fn group_accounts(w: &mut BudgetWorld, group: String, expected: String) {
    let b = budget(w, "2026-11");
    let g = b["groups"].as_array().unwrap().iter().find(|g| g["name"] == group.as_str()).expect("group");
    let names: Vec<&str> = g["accounts"].as_array().unwrap().iter().map(|a| a["name"].as_str().unwrap()).collect();
    assert_eq!(names.join(", "), expected);
}

#[then(expr = "budget screen has no group {string}")]
fn no_group(w: &mut BudgetWorld, group: String) {
    let b = budget(w, "2026-11");
    assert!(!b["groups"].as_array().unwrap().iter().any(|g| g["name"] == group.as_str()));
}

#[then(expr = "budget screen has no budget account {string}")]
fn no_account(w: &mut BudgetWorld, name: String) {
    let b = budget(w, "2026-11");
    let any = b["groups"].as_array().unwrap().iter().flat_map(|g| g["accounts"].as_array().unwrap().clone()).any(|a| a["name"] == name.as_str());
    assert!(!any, "{name} is still on the budget screen");
}

#[then(expr = "in {string} budget account {string} shows budgeted {string}, spent {string} and available {string}")]
fn shows(w: &mut BudgetWorld, month: String, name: String, budgeted: String, spent: String, available: String) {
    let b = budget(w, &month);
    let a = b["groups"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["accounts"].as_array().unwrap().clone())
        .find(|a| a["name"] == name.as_str())
        .unwrap_or_else(|| panic!("no budget account {name}"));
    assert_eq!(a["budgeted_cents"].as_i64().unwrap(), cents(&budgeted), "budgeted");
    assert_eq!(a["spent_cents"].as_i64().unwrap(), cents(&spent), "spent");
    assert_eq!(a["available_cents"].as_i64().unwrap(), cents(&available), "available");
}

#[then(expr = "in {string} left to budget is {string}")]
fn left_to_budget(w: &mut BudgetWorld, month: String, expected: String) {
    assert_eq!(budget(w, &month)["left_to_budget_cents"].as_i64().unwrap(), cents(&expected));
}

#[then(expr = "in {string} over budgeted is {string}")]
fn over_budgeted(w: &mut BudgetWorld, month: String, expected: String) {
    assert_eq!(budget(w, &month)["over_budgeted_cents"].as_i64().unwrap(), cents(&expected));
}

/// The id of the budget account with this name in this group, from the budget screen.
pub fn account_id(w: &mut BudgetWorld, name: &str, group: &str) -> String {
    let b = budget(w, "2026-11");
    let g = b["groups"].as_array().unwrap().iter().find(|g| g["name"] == group).expect("group");
    g["accounts"].as_array().unwrap().iter().find(|a| a["name"] == name).unwrap_or_else(|| panic!("no {name} in {group}"))["id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[when(expr = "I move the budget account {string} in group {string} to group {string}")]
fn move_from_group(w: &mut BudgetWorld, name: String, from: String, to: String) {
    let id = account_id(w, &name, &from);
    w.send("MoveBudgetAccount", json!({ "id": id, "group": to }));
}

#[then(expr = "in {string} budget account {string} in group {string} shows budgeted {string}, spent {string} and available {string}")]
fn shows_in_group(w: &mut BudgetWorld, month: String, name: String, group: String, budgeted: String, spent: String, available: String) {
    let b = budget(w, &month);
    let g = b["groups"].as_array().unwrap().iter().find(|g| g["name"] == group.as_str()).expect("group");
    let a = g["accounts"].as_array().unwrap().iter().find(|a| a["name"] == name.as_str()).unwrap_or_else(|| panic!("no {name} in {group}"));
    assert_eq!(a["budgeted_cents"].as_i64().unwrap(), cents(&budgeted), "budgeted");
    assert_eq!(a["spent_cents"].as_i64().unwrap(), cents(&spent), "spent");
    assert_eq!(a["available_cents"].as_i64().unwrap(), cents(&available), "available");
}
