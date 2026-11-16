//! Host app database schema: `event_template` and the `hosted_event` pointer.

use proto::model::{AccessType, EventStatus};

use super::{sql_list, Migration};

/// Migrations of the host app database.
pub fn app_migrations() -> Vec<Migration> {
    let access = sql_list(AccessType::ALL.iter().map(|v| v.as_str()));
    // The pointer only ever holds a running or paused event.
    let status = sql_list([EventStatus::Active.as_str(), EventStatus::Paused.as_str()]);
    vec![Migration {
        version: 1,
        sql: format!(
            "CREATE TABLE event_template (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                links TEXT,
                access_type TEXT NOT NULL CHECK (access_type IN ({access})),
                retention_hours INTEGER NOT NULL,
                browser_guests_allowed INTEGER NOT NULL CHECK (browser_guests_allowed IN (0,1)),
                last_used_at INTEGER
            );
            -- At most one row: the one active or paused event (D15, DU-17).
            CREATE TABLE hosted_event (
                slot INTEGER PRIMARY KEY NOT NULL CHECK (slot = 1),
                event_id TEXT NOT NULL,
                folder TEXT NOT NULL,
                status TEXT NOT NULL CHECK (status IN ({status})),
                updated_at INTEGER NOT NULL
            );"
        ),
    }]
}
