//! Writes the generated JSON Schema files and `CONTRACT.md` into `crates/proto`.
//!
//! Run with `cargo run -p proto --bin proto-gen`. Stale schema files that are no
//! longer generated are removed.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn main() -> std::io::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = proto::contract::generated_files();
    let schema_dir = root.join(proto::contract::SCHEMA_DIR);
    fs::create_dir_all(&schema_dir)?;

    let wanted: BTreeSet<_> = files.iter().map(|(p, _)| root.join(p)).collect();
    for entry in fs::read_dir(&schema_dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "json") && !wanted.contains(&path) {
            fs::remove_file(&path)?;
            println!("removed {}", path.display());
        }
    }
    for (rel, contents) in &files {
        let path = root.join(rel);
        if fs::read_to_string(&path).ok().as_deref() != Some(contents.as_str()) {
            fs::write(&path, contents)?;
            println!("wrote {}", path.display());
        }
    }
    println!("{} files up to date", files.len());
    Ok(())
}
