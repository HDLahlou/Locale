//! Schema, enum CHECK and migration tests for the per-event index and the app database.

use std::collections::BTreeSet;
use std::path::Path;

use host_core::db::{
    app_migrations, event_migrations, migrate, schema_version, AppDb, DbError, EventDb,
};
use proto::codes::ItemReason;
use proto::model::{
    AccessType, ClientKind, EventKind, EventStatus, ItemState, PostKind, Role, SessionScope,
};
use rusqlite::Connection;

/// Digest §6 written out: table, then (column, declared type).
const EXPECTED_COLUMNS: &[(&str, &[(&str, &str)])] = &[
    (
        "event",
        &[
            ("id", "TEXT"),
            ("name", "TEXT"),
            ("description", "TEXT"),
            ("starts_at", "INTEGER"),
            ("ends_at", "INTEGER"),
            ("links", "TEXT"),
            ("access_type", "TEXT"),
            ("password_hash", "TEXT"),
            ("retention_hours", "INTEGER"),
            ("expires_at", "INTEGER"),
            ("extension_count", "INTEGER"),
            ("created_at", "INTEGER"),
            ("storage_limit_bytes", "INTEGER"),
            ("version", "INTEGER"),
            ("status", "TEXT"),
            ("kind", "TEXT"),
            ("max_ends_at", "INTEGER"),
            ("host_account_id", "TEXT"),
            ("host_display_name", "TEXT"),
            ("browser_guests_allowed", "INTEGER"),
            ("join_token", "TEXT"),
            ("storage_path", "TEXT"),
            ("port", "INTEGER"),
            ("config_version", "INTEGER"),
            ("config_json", "TEXT"),
        ],
    ),
    (
        "attendee",
        &[
            ("id", "TEXT"),
            ("event_id", "TEXT"),
            ("account_id", "TEXT"),
            ("guest_id", "TEXT"),
            ("display_name", "TEXT"),
            ("role", "TEXT"),
            ("is_host_device", "INTEGER"),
            ("client_kind", "TEXT"),
            ("joined_at", "INTEGER"),
            ("removed_at", "INTEGER"),
        ],
    ),
    (
        "session",
        &[
            ("token_hash", "TEXT"),
            ("attendee_id", "TEXT"),
            ("scope", "TEXT"),
            ("expires_at", "INTEGER"),
            ("revoked_at", "INTEGER"),
        ],
    ),
    (
        "item",
        &[
            ("id", "TEXT"),
            ("submission_id", "TEXT"),
            ("event_id", "TEXT"),
            ("uploader_id", "TEXT"),
            ("state", "TEXT"),
            ("reason_code", "TEXT"),
            ("content_type", "TEXT"),
            ("size_bytes", "INTEGER"),
            ("sha256", "TEXT"),
            ("width", "INTEGER"),
            ("height", "INTEGER"),
            ("has_hdr_gain_map", "INTEGER"),
            ("file_path", "TEXT"),
            ("preview_path", "TEXT"),
            ("compatible_path", "TEXT"),
            ("compatible_sha256", "TEXT"),
            ("download_count", "INTEGER"),
            ("hot_score", "REAL"),
            ("hot_score_updated_at", "INTEGER"),
            ("created_at", "INTEGER"),
            ("updated_at", "INTEGER"),
            ("last_progress_at", "INTEGER"),
            ("attempt_count", "INTEGER"),
            ("claimed_sha256", "TEXT"),
            ("original_name", "TEXT"),
            ("received_bytes", "INTEGER"),
            ("integrity_failures", "INTEGER"),
            ("available_at", "INTEGER"),
        ],
    ),
    (
        "change_log",
        &[
            ("version", "INTEGER"),
            ("event_id", "TEXT"),
            ("change_type", "TEXT"),
            ("payload", "TEXT"),
            ("created_at", "INTEGER"),
        ],
    ),
    (
        "post",
        &[
            ("id", "TEXT"),
            ("event_id", "TEXT"),
            ("author_id", "TEXT"),
            ("kind", "TEXT"),
            ("text", "TEXT"),
            ("created_at", "INTEGER"),
            ("deleted_at", "INTEGER"),
        ],
    ),
    (
        "event_log",
        &[
            ("id", "INTEGER"),
            ("ts", "INTEGER"),
            ("actor_id", "TEXT"),
            ("client_kind", "TEXT"),
            ("action", "TEXT"),
            ("target_id", "TEXT"),
            ("result", "TEXT"),
            ("reason", "TEXT"),
        ],
    ),
];

/// Expected indexes: table, name, unique, columns. The `sqlite_autoindex_*` rows
/// are the text primary keys.
const EXPECTED_INDEXES: &[(&str, &str, bool, &[&str])] = &[
    ("event", "sqlite_autoindex_event_1", true, &["id"]),
    ("attendee", "sqlite_autoindex_attendee_1", true, &["id"]),
    (
        "attendee",
        "attendee_event_account",
        true,
        &["event_id", "account_id"],
    ),
    (
        "attendee",
        "attendee_event_guest",
        true,
        &["event_id", "guest_id"],
    ),
    (
        "session",
        "sqlite_autoindex_session_1",
        true,
        &["token_hash"],
    ),
    ("item", "sqlite_autoindex_item_1", true, &["id"]),
    ("item", "item_submission_id", true, &["submission_id"]),
    (
        "item",
        "item_state_updated_at",
        false,
        &["state", "updated_at"],
    ),
    ("post", "sqlite_autoindex_post_1", true, &["id"]),
];

const EXPECTED_APP_COLUMNS: &[(&str, &[(&str, &str)])] = &[
    (
        "event_template",
        &[
            ("id", "TEXT"),
            ("name", "TEXT"),
            ("description", "TEXT"),
            ("links", "TEXT"),
            ("access_type", "TEXT"),
            ("retention_hours", "INTEGER"),
            ("browser_guests_allowed", "INTEGER"),
            ("last_used_at", "INTEGER"),
        ],
    ),
    (
        "hosted_event",
        &[
            ("slot", "INTEGER"),
            ("event_id", "TEXT"),
            ("folder", "TEXT"),
            ("status", "TEXT"),
            ("updated_at", "INTEGER"),
        ],
    ),
];

fn memory_db(migrations: &[host_core::db::Migration]) -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "foreign_keys", true).unwrap();
    migrate(&mut conn, migrations).unwrap();
    conn
}

fn tables(conn: &Connection) -> BTreeSet<String> {
    conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn columns(conn: &Connection, table: &str) -> Vec<(String, String)> {
    conn.prepare(&format!("PRAGMA table_info({table})"))
        .unwrap()
        .query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, String>(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn check_columns(conn: &Connection, expected: &[(&str, &[(&str, &str)])]) {
    let want_tables: BTreeSet<String> = expected.iter().map(|(t, _)| (*t).to_owned()).collect();
    assert_eq!(tables(conn), want_tables, "table list");
    for (table, cols) in expected {
        let got = columns(conn, table);
        let want: Vec<(String, String)> = cols
            .iter()
            .map(|(n, t)| ((*n).to_owned(), (*t).to_owned()))
            .collect();
        let got_names: BTreeSet<&str> = got.iter().map(|c| c.0.as_str()).collect();
        let want_names: BTreeSet<&str> = want.iter().map(|c| c.0.as_str()).collect();
        let missing: Vec<_> = want_names.difference(&got_names).collect();
        let extra: Vec<_> = got_names.difference(&want_names).collect();
        assert!(missing.is_empty(), "{table}: missing columns {missing:?}");
        assert!(extra.is_empty(), "{table}: extra columns {extra:?}");
        assert_eq!(got, want, "{table}: column types or order");
    }
}

#[test]
fn schema_event_index_has_exactly_the_digest_columns() {
    let conn = memory_db(&event_migrations());
    check_columns(&conn, EXPECTED_COLUMNS);
}

#[test]
fn schema_event_index_has_exactly_the_expected_indexes() {
    let conn = memory_db(&event_migrations());
    let mut got: BTreeSet<(String, String, bool, Vec<String>)> = BTreeSet::new();
    for (table, _) in EXPECTED_COLUMNS {
        let rows: Vec<(String, bool)> = conn
            .prepare(&format!("PRAGMA index_list({table})"))
            .unwrap()
            .query_map([], |r| {
                Ok((r.get::<_, String>(1)?, r.get::<_, i64>(2)? == 1))
            })
            .unwrap()
            .map(Result::unwrap)
            .collect();
        for (name, unique) in rows {
            let cols: Vec<String> = conn
                .prepare(&format!("PRAGMA index_info({name})"))
                .unwrap()
                .query_map([], |r| r.get(2))
                .unwrap()
                .map(Result::unwrap)
                .collect();
            got.insert(((*table).to_owned(), name, unique, cols));
        }
    }
    let want: BTreeSet<(String, String, bool, Vec<String>)> = EXPECTED_INDEXES
        .iter()
        .map(|(t, n, u, c)| {
            (
                (*t).to_owned(),
                (*n).to_owned(),
                *u,
                c.iter().map(|s| (*s).to_owned()).collect(),
            )
        })
        .collect();
    let missing: Vec<_> = want.difference(&got).collect();
    let extra: Vec<_> = got.difference(&want).collect();
    assert!(missing.is_empty(), "missing indexes {missing:?}");
    assert!(extra.is_empty(), "extra indexes {extra:?}");
}

#[test]
fn schema_app_database_has_template_and_hosted_event_tables() {
    let conn = memory_db(&app_migrations());
    check_columns(&conn, EXPECTED_APP_COLUMNS);
}

#[test]
fn schema_foreign_keys_point_at_parents() {
    let conn = memory_db(&event_migrations());
    let want: &[(&str, &str, &str)] = &[
        ("attendee", "event_id", "event"),
        ("session", "attendee_id", "attendee"),
        ("item", "event_id", "event"),
        ("item", "uploader_id", "attendee"),
        ("post", "event_id", "event"),
        ("post", "author_id", "attendee"),
        ("change_log", "event_id", "event"),
    ];
    for (table, from, parent) in want {
        let found = conn
            .prepare(&format!("PRAGMA foreign_key_list({table})"))
            .unwrap()
            .query_map([], |r| Ok((r.get::<_, String>(2)?, r.get::<_, String>(3)?)))
            .unwrap()
            .map(Result::unwrap)
            .any(|(p, f)| p == *parent && f == *from);
        assert!(found, "{table}.{from} must reference {parent}");
    }
}

/// Values of the quoted list after `<column> IN (` in a table's CREATE statement.
fn check_values(conn: &Connection, table: &str, column: &str) -> BTreeSet<String> {
    let sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name = ?1",
            [table],
            |r| r.get(0),
        )
        .unwrap();
    let marker = format!("{column} IN (");
    let start = sql
        .find(&marker)
        .unwrap_or_else(|| panic!("{table}.{column} has no CHECK list"))
        + marker.len();
    let end = start + sql[start..].find(')').unwrap();
    sql[start..end]
        .split(',')
        .map(|v| v.trim().trim_matches('\'').to_owned())
        .collect()
}

fn set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|s| (*s).to_owned()).collect()
}

fn wire<T: Copy>(all: &[T], f: impl Fn(T) -> &'static str) -> BTreeSet<String> {
    all.iter().map(|v| f(*v).to_owned()).collect()
}

/// Each row: table, column, the contract's values written out here, and the
/// values of the `proto` enum. A new enum value fails this test until the
/// literal list and a migration for the CHECK are added.
#[test]
fn schema_enum_checks_list_exactly_the_contract_values() {
    let conn = memory_db(&event_migrations());
    let rows: Vec<(&str, &str, BTreeSet<String>, BTreeSet<String>)> = vec![
        (
            "event",
            "access_type",
            set(&["open", "invite", "password"]),
            wire(AccessType::ALL, AccessType::as_str),
        ),
        (
            "event",
            "status",
            set(&["active", "paused", "ended"]),
            wire(EventStatus::ALL, EventStatus::as_str),
        ),
        (
            "event",
            "kind",
            set(&["event", "quick"]),
            wire(EventKind::ALL, EventKind::as_str),
        ),
        (
            "attendee",
            "role",
            set(&["host", "uploader", "viewer"]),
            wire(Role::ALL, Role::as_str),
        ),
        (
            "attendee",
            "client_kind",
            set(&["ios", "android", "desktop", "web"]),
            wire(ClientKind::ALL, ClientKind::as_str),
        ),
        (
            "event_log",
            "client_kind",
            set(&["ios", "android", "desktop", "web"]),
            wire(ClientKind::ALL, ClientKind::as_str),
        ),
        (
            "session",
            "scope",
            set(&["full", "read"]),
            wire(SessionScope::ALL, SessionScope::as_str),
        ),
        (
            "item",
            "state",
            set(&[
                "PENDING",
                "UPLOADING",
                "VALIDATING",
                "AVAILABLE",
                "FAILED",
                "DELETED",
            ]),
            wire(ItemState::ALL, ItemState::as_str),
        ),
        (
            "post",
            "kind",
            set(&["host", "system"]),
            wire(PostKind::ALL, PostKind::as_str),
        ),
        (
            "item",
            "reason_code",
            set(&[
                "type",
                "size",
                "storage_full",
                "abandoned",
                "stalled",
                "cancelled",
                "unauthorized",
                "invalid_image",
                "integrity",
                "duplicate",
            ]),
            wire(ItemReason::ALL, ItemReason::as_str),
        ),
    ];
    for (table, column, literal, from_proto) in rows {
        assert_eq!(
            literal, from_proto,
            "{table}.{column}: proto enum changed, update this test and add a migration"
        );
        assert_eq!(
            check_values(&conn, table, column),
            literal,
            "{table}.{column} CHECK"
        );
    }
    let app = memory_db(&app_migrations());
    assert_eq!(
        check_values(&app, "event_template", "access_type"),
        wire(AccessType::ALL, AccessType::as_str)
    );
    assert_eq!(
        check_values(&app, "hosted_event", "status"),
        set(&["active", "paused"])
    );
}

fn seed_event_and_attendee(conn: &Connection) {
    conn.execute_batch(
        "INSERT INTO event (id, name, access_type, retention_hours, created_at, storage_limit_bytes,
            version, status, kind, host_account_id, host_display_name, browser_guests_allowed,
            storage_path, config_version, config_json)
         VALUES ('evt_000000000001', 'E', 'open', 0, 1, 1, 1, 'active', 'event', 'a', 'H', 0, '/x', 1, '{}');
         INSERT INTO attendee (id, event_id, account_id, display_name, role, client_kind, joined_at)
         VALUES ('att1', 'evt_000000000001', 'acct1', 'H', 'host', 'desktop', 1);",
    )
    .unwrap();
}

#[test]
fn schema_rejects_values_outside_the_contract() {
    let conn = memory_db(&event_migrations());
    seed_event_and_attendee(&conn);
    let bad_state = conn.execute(
        "INSERT INTO item (id, submission_id, event_id, uploader_id, state, content_type, size_bytes, created_at, updated_at)
         VALUES ('i', 's', 'evt_000000000001', 'att1', 'pending', 'image/jpeg', 1, 1, 1)",
        [],
    );
    assert!(bad_state.is_err(), "lowercase state must be rejected");
    let bad_role = conn.execute(
        "INSERT INTO attendee (id, event_id, guest_id, display_name, role, client_kind, joined_at)
         VALUES ('att2', 'evt_000000000001', 'g', 'G', 'admin', 'web', 1)",
        [],
    );
    assert!(bad_role.is_err());
}

#[test]
fn schema_submission_id_and_attendee_identity_are_unique() {
    let conn = memory_db(&event_migrations());
    seed_event_and_attendee(&conn);
    let item = |id: &str| {
        conn.execute(
            "INSERT INTO item (id, submission_id, event_id, uploader_id, state, content_type, size_bytes, created_at, updated_at)
             VALUES (?1, 'same', 'evt_000000000001', 'att1', 'PENDING', 'image/jpeg', 1, 1, 1)",
            [id],
        )
    };
    item("i1").unwrap();
    assert!(item("i2").is_err(), "duplicate submission_id");

    let attendee = |id: &str, account: Option<&str>, guest: Option<&str>| {
        conn.execute(
            "INSERT INTO attendee (id, event_id, account_id, guest_id, display_name, role, client_kind, joined_at)
             VALUES (?1, 'evt_000000000001', ?2, ?3, 'X', 'viewer', 'web', 1)",
            rusqlite::params![id, account, guest],
        )
    };
    assert!(
        attendee("a2", Some("acct1"), None).is_err(),
        "same account twice"
    );
    attendee("a3", None, Some("g1")).unwrap();
    assert!(
        attendee("a4", None, Some("g1")).is_err(),
        "same guest twice"
    );
    attendee("a5", None, Some("g2")).unwrap();
    attendee("a6", Some("acct2"), None).unwrap();
    assert!(
        attendee("a7", None, None).is_err(),
        "needs an account or a guest ID"
    );
    assert!(attendee("a8", Some("a"), Some("b")).is_err(), "not both");
}

#[test]
fn schema_foreign_keys_are_enforced() {
    let conn = memory_db(&event_migrations());
    let r = conn.execute(
        "INSERT INTO post (id, event_id, kind, text, created_at) VALUES ('p', 'evt_missing', 'host', 't', 1)",
        [],
    );
    assert!(r.is_err());
}

#[test]
fn schema_change_log_versions_are_gapless() {
    let conn = memory_db(&event_migrations());
    seed_event_and_attendee(&conn);
    let insert = |version: Option<i64>| {
        conn.execute(
            "INSERT INTO change_log (version, event_id, change_type, payload, created_at)
             VALUES (?1, 'evt_000000000001', 't', '{}', 1)",
            [version],
        )
    };
    insert(None).unwrap();
    insert(Some(2)).unwrap();
    assert!(insert(Some(4)).is_err(), "a gap must be rejected");
    assert!(insert(Some(2)).is_err(), "a repeat must be rejected");
    insert(None).unwrap();
    let versions: Vec<i64> = conn
        .prepare("SELECT version FROM change_log ORDER BY version")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(versions, vec![1, 2, 3]);
}

#[test]
fn schema_event_log_target_is_nullable() {
    let conn = memory_db(&event_migrations());
    conn.execute(
        "INSERT INTO event_log (ts, action, target_id, result) VALUES (1, 'download', NULL, 'ok')",
        [],
    )
    .unwrap();
}

#[test]
fn schema_hosted_event_holds_at_most_one_row() {
    let conn = memory_db(&app_migrations());
    let put = |slot: i64| {
        conn.execute(
            "INSERT INTO hosted_event (slot, event_id, folder, status, updated_at) VALUES (?1, 'evt_000000000001', '/f', 'active', 1)",
            [slot],
        )
    };
    put(1).unwrap();
    assert!(put(1).is_err());
    assert!(put(2).is_err());
}

fn pragma_text(conn: &Connection, name: &str) -> String {
    conn.pragma_query_value(None, name, |r| r.get::<_, rusqlite::types::Value>(0))
        .map(|v| match v {
            rusqlite::types::Value::Text(t) => t,
            rusqlite::types::Value::Integer(i) => i.to_string(),
            other => format!("{other:?}"),
        })
        .unwrap()
}

#[tokio::test]
async fn schema_open_sets_wal_foreign_keys_and_synchronous() {
    let dir = tempfile::tempdir().unwrap();
    let db = EventDb::open(&dir.path().join("index.sqlite")).unwrap();
    let (journal, fk, sync, busy) = db
        .write(|tx| -> Result<_, DbError> {
            Ok((
                pragma_text(tx, "journal_mode"),
                pragma_text(tx, "foreign_keys"),
                pragma_text(tx, "synchronous"),
                pragma_text(tx, "busy_timeout"),
            ))
        })
        .await
        .unwrap();
    assert_eq!(journal, "wal");
    assert_eq!(fk, "1");
    assert_eq!(sync, "1", "NORMAL");
    assert!(busy.parse::<u32>().unwrap() > 0);
}

fn file_bytes(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap()
}

#[tokio::test]
async fn schema_reopening_an_up_to_date_database_applies_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite");
    let latest = event_migrations().last().unwrap().version;
    {
        let db = EventDb::open(&path).unwrap();
        let v = db.read(schema_version).await.unwrap();
        assert_eq!(v, latest);
    }
    // Checkpoint is done on last close; compare the main file across a reopen.
    let before = file_bytes(&path);
    let mut conn = Connection::open(&path).unwrap();
    let probe = event_migrations();
    migrate(&mut conn, &probe).unwrap();
    drop(conn);
    let _db = EventDb::open(&path).unwrap();
    assert_eq!(file_bytes(&path), before, "reopening must not write");
}

#[test]
fn schema_migrations_are_numbered_without_gaps() {
    for m in [event_migrations(), app_migrations()] {
        for (i, mig) in m.iter().enumerate() {
            assert_eq!(mig.version as usize, i + 1);
        }
    }
}

#[test]
fn schema_migrate_applies_only_newer_steps_each_in_a_transaction() {
    use host_core::db::Migration;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("m.sqlite");
    let m1 = Migration {
        version: 1,
        sql: "CREATE TABLE a (x INTEGER);".into(),
    };
    let m2 = Migration {
        version: 2,
        sql: "CREATE TABLE b (x INTEGER);".into(),
    };
    let broken = Migration {
        version: 3,
        sql: "CREATE TABLE c (x INTEGER); CREATE TABLE a (x INTEGER);".into(),
    };
    let mut conn = Connection::open(&path).unwrap();
    migrate(&mut conn, std::slice::from_ref(&m1)).unwrap();
    assert_eq!(schema_version(&conn).unwrap(), 1);
    migrate(&mut conn, &[m1.clone(), m2.clone()]).unwrap();
    assert_eq!(schema_version(&conn).unwrap(), 2);
    // Applying again is a no-op (it would fail on "table already exists" otherwise).
    migrate(&mut conn, &[m1.clone(), m2.clone()]).unwrap();
    // A failing step rolls back whole: no table c, version stays 2.
    assert!(migrate(&mut conn, &[m1, m2, broken]).is_err());
    assert_eq!(schema_version(&conn).unwrap(), 2);
    assert!(!tables(&conn).contains("c"));
}

#[test]
fn schema_newer_database_fails_with_typed_error_and_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite");
    let latest = event_migrations().last().unwrap().version;
    {
        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "user_version", latest + 1)
            .unwrap();
    }
    let before = file_bytes(&path);
    let err = EventDb::open(&path).unwrap_err();
    assert!(
        matches!(err, DbError::NewerSchema { found, supported } if found == latest + 1 && supported == latest),
        "{err:?}"
    );
    assert_eq!(file_bytes(&path), before);
    assert!(!dir.path().join("index.sqlite-wal").exists());
    let err = AppDb::open(&path).unwrap_err();
    assert!(matches!(err, DbError::NewerSchema { .. }));
}
