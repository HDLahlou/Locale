//! Static rules over this crate's source: `unsafe` only in the macOS back end
//! (each block justified), no network APIs, no logging of names or paths, no
//! file opened except the output file the parent names, no `PATH` search.

use std::path::{Path, PathBuf};

fn src_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<(String, String)>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                let rel = p
                    .strip_prefix(src_dir())
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                out.push((rel, std::fs::read_to_string(&p).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(&src_dir(), &mut out);
    assert!(out.len() >= 8, "found {} sources", out.len());
    out
}

/// Source with `//` comments removed, so rules apply to code only.
fn code(text: &str) -> String {
    text.lines()
        .map(|l| match l.find("//") {
            Some(i) => &l[..i],
            None => l,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn unsafe_only_in_the_macos_back_end() {
    for (name, text) in sources() {
        let code = code(&text);
        if name == "macos.rs" {
            continue;
        }
        assert!(!code.contains("unsafe "), "{name} uses unsafe");
        let allows = code.matches("allow(unsafe_code)").count();
        if name == "lib.rs" {
            assert_eq!(allows, 1, "lib.rs allows unsafe once, on `mod macos`");
            assert!(code.contains("#[allow(unsafe_code)]\nmod macos;"));
        } else {
            assert_eq!(allows, 0, "{name} allows unsafe");
        }
    }
}

#[test]
fn every_unsafe_block_has_a_safety_comment() {
    let text = std::fs::read_to_string(src_dir().join("macos.rs")).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let mut blocks = 0;
    for (i, line) in lines.iter().enumerate() {
        if code(line).contains("unsafe {") {
            blocks += 1;
            // The nearest comment block above the statement (at most two code
            // lines up) must contain a SAFETY comment.
            let is_comment = |l: &&&str| l.trim_start().starts_with("//");
            let mut above = lines[..i].iter().rev().peekable();
            let mut code_lines = 0;
            while above.peek().is_some_and(|l| !is_comment(l)) && code_lines < 2 {
                above.next();
                code_lines += 1;
            }
            let justified = above
                .take_while(is_comment)
                .any(|l| l.contains("// SAFETY:"));
            assert!(
                justified,
                "macos.rs line {}: unsafe block without a SAFETY comment",
                i + 1
            );
        }
    }
    assert!(blocks > 0);
}

#[test]
fn no_network_apis() {
    for (name, text) in sources() {
        let code = code(&text);
        for banned in [
            "std::net",
            "TcpStream",
            "TcpListener",
            "UdpSocket",
            "UnixStream",
            "socket(",
        ] {
            assert!(!code.contains(banned), "{name} uses {banned}");
        }
    }
}

#[test]
fn nothing_is_logged() {
    for (name, text) in sources() {
        let code = code(&text);
        for banned in [
            "println!",
            "eprintln!",
            "print!",
            "eprint!",
            "dbg!",
            "log::",
            "tracing::",
        ] {
            assert!(!code.contains(banned), "{name} uses {banned}");
        }
    }
}

#[test]
fn child_opens_only_the_parent_named_output_file() {
    for (name, text) in sources() {
        let code = code(&text);
        let opens = code.matches("OpenOptions::new()").count()
            + code.matches("File::open(").count()
            + code.matches("File::create(").count()
            + code.matches("fs::read(").count()
            + code.matches("fs::write(").count();
        match name.as_str() {
            // `write_output`: the job's output path, a temp file the parent made.
            "child.rs" => {
                assert_eq!(opens, 1, "child.rs opens one file");
                assert!(code.contains(".create(false)") || !code.contains(".create("));
            }
            // Parent side: the input file (and /dev/null for a test job).
            "worker.rs" => assert!(opens <= 2, "worker.rs opens {opens} files"),
            // Unit-test fixtures only.
            "structure.rs" => {
                assert!(code[code.find("#[cfg(test)]").unwrap()..].contains("fs::read("))
            }
            _ => assert_eq!(opens, 0, "{name} opens files"),
        }
    }
}

#[test]
fn worker_never_searches_path() {
    let code = code(&std::fs::read_to_string(src_dir().join("worker.rs")).unwrap());
    assert_eq!(code.matches("Command::new(").count(), 1);
    assert!(code.contains("Command::new(&self.exe)"));
    assert!(code.contains(".env_clear()"));
    assert!(code.contains("is_absolute()"));
    for (name, text) in sources() {
        if name != "worker.rs" {
            assert!(!code_contains(&text, "Command::new("), "{name} spawns");
        }
    }
}

fn code_contains(text: &str, needle: &str) -> bool {
    code(text).contains(needle)
}
