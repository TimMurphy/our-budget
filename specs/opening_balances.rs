//! Steps for opening_balances.feature.

use crate::BudgetWorld;
use cucumber::{gherkin::Step, given, when};
use our_budget::setup;

fn secrets_path(w: &BudgetWorld) -> std::path::PathBuf {
    w.app.statements_dir.join("secrets.md")
}

#[given(expr = "the secrets file contains:")]
#[when(expr = "the secrets file contains:")]
fn secrets(w: &mut BudgetWorld, step: &Step) {
    std::fs::write(secrets_path(w), step.docstring.clone().expect("docstring")).unwrap();
}

#[given(expr = "the household bank accounts are set up")]
#[when(expr = "the household bank accounts are set up")]
fn set_up(w: &mut BudgetWorld) {
    let path = secrets_path(w);
    setup::register_accounts(&mut w.app, &w.mediator, &path).unwrap();
}
