//! The committed schema files and CONTRACT.md match freshly generated ones, and
//! the crate keeps its size rules.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use proto::contract::{generated_files, CONTRACT_PATH, REGENERATE_COMMAND, SCHEMA_DIR};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn generated_files_are_up_to_date() {
    let files = generated_files();
    let mut stale = Vec::new();
    for (rel, fresh) in &files {
        match fs::read_to_string(root().join(rel)) {
            Ok(committed) if committed == *fresh => {}
            Ok(_) => stale.push(format!("{rel} differs")),
            Err(_) => stale.push(format!("{rel} is missing")),
        }
    }
    let wanted: BTreeSet<String> = files.iter().map(|(p, _)| p.clone()).collect();
    for entry in fs::read_dir(root().join(SCHEMA_DIR)).expect("schema dir exists") {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        let rel = format!("{SCHEMA_DIR}/{name}");
        if !wanted.contains(&rel) {
            stale.push(format!("{rel} is no longer generated"));
        }
    }
    assert!(
        stale.is_empty(),
        "generated contract files are stale:\n  {}\nRegenerate them with `{REGENERATE_COMMAND}` and commit the result.",
        stale.join("\n  ")
    );
}

#[test]
fn generation_is_deterministic_and_complete() {
    assert_eq!(generated_files(), generated_files());
    let paths: Vec<String> = generated_files().into_iter().map(|(p, _)| p).collect();
    assert!(paths.contains(&CONTRACT_PATH.to_owned()));
    for name in [
        "error_body",
        "meta_response",
        "join_request",
        "join_response",
        "create_item_request",
        "create_item_response",
        "client_message",
        "server_message",
        "change",
        "auth_verify_request",
        "credential_payload",
        "config",
    ] {
        assert!(
            paths.contains(&format!("{SCHEMA_DIR}/{name}.json")),
            "{name}"
        );
    }
}

#[test]
fn contract_document_covers_every_code_and_row() {
    let doc = proto::contract::contract_markdown();
    for r in proto::codes::ItemReason::ALL {
        assert!(doc.contains(&format!("| `{}` |", r.as_str())), "{r}");
    }
    for c in proto::codes::ApiCode::ALL {
        assert!(doc.contains(&format!("`{}`", c.as_str())), "{c}");
    }
    for e in proto::api::ENDPOINTS {
        assert!(doc.contains(&format!("| `{}` | `{}` |", e.method, e.path)));
    }
    for b in proto::config::TUNABLES {
        assert!(doc.contains(&format!(
            "| `{}` | {} | {} | {} |",
            b.key, b.default, b.min, b.max
        )));
    }
    for t in proto::ws::ChangeType::ALL {
        assert!(doc.contains(&format!("| `{}` |", t.as_str())), "{t}");
    }
    for t in proto::state::TRANSITIONS {
        let from = t.from.map_or("(none)", |s| s.as_str());
        let row = format!("| {from} | {} | {} |", t.to.as_str(), t.trigger);
        assert!(doc.contains(&row), "{row}");
    }
    assert!(doc.contains("4F2A · 9C1E · 77B0"));
}

/// The crate is compiled into the phone app: no async runtime, networking, TLS,
/// database or crypto dependencies, except the one Ed25519 crate the credential
/// module uses (M1-2).
#[test]
fn dependencies_stay_small() {
    let manifest = fs::read_to_string(root().join("Cargo.toml")).unwrap();
    let deps_section = manifest
        .split("[dependencies]")
        .nth(1)
        .expect("has [dependencies]")
        .split("\n[")
        .next()
        .unwrap();
    let names: Vec<&str> = deps_section
        .lines()
        .filter_map(|l| l.split('=').next())
        .map(str::trim)
        .filter(|n| !n.is_empty() && !n.starts_with('#'))
        .collect();
    let signature_crate = "ed25519-dalek";
    let allowed = [
        "base64",
        signature_crate,
        "percent-encoding",
        "schemars",
        "serde",
        "serde_json",
        "thiserror",
    ];
    for n in &names {
        assert!(allowed.contains(n), "unexpected dependency {n}");
    }
    let forbidden = [
        "tokio",
        "async-std",
        "hyper",
        "reqwest",
        "axum",
        "http",
        "rustls",
        "openssl",
        "native-tls",
        "rusqlite",
        "sqlx",
        "ring",
        "ed25519",
        "sha2",
        "aws-lc",
    ];
    for f in forbidden {
        assert!(
            !names.iter().any(|n| *n != signature_crate && n.contains(f)),
            "forbidden dependency {f}"
        );
    }
}

#[test]
fn no_unsafe_code() {
    fn walk(dir: &Path, hits: &mut Vec<String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, hits);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = fs::read_to_string(&path).unwrap();
                if text.contains("unsafe {")
                    || text.contains("unsafe fn")
                    || text.contains("unsafe impl")
                {
                    hits.push(path.display().to_string());
                }
            }
        }
    }
    let mut hits = Vec::new();
    walk(&root().join("src"), &mut hits);
    assert!(hits.is_empty(), "unsafe code in {hits:?}");
}
