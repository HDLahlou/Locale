//! Event folder layout (DU-35):
//! `<storage_path>/<event_id>/{index.sqlite, tls/, quarantine/, originals/, previews/, compatible/}`.

use std::fs::DirBuilder;
use std::io;
use std::path::{Path, PathBuf};

use proto::transport::EventId;

/// Subfolders created inside every event folder.
pub const SUBFOLDERS: [&str; 5] = ["tls", "quarantine", "originals", "previews", "compatible"];

/// File name of the per-event index.
pub const INDEX_FILE: &str = "index.sqlite";

/// An event folder error.
#[derive(Debug, thiserror::Error)]
pub enum FolderError {
    /// The event ID is not a valid `evt_` ID, so it cannot name a folder.
    #[error("invalid event id: {0}")]
    InvalidEventId(#[from] proto::ParseError),
    /// A file system error.
    #[error("event folder: {0}")]
    Io(#[from] io::Error),
}

/// The paths inside one event folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventFolder {
    root: PathBuf,
}

impl EventFolder {
    /// The event folder itself.
    pub fn root(&self) -> &Path {
        &self.root
    }
    /// `index.sqlite`.
    pub fn index_path(&self) -> PathBuf {
        self.root.join(INDEX_FILE)
    }
    /// `tls/`.
    pub fn tls(&self) -> PathBuf {
        self.root.join("tls")
    }
    /// `quarantine/`.
    pub fn quarantine(&self) -> PathBuf {
        self.root.join("quarantine")
    }
    /// `originals/`.
    pub fn originals(&self) -> PathBuf {
        self.root.join("originals")
    }
    /// `previews/`.
    pub fn previews(&self) -> PathBuf {
        self.root.join("previews")
    }
    /// `compatible/`.
    pub fn compatible(&self) -> PathBuf {
        self.root.join("compatible")
    }
}

/// Where the folder of `event_id` lives. The ID is parsed first: only
/// `evt_` plus 12 hex digits gets through, so no `..`, separator or absolute
/// path can reach the join.
pub fn event_folder_path(storage_path: &Path, event_id: &str) -> Result<EventFolder, FolderError> {
    let id = EventId::parse(event_id)?;
    Ok(EventFolder {
        root: storage_path.join(id.as_str()),
    })
}

fn private_dir() -> DirBuilder {
    let mut b = DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        b.mode(0o700);
    }
    b
}

/// Creates the event folder and its subfolders, owner-only on Unix. Creates
/// `storage_path` if missing. Fails if the event folder already exists.
/// `index.sqlite` is created when the index is opened.
pub fn create_event_folder(
    storage_path: &Path,
    event_id: &str,
) -> Result<EventFolder, FolderError> {
    let folder = event_folder_path(storage_path, event_id)?;
    std::fs::create_dir_all(storage_path)?;
    let b = private_dir();
    b.create(folder.root())?;
    for sub in SUBFOLDERS {
        b.create(folder.root().join(sub))?;
    }
    Ok(folder)
}

/// Removes the event folder and everything in it, including the SQLite `-wal`
/// and `-shm` files. Succeeds if the folder is already gone.
pub fn delete_event_folder(storage_path: &Path, event_id: &str) -> Result<(), FolderError> {
    let folder = event_folder_path(storage_path, event_id)?;
    match std::fs::remove_dir_all(folder.root()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}
