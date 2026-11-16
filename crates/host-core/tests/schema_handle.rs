//! Single-writer handle tests: transactions, rollback, snapshots, read-only reads.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

use host_core::db::{DbError, EventDb};

fn open() -> (tempfile::TempDir, EventDb) {
    let dir = tempfile::tempdir().unwrap();
    let db = EventDb::open(&dir.path().join("index.sqlite")).unwrap();
    (dir, db)
}

async fn count(db: &EventDb, table: &'static str) -> i64 {
    db.read(move |c| -> Result<i64, DbError> {
        Ok(c.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))?)
    })
    .await
    .unwrap()
}

fn log_row(tx: &rusqlite::Connection, tag: &str) -> rusqlite::Result<usize> {
    tx.execute(
        "INSERT INTO event_log (ts, action, result, reason) VALUES (1, 'a', 'ok', ?1)",
        [tag],
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn schema_handle_concurrent_writes_do_not_interleave() {
    let (_d, db) = open();
    let inside = Arc::new(AtomicUsize::new(0));
    let max_inside = Arc::new(AtomicUsize::new(0));
    let mut tasks = Vec::new();
    for tag in ["a", "b", "c", "d"] {
        let db = db.clone();
        let (inside, max_inside) = (inside.clone(), max_inside.clone());
        tasks.push(tokio::spawn(async move {
            db.write(move |tx| -> Result<(), DbError> {
                let now = inside.fetch_add(1, Ordering::SeqCst) + 1;
                max_inside.fetch_max(now, Ordering::SeqCst);
                log_row(tx, tag)?;
                std::thread::sleep(Duration::from_millis(30));
                log_row(tx, tag)?;
                inside.fetch_sub(1, Ordering::SeqCst);
                Ok(())
            })
            .await
        }));
    }
    for t in tasks {
        t.await.unwrap().unwrap();
    }
    assert_eq!(
        max_inside.load(Ordering::SeqCst),
        1,
        "two writes ran at once"
    );
    let tags: Vec<String> = db
        .read(|c| -> Result<_, DbError> {
            let mut s = c.prepare("SELECT reason FROM event_log ORDER BY id")?;
            let rows = s
                .query_map([], |r| r.get(0))?
                .collect::<Result<Vec<String>, _>>()?;
            Ok(rows)
        })
        .await
        .unwrap();
    assert_eq!(tags.len(), 8);
    for pair in tags.chunks(2) {
        assert_eq!(
            pair[0], pair[1],
            "rows of different writes interleaved: {tags:?}"
        );
    }
}

#[tokio::test]
async fn schema_handle_failed_write_leaves_no_partial_rows() {
    let (_d, db) = open();
    let r: Result<(), DbError> = db
        .write(|tx| {
            log_row(tx, "first")?;
            log_row(tx, "second")?;
            // Violates a CHECK, so the whole closure must roll back.
            tx.execute(
                "INSERT INTO event_log (ts, client_kind, action, result) VALUES (1, 'toaster', 'a', 'ok')",
                [],
            )?;
            Ok(())
        })
        .await;
    assert!(r.is_err());
    assert_eq!(count(&db, "event_log").await, 0);
}

#[derive(Debug)]
enum MyErr {
    Db(DbError),
    Business,
}
impl From<DbError> for MyErr {
    fn from(e: DbError) -> Self {
        MyErr::Db(e)
    }
}
impl From<rusqlite::Error> for MyErr {
    fn from(e: rusqlite::Error) -> Self {
        MyErr::Db(e.into())
    }
}

#[tokio::test]
async fn schema_handle_callers_error_type_rolls_back_too() {
    let (_d, db) = open();
    let r = db
        .write(|tx| -> Result<(), MyErr> {
            log_row(tx, "x")?;
            Err(MyErr::Business)
        })
        .await;
    match r {
        Err(MyErr::Business) => {}
        Err(MyErr::Db(e)) => panic!("unexpected db error: {e}"),
        Ok(()) => panic!("write should have failed"),
    }
    assert_eq!(count(&db, "event_log").await, 0);
}

#[tokio::test]
async fn schema_handle_panicking_write_rolls_back_and_handle_stays_usable() {
    let (_d, db) = open();
    let r = db
        .write(|tx| -> Result<(), DbError> {
            log_row(tx, "x")?;
            panic!("boom");
        })
        .await;
    assert!(matches!(r, Err(DbError::Task(_))));
    assert_eq!(count(&db, "event_log").await, 0);
    db.write(|tx| -> Result<(), DbError> {
        log_row(tx, "after")?;
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(count(&db, "event_log").await, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn schema_handle_reader_sees_a_consistent_snapshot_during_a_write() {
    let (_d, db) = open();
    db.write(|tx| -> Result<(), DbError> {
        log_row(tx, "old")?;
        Ok(())
    })
    .await
    .unwrap();

    // The writer inserts, then waits mid-transaction until told to go on.
    let (started_tx, started_rx) = mpsc::channel::<()>();
    let (go_tx, go_rx) = mpsc::channel::<()>();
    let writer = {
        let db = db.clone();
        tokio::spawn(async move {
            db.write(move |tx| -> Result<(), DbError> {
                log_row(tx, "new")?;
                started_tx.send(()).unwrap();
                go_rx.recv().unwrap();
                Ok(())
            })
            .await
        })
    };
    tokio::task::spawn_blocking(move || started_rx.recv().unwrap())
        .await
        .unwrap();

    // A reader during the open write transaction does not block and sees only old data.
    let during = tokio::time::timeout(Duration::from_secs(2), count(&db, "event_log"))
        .await
        .expect("reader blocked on the writer");
    assert_eq!(during, 1);

    // A reader whose snapshot began before the commit keeps it after the commit.
    let (snap_started_tx, snap_started_rx) = mpsc::channel::<()>();
    let (snap_go_tx, snap_go_rx) = mpsc::channel::<()>();
    let reader = {
        let db = db.clone();
        tokio::spawn(async move {
            db.read(move |c| -> Result<(i64, i64), DbError> {
                let n = |c: &rusqlite::Connection| -> rusqlite::Result<i64> {
                    c.query_row("SELECT COUNT(*) FROM event_log", [], |r| r.get(0))
                };
                let first = n(c)?;
                snap_started_tx.send(()).unwrap();
                snap_go_rx.recv().unwrap();
                Ok((first, n(c)?))
            })
            .await
        })
    };
    tokio::task::spawn_blocking(move || snap_started_rx.recv().unwrap())
        .await
        .unwrap();
    go_tx.send(()).unwrap();
    writer.await.unwrap().unwrap();
    assert_eq!(
        count(&db, "event_log").await,
        2,
        "commit is visible to new readers"
    );
    snap_go_tx.send(()).unwrap();
    let (first, second) = reader.await.unwrap().unwrap();
    assert_eq!((first, second), (1, 1), "snapshot changed mid-read");
}

#[tokio::test]
async fn schema_handle_write_through_a_read_connection_fails() {
    let (_d, db) = open();
    let r = db
        .read(|c| -> Result<(), DbError> {
            log_row(c, "nope")?;
            Ok(())
        })
        .await;
    assert!(r.is_err(), "writes must not work on a read connection");
    assert_eq!(count(&db, "event_log").await, 0);
}
