//! Event folder layout tests (DU-35).

use std::path::Path;

use host_core::db::{DbError, EventDb};
use host_core::event_folder::{
    create_event_folder, delete_event_folder, FolderError, INDEX_FILE, SUBFOLDERS,
};

const ID: &str = "evt_3f9a0c12b7de";

#[test]
fn schema_folder_layout_matches_du35() {
    let storage = tempfile::tempdir().unwrap();
    let f = create_event_folder(storage.path(), ID).unwrap();
    assert_eq!(f.root(), storage.path().join(ID));
    for sub in ["tls", "quarantine", "originals", "previews", "compatible"] {
        assert!(f.root().join(sub).is_dir(), "{sub}/ missing");
    }
    assert_eq!(SUBFOLDERS.len(), 5);
    assert_eq!(f.index_path(), f.root().join("index.sqlite"));
    assert_eq!(INDEX_FILE, "index.sqlite");
    // Creating it twice is an error, not a silent reuse.
    assert!(create_event_folder(storage.path(), ID).is_err());
}

#[cfg(unix)]
#[test]
fn schema_folder_is_owner_only_on_unix() {
    use std::os::unix::fs::PermissionsExt;
    let storage = tempfile::tempdir().unwrap();
    let f = create_event_folder(storage.path(), ID).unwrap();
    let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(f.root()), 0o700);
    assert_eq!(mode(&f.tls()), 0o700);
}

#[test]
fn schema_folder_rejects_ids_that_could_escape_storage() {
    let storage = tempfile::tempdir().unwrap();
    let outside = storage.path().parent().unwrap().to_owned();
    for bad in [
        "..",
        "../evt_3f9a0c12b7de",
        "evt_3f9a0c12b7de/..",
        "/etc",
        "/tmp/evt_3f9a0c12b7de",
        "",
        "evt_../../../x",
        "evt_3F9A0C12B7DE",
    ] {
        assert!(
            matches!(
                create_event_folder(storage.path(), bad),
                Err(FolderError::InvalidEventId(_))
            ),
            "create accepted {bad:?}"
        );
        assert!(
            matches!(
                delete_event_folder(storage.path(), bad),
                Err(FolderError::InvalidEventId(_))
            ),
            "delete accepted {bad:?}"
        );
    }
    assert!(outside.exists());
    assert_eq!(std::fs::read_dir(storage.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn schema_folder_delete_removes_everything_including_wal_and_shm() {
    let storage = tempfile::tempdir().unwrap();
    let f = create_event_folder(storage.path(), ID).unwrap();
    // A live index, so the -wal and -shm files really exist.
    let db = EventDb::open(&f.index_path()).unwrap();
    db.write(|tx| -> Result<(), DbError> {
        tx.execute(
            "INSERT INTO event_log (ts, action, result) VALUES (1, 'a', 'ok')",
            [],
        )?;
        Ok(())
    })
    .await
    .unwrap();
    let wal = f.root().join("index.sqlite-wal");
    let shm = f.root().join("index.sqlite-shm");
    assert!(
        wal.exists() && shm.exists(),
        "test needs real wal and shm files"
    );
    for sub in SUBFOLDERS {
        std::fs::write(f.root().join(sub).join("file.bin"), b"x").unwrap();
    }
    std::fs::create_dir(f.originals().join("nested")).unwrap();
    std::fs::write(f.originals().join("nested").join("deep.bin"), b"y").unwrap();

    delete_event_folder(storage.path(), ID).unwrap();
    assert!(!f.root().exists());
    assert_eq!(
        std::fs::read_dir(storage.path()).unwrap().count(),
        0,
        "storage must be empty"
    );
    // Deleting again is fine.
    delete_event_folder(storage.path(), ID).unwrap();
    drop(db);
    assert!(
        !f.root().exists(),
        "closing the handle must not bring files back"
    );
}
