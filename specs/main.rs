//! Gherkin runner. Features and step definitions live side by side in `specs/`.
//! Steps drive the application through its mediator, against an in-memory event
//! store and a temporary statements folder.

mod bank_accounts;
mod budgets;

use cucumber::{gherkin::Step, given, then, when, World};
use our_budget::app::App;
use our_budget::banks::BANKS;
use our_budget::dates::{month_abbrev, parse_iso};
use our_budget::event_store::EventStore;
use our_budget::mediator::Mediator;
use our_budget::money::parse_cents;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct BudgetWorld {
    pub app: App,
    pub mediator: Mediator,
    pub last: Result<Value, String>,
}

impl BudgetWorld {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("our-budget-specs-{}-{}", std::process::id(), COUNTER.fetch_add(1, Ordering::SeqCst)));
        std::fs::create_dir_all(&dir).unwrap();
        let mut app = App::new(EventStore::open_in_memory().unwrap(), dir).unwrap();
        app.today_override = Some("2026-11-16".into());
        BudgetWorld { app, mediator: Mediator::default(), last: Ok(json!({})) }
    }

    /// Sends a request and remembers the outcome for "Then the request fails with".
    pub fn send(&mut self, request: &str, body: Value) -> Value {
        self.last = self.mediator.send(&mut self.app, request, &body);
        self.last.clone().unwrap_or(Value::Null)
    }

    /// Sends a request that must succeed.
    pub fn ok(&mut self, request: &str, body: Value) -> Value {
        self.last = self.mediator.send(&mut self.app, request, &body);
        match &self.last {
            Ok(v) => v.clone(),
            Err(e) => panic!("{request} failed: {e}"),
        }
    }
}

pub fn cents(s: &str) -> i64 {
    parse_cents(s).unwrap_or_else(|| panic!("bad amount {s}"))
}

pub fn table(step: &Step) -> Vec<Vec<String>> {
    step.table.as_ref().expect("step needs a table").rows.iter().skip(1).cloned().collect()
}

/// Renders a statement row in the bank's own download format.
fn statement_line(bank: &str, date: &str, amount: &str, description: &str) -> String {
    let (y, m, d) = parse_iso(date).expect("ISO date");
    let c = cents(amount);
    if bank == "NAB" {
        format!("{d:02} {} {:02},{:.2},123456789,,Payment,{description},,,", cap(month_abbrev(m)), y % 100, c as f64 / 100.0)
    } else {
        let (debit, credit) = if c < 0 { (format!("{:.2}", -c as f64 / 100.0), String::new()) } else { (String::new(), format!("{:.2}", c as f64 / 100.0)) };
        format!("123456,{d:02}/{m:02}/{y},{description},{debit},{credit},,,")
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

#[given(expr = "today is {string}")]
fn today(w: &mut BudgetWorld, date: String) {
    w.app.today_override = Some(date);
}

#[given(expr = "a {word} bank account {string} with opening balance {string}")]
fn bank_account(w: &mut BudgetWorld, bank: String, name: String, opening: String) {
    assert!(BANKS.contains(&bank.as_str()), "unknown bank {bank}");
    let file = format!("{}.csv", our_budget::model::slug(&name));
    w.ok("RegisterBankAccount", json!({ "name": name, "bank": bank, "file": file, "opening_cents": cents(&opening) }));
}

fn write_statement(w: &mut BudgetWorld, account: &str, text: String) {
    let acct = w.app.model.bank_account(account).expect("bank account").clone();
    std::fs::write(w.app.statements_dir.join(&acct.file), text).unwrap();
}

#[given(expr = "the {string} statement contains:")]
fn statement(w: &mut BudgetWorld, step: &Step, account: String) {
    let bank = w.app.model.bank_account(&account).expect("bank account").bank.clone();
    let lines: Vec<String> = table(step).iter().map(|r| statement_line(&bank, &r[0], &r[1], &r[2])).collect();
    write_statement(w, &account, lines.join("\n"));
}

#[given(expr = "the {string} statement file contains:")]
fn raw_statement(w: &mut BudgetWorld, step: &Step, account: String) {
    write_statement(w, &account, step.docstring.clone().expect("docstring"));
}

#[given(expr = "the {string} statement file is missing")]
fn missing_statement(_w: &mut BudgetWorld, _account: String) {}

#[given(expr = "the statements have been imported")]
#[when(expr = "the statements are imported")]
fn import(w: &mut BudgetWorld) {
    w.ok("ImportStatements", json!({}));
}

#[then(expr = "the request fails with {string}")]
fn fails(w: &mut BudgetWorld, expected: String) {
    match &w.last {
        Err(e) => assert!(e.to_lowercase().contains(&expected.to_lowercase()), "error was {e:?}, expected {expected:?}"),
        Ok(v) => panic!("expected failure containing {expected:?}, but it succeeded: {v}"),
    }
}

#[tokio::main]
async fn main() {
    BudgetWorld::cucumber().fail_on_skipped().run_and_exit("specs").await;
}
