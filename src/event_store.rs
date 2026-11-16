//! Hand-rolled, append-only event store on SQLite. The read model tables live in the same database,
//! and are updated in the same transaction as the events they reflect.

use crate::model::Event;
use crate::projection::{self, PROJECTION_VERSION};
use rusqlite::{params, Connection, OptionalExtension};
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
            );
            CREATE TABLE IF NOT EXISTS read_model_state (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                version INTEGER NOT NULL,
                last_seq INTEGER NOT NULL,
                event_count INTEGER NOT NULL,
                seq_sum INTEGER NOT NULL DEFAULT -1
            );
            CREATE INDEX IF NOT EXISTS events_type ON events (type);
            DROP TABLE IF EXISTS read_model;",
        )
        .map_err(err)?;
        // Databases from before the sum was kept get the column, and are rebuilt once because it does not match.
        let has_sum = conn
            .prepare("SELECT 1 FROM pragma_table_info('read_model_state') WHERE name = 'seq_sum'")
            .map_err(err)?
            .exists([])
            .map_err(err)?;
        if !has_sum {
            conn.execute("ALTER TABLE read_model_state ADD COLUMN seq_sum INTEGER NOT NULL DEFAULT -1", []).map_err(err)?;
        }
        projection::create(&conn)?;
        Ok(EventStore { conn })
    }

    /// The read model tables, for queries.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    fn insert(tx: &Connection, events: &[Event]) -> Result<(), String> {
        let at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        for e in events {
            let data = serde_json::to_string(e).map_err(err)?;
            tx.execute("INSERT INTO events (type, data, at) VALUES (?1, ?2, ?3)", params![e.kind(), data, at])
                .map_err(err)?;
        }
        Ok(())
    }

    /// Appends all events or none, and applies them to the read model tables in the same transaction.
    pub fn append(&mut self, events: &[Event]) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(err)?;
        Self::insert(&tx, events)?;
        for e in events {
            projection::apply(&tx, e)?;
        }
        save_state(&tx)?;
        tx.commit().map_err(err)
    }

    /// Appends events without touching the read model, which the next start then catches up.
    pub fn append_events_only(&mut self, events: &[Event]) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(err)?;
        Self::insert(&tx, events)?;
        tx.commit().map_err(err)
    }

    /// Brings the read model tables up to date. Normally this applies only the events stored since
    /// the tables were last written. It rebuilds the tables from every event only when there is no
    /// record of them, they are from a different projection version, or the stored events no longer
    /// match them (for example events were deleted). Returns (events applied, rebuilt).
    pub fn catch_up(&mut self) -> Result<(usize, bool), String> {
        let state: Option<(u32, i64, i64, i64)> = self
            .conn
            .query_row("SELECT version, last_seq, event_count, seq_sum FROM read_model_state WHERE id = 1", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })
            .optional()
            .map_err(err)?;
        let mut resume_after = None;
        if let Some((version, last_seq, event_count, seq_sum)) = state {
            // The count and the sum of positions both have to match, so events that were deleted, or
            // replaced by others, behind the application's back are noticed.
            let (stored, stored_sum): (i64, i64) = self
                .conn
                .query_row("SELECT COUNT(*), COALESCE(SUM(seq), 0) FROM events WHERE seq <= ?1", params![last_seq], |r| Ok((r.get(0)?, r.get(1)?)))
                .map_err(err)?;
            if version == PROJECTION_VERSION && stored == event_count && stored_sum == seq_sum {
                resume_after = Some(last_seq);
            }
        }
        let rebuilt = resume_after.is_none();
        let events = self.load_since(resume_after.unwrap_or(0))?;
        if rebuilt || !events.is_empty() {
            let tx = self.conn.transaction().map_err(err)?;
            if rebuilt {
                projection::reset(&tx)?;
            }
            for e in &events {
                projection::apply(&tx, e)?;
            }
            save_state(&tx)?;
            tx.commit().map_err(err)?;
        }
        Ok((events.len(), rebuilt))
    }

    pub fn is_empty(&self) -> Result<bool, String> {
        let n: i64 = self.conn.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0)).map_err(err)?;
        Ok(n == 0)
    }

    pub fn load_all(&self) -> Result<Vec<Event>, String> {
        self.load_since(0)
    }

    /// Events stored after position `seq`, oldest first.
    pub fn load_since(&self, seq: i64) -> Result<Vec<Event>, String> {
        let mut stmt = self.conn.prepare("SELECT data FROM events WHERE seq > ?1 ORDER BY seq").map_err(err)?;
        let rows = stmt.query_map(params![seq], |r| r.get::<_, String>(0)).map_err(err)?;
        let mut out = vec![];
        for r in rows {
            out.push(serde_json::from_str(&r.map_err(err)?).map_err(err)?);
        }
        Ok(out)
    }
}

/// Records that the read model tables include every stored event.
fn save_state(tx: &Connection) -> Result<(), String> {
    let (last_seq, count, seq_sum): (i64, i64, i64) = tx
        .query_row("SELECT COALESCE(MAX(seq), 0), COUNT(*), COALESCE(SUM(seq), 0) FROM events", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .map_err(err)?;
    tx.execute(
        "INSERT INTO read_model_state (id, version, last_seq, event_count, seq_sum) VALUES (1, ?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET version = ?1, last_seq = ?2, event_count = ?3, seq_sum = ?4",
        params![PROJECTION_VERSION, last_seq, count, seq_sum],
    )
    .map_err(err)?;
    Ok(())
}
