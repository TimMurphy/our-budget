//! Hand-rolled, append-only event store on SQLite.

use crate::model::Event;
use rusqlite::{params, Connection};
use std::path::Path;

#[derive(Debug)]
pub struct EventStore {
    conn: Connection,
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

impl EventStore {
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(err)?;
        }
        Self::init(Connection::open(path).map_err(err)?)
    }

    pub fn open_in_memory() -> Result<Self, String> {
        Self::init(Connection::open_in_memory().map_err(err)?)
    }

    fn init(conn: Connection) -> Result<Self, String> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS events (
                seq INTEGER PRIMARY KEY AUTOINCREMENT,
                type TEXT NOT NULL,
                data TEXT NOT NULL,
                at INTEGER NOT NULL
            );",
        )
        .map_err(err)?;
        Ok(EventStore { conn })
    }

    /// Appends all events or none.
    pub fn append(&mut self, events: &[Event]) -> Result<(), String> {
        let at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let tx = self.conn.transaction().map_err(err)?;
        for e in events {
            let data = serde_json::to_string(e).map_err(err)?;
            tx.execute("INSERT INTO events (type, data, at) VALUES (?1, ?2, ?3)", params![e.kind(), data, at])
                .map_err(err)?;
        }
        tx.commit().map_err(err)
    }

    pub fn load_all(&self) -> Result<Vec<Event>, String> {
        let mut stmt = self.conn.prepare("SELECT data FROM events ORDER BY seq").map_err(err)?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(err)?;
        let mut out = vec![];
        for r in rows {
            out.push(serde_json::from_str(&r.map_err(err)?).map_err(err)?);
        }
        Ok(out)
    }
}
