//! The read models: SQL tables kept up to date by applying each event as it is stored.

use crate::model::{Event, INCOME, UNGROUPED};
use rusqlite::{params, Connection};

/// Bump when the tables or what the projection writes to them changes. A different version
/// rebuilds the tables once from the events.
pub const PROJECTION_VERSION: u32 = 2;

const TABLES: [&str; 8] = [
    "bank_accounts",
    "transactions",
    "assignments",
    "import_failures",
    "groups",
    "budget_accounts",
    "budgets",
    "auto_assign_rules",
];

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS bank_accounts (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, bank TEXT NOT NULL, file TEXT NOT NULL, opening_cents INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS transactions (
    id TEXT PRIMARY KEY, account TEXT NOT NULL, date TEXT NOT NULL, cents INTEGER NOT NULL,
    description TEXT NOT NULL, link TEXT, pending INTEGER NOT NULL DEFAULT 0);
CREATE INDEX IF NOT EXISTS transactions_account ON transactions (account, date);
CREATE TABLE IF NOT EXISTS assignments (
    transaction_id TEXT NOT NULL, budget_account TEXT NOT NULL, cents INTEGER NOT NULL);
CREATE INDEX IF NOT EXISTS assignments_transaction ON assignments (transaction_id);
CREATE TABLE IF NOT EXISTS import_failures (
    account TEXT NOT NULL, file TEXT NOT NULL, row INTEGER NOT NULL, reason TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS groups (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, position INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS budget_accounts (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, group_id TEXT NOT NULL, closed INTEGER NOT NULL DEFAULT 0,
    auto_period TEXT, auto_cents INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS budgets (
    account TEXT NOT NULL, month TEXT NOT NULL, cents INTEGER NOT NULL, PRIMARY KEY (account, month));
CREATE TABLE IF NOT EXISTS auto_assign_rules (
    id TEXT PRIMARY KEY, phrase TEXT NOT NULL, account TEXT NOT NULL);
";

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Creates any missing tables and the two built-in rows: the Ungrouped group and the Income account.
pub fn create(c: &Connection) -> Result<(), String> {
    c.execute_batch(SCHEMA).map_err(err)?;
    c.execute("INSERT OR IGNORE INTO groups (id, name, position) VALUES (?1, 'Ungrouped', 0)", params![UNGROUPED]).map_err(err)?;
    c.execute(
        "INSERT OR IGNORE INTO budget_accounts (id, name, group_id, closed, auto_cents) VALUES (?1, 'Income', '', 0, 0)",
        params![INCOME],
    )
    .map_err(err)?;
    Ok(())
}

/// Empties the read model so it can be rebuilt from the events.
pub fn reset(c: &Connection) -> Result<(), String> {
    for t in TABLES {
        c.execute(&format!("DROP TABLE IF EXISTS {t}"), []).map_err(err)?;
    }
    create(c)
}

pub fn apply(c: &Connection, e: &Event) -> Result<(), String> {
    match e {
        Event::BankAccountRegistered { id, name, bank, file, opening_cents } => {
            c.execute(
                "INSERT INTO bank_accounts (id, name, bank, file, opening_cents) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, name, bank, file, opening_cents],
            )
        }
        Event::OpeningBalanceSet { account, cents } => {
            c.execute("UPDATE bank_accounts SET opening_cents = ?2 WHERE id = ?1", params![account, cents])
        }
        Event::TransactionImported { id, account, date, cents, description } => c.execute(
            "INSERT INTO transactions (id, account, date, cents, description) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, account, date, cents, description],
        ),
        Event::ImportRowFailed { account, file, row, reason } => c.execute(
            "INSERT INTO import_failures (account, file, row, reason) VALUES (?1, ?2, ?3, ?4)",
            params![account, file, row, reason],
        ),
        Event::GroupAdded { id, name } => c.execute(
            "INSERT INTO groups (id, name, position) VALUES (?1, ?2, (SELECT COALESCE(MAX(position), 0) + 1 FROM groups))",
            params![id, name],
        ),
        Event::GroupRenamed { id, name } => c.execute("UPDATE groups SET name = ?2 WHERE id = ?1", params![id, name]),
        Event::GroupRemoved { id } => c.execute("DELETE FROM groups WHERE id = ?1", params![id]),
        Event::GroupsReordered { ids } => {
            // Listed groups first, in the order given; any others keep their order after them.
            c.execute("UPDATE groups SET position = position + 1000000", []).map_err(err)?;
            for (i, id) in ids.iter().enumerate() {
                c.execute("UPDATE groups SET position = ?2 WHERE id = ?1", params![id, i as i64 + 1]).map_err(err)?;
            }
            Ok(0)
        }
        Event::BudgetAccountCreated { id, name, group } => c.execute(
            "INSERT INTO budget_accounts (id, name, group_id, closed, auto_cents) VALUES (?1, ?2, ?3, 0, 0)",
            params![id, name, group],
        ),
        Event::BudgetAccountRenamed { id, name } => {
            c.execute("UPDATE budget_accounts SET name = ?2 WHERE id = ?1", params![id, name])
        }
        Event::BudgetAccountClosed { id } => c.execute("UPDATE budget_accounts SET closed = 1 WHERE id = ?1", params![id]),
        Event::BudgetAccountMoved { id, group } => {
            c.execute("UPDATE budget_accounts SET group_id = ?2 WHERE id = ?1", params![id, group])
        }
        Event::Assigned { transaction, splits } => {
            c.execute("DELETE FROM assignments WHERE transaction_id = ?1", params![transaction]).map_err(err)?;
            c.execute("UPDATE transactions SET pending = 0 WHERE id = ?1", params![transaction]).map_err(err)?;
            for s in splits {
                c.execute(
                    "INSERT INTO assignments (transaction_id, budget_account, cents) VALUES (?1, ?2, ?3)",
                    params![transaction, s.account, s.cents],
                )
                .map_err(err)?;
            }
            Ok(0)
        }
        Event::TransferLinked { a, b } => {
            // A linked transaction is never pending approval.
            c.execute("UPDATE transactions SET pending = 0 WHERE id = ?1 OR id = ?2", params![a, b]).map_err(err)?;
            c.execute("UPDATE transactions SET link = ?2 WHERE id = ?1", params![a, b]).map_err(err)?;
            c.execute("UPDATE transactions SET link = ?2 WHERE id = ?1", params![b, a])
        }
        Event::TransferUnlinked { a, b } => {
            c.execute("UPDATE transactions SET link = NULL WHERE id = ?1 OR id = ?2", params![a, b])
        }
        Event::BudgetSet { account, month, cents } => c.execute(
            "INSERT INTO budgets (account, month, cents) VALUES (?1, ?2, ?3)
             ON CONFLICT(account, month) DO UPDATE SET cents = ?3",
            params![account, month, cents],
        ),
        Event::AutoAssignRuleCreated { id, phrase, account } => c.execute(
            "INSERT INTO auto_assign_rules (id, phrase, account) VALUES (?1, ?2, ?3)",
            params![id, phrase, account],
        ),
        Event::AutoAssignRuleRemoved { id } => c.execute("DELETE FROM auto_assign_rules WHERE id = ?1", params![id]),
        Event::AutoAssigned { transaction, account, cents, .. } => {
            c.execute("DELETE FROM assignments WHERE transaction_id = ?1", params![transaction]).map_err(err)?;
            c.execute(
                "INSERT INTO assignments (transaction_id, budget_account, cents) VALUES (?1, ?2, ?3)",
                params![transaction, account, cents],
            )
            .map_err(err)?;
            c.execute("UPDATE transactions SET pending = 1 WHERE id = ?1", params![transaction])
        }
        Event::TransactionApproved { transaction } => {
            c.execute("UPDATE transactions SET pending = 0 WHERE id = ?1", params![transaction])
        }
        Event::AutoMonthlySet { account, period, cents } => c.execute(
            "UPDATE budget_accounts SET auto_period = ?2, auto_cents = ?3 WHERE id = ?1",
            params![account, period, cents],
        ),
    }
    .map(|_| ())
    .map_err(err)
}
