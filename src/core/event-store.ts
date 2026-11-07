/** Hand-rolled event store on SQLite (node:sqlite). Append-only. */
import { mkdirSync } from "node:fs";
import { dirname } from "node:path";
import { DatabaseSync } from "node:sqlite";

export interface NewEvent {
  readonly type: string;
  readonly data: Record<string, unknown>;
}

export interface StoredEvent extends NewEvent {
  readonly position: number; // global, monotonic
  readonly streamId: string;
  readonly version: number; // per stream, starting at 1
  readonly recordedAt: string;
}

export class ConcurrencyError extends Error {}

export type Subscriber = (event: StoredEvent) => void;

export class EventStore {
  private readonly db: DatabaseSync;
  private readonly subscribers: Subscriber[] = [];

  constructor(path: string) {
    mkdirSync(dirname(path), { recursive: true });
    this.db = new DatabaseSync(path);
    this.db.exec(`
      PRAGMA journal_mode = WAL;
      CREATE TABLE IF NOT EXISTS events (
        position    INTEGER PRIMARY KEY AUTOINCREMENT,
        stream_id   TEXT    NOT NULL,
        version     INTEGER NOT NULL,
        type        TEXT    NOT NULL,
        data        TEXT    NOT NULL,
        recorded_at TEXT    NOT NULL,
        UNIQUE (stream_id, version)
      );
    `);
  }

  /**
   * Append events. expectedVersion is the stream's current version (0 = new stream),
   * or "any" to append regardless.
   */
  append(streamId: string, expectedVersion: number | "any", events: readonly NewEvent[]): StoredEvent[] {
    const stored: StoredEvent[] = [];
    this.db.exec("BEGIN IMMEDIATE");
    try {
      const row = this.db
        .prepare("SELECT COALESCE(MAX(version), 0) AS v FROM events WHERE stream_id = ?")
        .get(streamId) as { v: number };
      if (expectedVersion !== "any" && row.v !== expectedVersion) {
        throw new ConcurrencyError(
          `Stream '${streamId}' is at version ${row.v}, expected ${expectedVersion}`,
        );
      }
      const insert = this.db.prepare(
        "INSERT INTO events (stream_id, version, type, data, recorded_at) VALUES (?, ?, ?, ?, ?)",
      );
      let version = row.v;
      for (const e of events) {
        version++;
        const recordedAt = new Date().toISOString();
        const result = insert.run(streamId, version, e.type, JSON.stringify(e.data), recordedAt);
        stored.push({ ...e, streamId, version, recordedAt, position: Number(result.lastInsertRowid) });
      }
      this.db.exec("COMMIT");
    } catch (err) {
      this.db.exec("ROLLBACK");
      throw err;
    }
    for (const e of stored) this.subscribers.forEach((s) => s(e));
    return stored;
  }

  readStream(streamId: string): StoredEvent[] {
    return this.rows("WHERE stream_id = ? ORDER BY version", [streamId]);
  }

  readAll(afterPosition = 0): StoredEvent[] {
    return this.rows("WHERE position > ? ORDER BY position", [afterPosition]);
  }

  subscribe(subscriber: Subscriber): void {
    this.subscribers.push(subscriber);
  }

  close(): void {
    this.db.close();
  }

  private rows(where: string, params: (string | number)[]): StoredEvent[] {
    const rows = this.db
      .prepare(`SELECT position, stream_id, version, type, data, recorded_at FROM events ${where}`)
      .all(...params) as any[];
    return rows.map((r) => ({
      position: r.position,
      streamId: r.stream_id,
      version: r.version,
      type: r.type,
      data: JSON.parse(r.data),
      recordedAt: r.recorded_at,
    }));
  }
}
