//! Steps for read-model.feature.

use crate::BudgetWorld;
use cucumber::{given, then, when};
use our_budget::app::App;
use our_budget::event_store::EventStore;
use our_budget::model::Event;

fn db_path(w: &BudgetWorld) -> std::path::PathBuf {
    w.app.statements_dir.join("our-budget.db")
}

#[given(expr = "an event is stored without updating the read model")]
fn stored_event(w: &mut BudgetWorld) {
    let e = Event::BudgetAccountCreated { id: "extra".into(), name: "Extra".into(), group: "ungrouped".into() };
    w.app.store.append_events_only(&[e]).unwrap();
}

#[given(expr = "the read model state is removed")]
fn remove_saved(w: &mut BudgetWorld) {
    let c = rusqlite::Connection::open(db_path(w)).unwrap();
    c.execute("DELETE FROM read_model_state", []).unwrap();
}

#[given(expr = "a stored event is deleted behind the application's back")]
fn delete_event(w: &mut BudgetWorld) {
    let c = rusqlite::Connection::open(db_path(w)).unwrap();
    c.execute("DELETE FROM events WHERE seq = (SELECT MIN(seq) FROM events)", []).unwrap();
}

#[when(expr = "the application is restarted")]
fn restart(w: &mut BudgetWorld) {
    let dir = w.app.statements_dir.clone();
    let (today, import_from) = (w.app.today_override.clone(), w.app.import_from.clone());
    let mut app = App::new(EventStore::open(&dir.join("our-budget.db")).unwrap(), dir).unwrap();
    app.today_override = today;
    app.import_from = import_from;
    w.app = app;
}

#[then(expr = "the read model was loaded without replaying events")]
fn loaded(w: &mut BudgetWorld) {
    assert!(!w.app.rebuilt, "the read model was rebuilt");
    assert_eq!(w.app.replayed, 0);
}

#[then(expr = "{int} newer event(s) was/were applied to the read model tables")]
fn newer(w: &mut BudgetWorld, n: usize) {
    assert!(!w.app.rebuilt, "the read model was rebuilt");
    assert_eq!(w.app.replayed, n);
}

#[then(expr = "the read model was rebuilt from the events")]
fn rebuilt(w: &mut BudgetWorld) {
    assert!(w.app.rebuilt, "the read model was loaded, not rebuilt");
}

#[then(expr = "the {word} table has {int} row(s)")]
fn table_rows(w: &mut BudgetWorld, table: String, n: i64) {
    let c = rusqlite::Connection::open(db_path(w)).unwrap();
    let count: i64 = c.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0)).unwrap();
    assert_eq!(count, n, "rows in {table}");
}
