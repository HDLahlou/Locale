//! SQLite storage for the host (D9: the host process is the only writer).
//!
//! Two databases share one handle type, [`Db`]:
//! * [`EventDb`]: the per-event index, `index.sqlite` inside the event folder.
//! * [`AppDb`]: the host app database, outside any event folder.
//!
//! All writes go through one connection, one closure per transaction. Reads use
//! their own read-only connections, so under WAL they never wait for the writer.
//! Both are async: the work runs on the blocking pool.

mod app_schema;
mod event_schema;
mod migrate;

use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags, Transaction};

pub use app_schema::app_migrations;
pub use event_schema::event_migrations;
pub use migrate::{migrate, schema_version};

/// How long a connection waits on a locked database before failing.
pub const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// A database error.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    /// SQLite reported an error.
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// The file was written by a newer app version; nothing was changed.
    #[error("database schema version {found} is newer than this app supports ({supported})")]
    NewerSchema {
        /// Version found in the file.
        found: u32,
        /// Newest version this build knows.
        supported: u32,
    },
    /// A blocking task panicked or was cancelled.
    #[error("database task failed: {0}")]
    Task(String),
}

/// One numbered, forward-only migration.
#[derive(Debug, Clone)]
pub struct Migration {
    /// Schema version after this migration. Versions start at 1 and have no gaps.
    pub version: u32,
    /// SQL run in one transaction.
    pub sql: String,
}

/// Single-writer handle to one SQLite file.
#[derive(Debug, Clone)]
pub struct Db {
    path: Arc<PathBuf>,
    writer: Arc<Mutex<Connection>>,
}

impl Db {
    fn open(path: &Path, migrations: &[Migration]) -> Result<Db, DbError> {
        let mut conn = Connection::open(path)?;
        // Check the version before any pragma that could change the file.
        let current = schema_version(&conn)?;
        let supported = migrations.last().map_or(0, |m| m.version);
        if current > supported {
            return Err(DbError::NewerSchema {
                found: current,
                supported,
            });
        }
        conn.busy_timeout(BUSY_TIMEOUT)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", true)?;
        migrate(&mut conn, migrations)?;
        Ok(Db {
            path: Arc::new(path.to_owned()),
            writer: Arc::new(Mutex::new(conn)),
        })
    }

    fn lock_writer(writer: &Mutex<Connection>) -> MutexGuard<'_, Connection> {
        // A panic in an earlier closure already rolled its transaction back.
        writer.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Runs `f` in one transaction on the single write connection. It commits if
    /// `f` returns `Ok` and rolls back as a whole if it returns `Err` or panics.
    pub async fn write<T, E, F>(&self, f: F) -> Result<T, E>
    where
        F: FnOnce(&Transaction<'_>) -> Result<T, E> + Send + 'static,
        T: Send + 'static,
        E: From<DbError> + Send + 'static,
    {
        let writer = Arc::clone(&self.writer);
        tokio::task::spawn_blocking(move || {
            let mut conn = Db::lock_writer(&writer);
            let tx = conn.transaction().map_err(DbError::from)?;
            let value = f(&tx)?;
            tx.commit().map_err(DbError::from)?;
            Ok(value)
        })
        .await
        .map_err(|e| E::from(DbError::Task(e.to_string())))?
    }

    /// Runs `f` on a fresh read-only connection inside one read transaction, so
    /// every query in `f` sees the same snapshot. Writes fail.
    pub async fn read<T, E, F>(&self, f: F) -> Result<T, E>
    where
        F: FnOnce(&Connection) -> Result<T, E> + Send + 'static,
        T: Send + 'static,
        E: From<DbError> + Send + 'static,
    {
        let path = Arc::clone(&self.path);
        tokio::task::spawn_blocking(move || {
            let mut conn = Connection::open_with_flags(
                path.as_path(),
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )
            .map_err(DbError::from)?;
            conn.busy_timeout(BUSY_TIMEOUT).map_err(DbError::from)?;
            let tx = conn.transaction().map_err(DbError::from)?;
            f(&tx)
        })
        .await
        .map_err(|e| E::from(DbError::Task(e.to_string())))?
    }

    /// Path of the database file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// The per-event index (`index.sqlite`).
#[derive(Debug, Clone)]
pub struct EventDb(Db);

impl EventDb {
    /// Opens (creating if needed) and migrates the index at `path`.
    pub fn open(path: &Path) -> Result<EventDb, DbError> {
        Db::open(path, &event_migrations()).map(EventDb)
    }
}

impl Deref for EventDb {
    type Target = Db;
    fn deref(&self) -> &Db {
        &self.0
    }
}

/// The host app database: event templates and the hosted-event pointer.
#[derive(Debug, Clone)]
pub struct AppDb(Db);

impl AppDb {
    /// Opens (creating if needed) and migrates the app database at `path`.
    pub fn open(path: &Path) -> Result<AppDb, DbError> {
        Db::open(path, &app_migrations()).map(AppDb)
    }
}

impl Deref for AppDb {
    type Target = Db;
    fn deref(&self) -> &Db {
        &self.0
    }
}

/// SQL list of wire strings for a CHECK constraint: `'a','b'`. The values are
/// compile-time constants from `proto`, never user input.
pub(crate) fn sql_list<'a>(values: impl IntoIterator<Item = &'a str>) -> String {
    values
        .into_iter()
        .map(|v| format!("'{v}'"))
        .collect::<Vec<_>>()
        .join(",")
}
