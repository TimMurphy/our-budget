//! Application state: the event store plus the read model built from it.

use crate::dates;
use crate::event_store::EventStore;
use crate::model::{Event, Model};
use std::path::PathBuf;

#[derive(Debug)]
pub struct App {
    pub store: EventStore,
    pub model: Model,
    /// Folder holding the downloaded statement CSVs.
    pub statements_dir: PathBuf,
    /// Fixed date for tests.
    pub today_override: Option<String>,
}

impl App {
    pub fn new(store: EventStore, statements_dir: PathBuf) -> Result<Self, String> {
        let mut model = Model::default();
        for e in store.load_all()? {
            model.apply(&e);
        }
        Ok(App { store, model, statements_dir, today_override: None })
    }

    /// Appends events, then applies them to the read model.
    pub fn record(&mut self, events: Vec<Event>) -> Result<(), String> {
        self.store.append(&events)?;
        for e in &events {
            self.model.apply(e);
        }
        Ok(())
    }

    pub fn today(&self) -> String {
        self.today_override.clone().unwrap_or_else(dates::today)
    }
}
