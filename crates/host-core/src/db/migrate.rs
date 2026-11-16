//! Forward-only migrations recorded in `PRAGMA user_version`.

use rusqlite::{Connection, TransactionBehavior};

use super::{DbError, Migration};

/// The schema version stored in the file (0 for a new file).
pub fn schema_version(conn: &Connection) -> Result<u32, DbError> {
    Ok(conn.pragma_query_value(None, "user_version", |r| r.get(0))?)
}

/// Applies every migration newer than the file's version, each in its own
/// transaction. An up-to-date file is left untouched; a file from a newer app
/// fails with [`DbError::NewerSchema`] before anything is changed.
pub fn migrate(conn: &mut Connection, migrations: &[Migration]) -> Result<(), DbError> {
    let supported = migrations.last().map_or(0, |m| m.version);
    let current = schema_version(conn)?;
    if current > supported {
        return Err(DbError::NewerSchema {
            found: current,
            supported,
        });
    }
    for m in migrations.iter().filter(|m| m.version > current) {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        // Re-check under the write lock so two openers cannot both apply it.
        if schema_version(&tx)? >= m.version {
            continue;
        }
        tx.execute_batch(&m.sql)?;
        tx.pragma_update(None, "user_version", m.version)?;
        tx.commit()?;
    }
    Ok(())
}
