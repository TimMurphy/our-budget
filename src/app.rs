//! Application state: the event store, whose read model tables the queries read from.

use crate::dates;
use crate::event_store::EventStore;
use crate::model::Event;
use std::path::PathBuf;

#[derive(Debug)]
pub struct App {
    pub store: EventStore,
    /// Folder holding the downloaded statement CSVs.
    pub statements_dir: PathBuf,
    /// Fixed date for tests.
    pub today_override: Option<String>,
    /// Statement rows dated before this ISO date are not imported. Empty imports everything.
    pub import_from: String,
    /// Events applied to the read model tables when the application started.
    pub replayed: usize,
    /// True when the read model tables were rebuilt from every event on start.
    pub rebuilt: bool,
}

impl App {
    /// Brings the read model tables up to date by applying only the events stored since they were
    /// last written. They are rebuilt from all events only when that is not possible.
    pub fn new(mut store: EventStore, statements_dir: PathBuf) -> Result<Self, String> {
        let (replayed, rebuilt) = store.catch_up()?;
        Ok(App { store, statements_dir, today_override: None, import_from: String::new(), replayed, rebuilt })
    }

    /// Appends events and updates the read model tables in the same transaction.
    pub fn record(&mut self, events: Vec<Event>) -> Result<(), String> {
        self.store.append(&events)
    }

    pub fn today(&self) -> String {
        self.today_override.clone().unwrap_or_else(dates::today)
    }
}
