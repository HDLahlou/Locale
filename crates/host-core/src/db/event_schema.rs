//! Per-event index schema (digest §6, DU-34). Enum CHECK lists come from `proto`.

use proto::codes::ItemReason;
use proto::model::{
    AccessType, ClientKind, EventKind, EventStatus, ItemState, PostKind, Role, SessionScope,
};

use super::{sql_list, Migration};

/// Migrations of the per-event index.
pub fn event_migrations() -> Vec<Migration> {
    let access = sql_list(AccessType::ALL.iter().map(|v| v.as_str()));
    let status = sql_list(EventStatus::ALL.iter().map(|v| v.as_str()));
    let kind = sql_list(EventKind::ALL.iter().map(|v| v.as_str()));
    let role = sql_list(Role::ALL.iter().map(|v| v.as_str()));
    let client = sql_list(ClientKind::ALL.iter().map(|v| v.as_str()));
    let scope = sql_list(SessionScope::ALL.iter().map(|v| v.as_str()));
    let state = sql_list(ItemState::ALL.iter().map(|v| v.as_str()));
    let post = sql_list(PostKind::ALL.iter().map(|v| v.as_str()));
    let reason = sql_list(ItemReason::ALL.iter().map(|v| v.as_str()));
    vec![Migration {
        version: 1,
        sql: format!(
            "CREATE TABLE event (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                starts_at INTEGER,
                ends_at INTEGER,
                links TEXT,
                access_type TEXT NOT NULL CHECK (access_type IN ({access})),
                password_hash TEXT,
                retention_hours INTEGER NOT NULL,
                expires_at INTEGER,
                extension_count INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                storage_limit_bytes INTEGER NOT NULL,
                version INTEGER NOT NULL,
                status TEXT NOT NULL CHECK (status IN ({status})),
                kind TEXT NOT NULL CHECK (kind IN ({kind})),
                max_ends_at INTEGER,
                host_account_id TEXT NOT NULL,
                host_display_name TEXT NOT NULL,
                browser_guests_allowed INTEGER NOT NULL CHECK (browser_guests_allowed IN (0,1)),
                join_token TEXT,
                storage_path TEXT NOT NULL,
                port INTEGER,
                config_version INTEGER NOT NULL,
                config_json TEXT NOT NULL
            );

            -- Exactly one of account_id and guest_id is set (accounts for native
            -- clients, guest_id for browsers). Each is unique per event, so a
            -- re-join finds and reuses the row.
            CREATE TABLE attendee (
                id TEXT PRIMARY KEY NOT NULL,
                event_id TEXT NOT NULL REFERENCES event(id),
                account_id TEXT,
                guest_id TEXT,
                display_name TEXT NOT NULL,
                role TEXT NOT NULL CHECK (role IN ({role})),
                is_host_device INTEGER NOT NULL DEFAULT 0 CHECK (is_host_device IN (0,1)),
                client_kind TEXT NOT NULL CHECK (client_kind IN ({client})),
                joined_at INTEGER NOT NULL,
                removed_at INTEGER,
                CHECK ((account_id IS NULL) <> (guest_id IS NULL))
            );
            CREATE UNIQUE INDEX attendee_event_account ON attendee(event_id, account_id);
            CREATE UNIQUE INDEX attendee_event_guest ON attendee(event_id, guest_id);

            CREATE TABLE session (
                token_hash TEXT PRIMARY KEY NOT NULL,
                attendee_id TEXT NOT NULL REFERENCES attendee(id),
                scope TEXT NOT NULL CHECK (scope IN ({scope})),
                expires_at INTEGER NOT NULL,
                revoked_at INTEGER
            );

            CREATE TABLE item (
                id TEXT PRIMARY KEY NOT NULL,
                submission_id TEXT NOT NULL,
                event_id TEXT NOT NULL REFERENCES event(id),
                uploader_id TEXT NOT NULL REFERENCES attendee(id),
                state TEXT NOT NULL CHECK (state IN ({state})),
                reason_code TEXT CHECK (reason_code IS NULL OR reason_code IN ({reason})),
                content_type TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                sha256 TEXT,
                width INTEGER,
                height INTEGER,
                has_hdr_gain_map INTEGER NOT NULL DEFAULT 0 CHECK (has_hdr_gain_map IN (0,1)),
                file_path TEXT,
                preview_path TEXT,
                compatible_path TEXT,
                compatible_sha256 TEXT,
                download_count INTEGER NOT NULL DEFAULT 0,
                hot_score REAL NOT NULL DEFAULT 0,
                hot_score_updated_at INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                last_progress_at INTEGER,
                attempt_count INTEGER NOT NULL DEFAULT 0,
                claimed_sha256 TEXT,
                original_name TEXT,
                received_bytes INTEGER NOT NULL DEFAULT 0,
                integrity_failures INTEGER NOT NULL DEFAULT 0,
                available_at INTEGER
            );
            CREATE UNIQUE INDEX item_submission_id ON item(submission_id);
            CREATE INDEX item_state_updated_at ON item(state, updated_at);

            -- version is the rowid: an insert without it takes max+1, and the
            -- trigger rejects (aborting the insert) any explicit value that would leave a gap.
            CREATE TABLE change_log (
                version INTEGER PRIMARY KEY NOT NULL,
                event_id TEXT NOT NULL REFERENCES event(id),
                change_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );
            CREATE TRIGGER change_log_gapless AFTER INSERT ON change_log
            WHEN NEW.version <> (SELECT COUNT(*) FROM change_log)
            BEGIN
                SELECT RAISE(ABORT, 'change_log version must be max + 1');
            END;

            CREATE TABLE post (
                id TEXT PRIMARY KEY NOT NULL,
                event_id TEXT NOT NULL REFERENCES event(id),
                author_id TEXT REFERENCES attendee(id),
                kind TEXT NOT NULL CHECK (kind IN ({post})),
                text TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                deleted_at INTEGER
            );

            CREATE TABLE event_log (
                id INTEGER PRIMARY KEY NOT NULL,
                ts INTEGER NOT NULL,
                actor_id TEXT,
                client_kind TEXT CHECK (client_kind IS NULL OR client_kind IN ({client})),
                action TEXT NOT NULL,
                target_id TEXT,
                result TEXT NOT NULL,
                reason TEXT
            );"
        ),
    }]
}
