// Each integration test binary compiles this module separately and uses only part of it, so unused
// helpers here are expected rather than dead.
#![allow(dead_code)]

//! Shared fixture loading for the integration tests.
//!
//! `crates/core` embeds no data ([#19](https://github.com/Furizaa/poe-graft/issues/19)), so the
//! tests reach the repository's `data/` and `tests/fixtures/` by path relative to the crate.

use std::path::{Path, PathBuf};

use poe_graft_core::ModPool;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the crate sits two levels below the repository root")
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The shipped mod pool, read from `data/ghastly-eye-jewel.json`.
pub fn pool() -> ModPool {
    let path = repo_root().join("data/ghastly-eye-jewel.json");
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    ModPool::from_json(&json).expect("the shipped pool parses")
}

/// One capture by file name, e.g. `capture("spike-17/05-annealed-of-order.txt")`.
pub fn capture(rel: &str) -> String {
    let path = fixtures().join("captures").join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

/// Every capture in a set, as `(file name, Item Text)`, sorted by file name.
pub fn capture_set(set: &str) -> Vec<(String, String)> {
    let dir = fixtures().join("captures").join(set);
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()))
        .map(|e| e.expect("a readable directory entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .map(|p| {
            let name = p
                .file_name()
                .expect("a capture has a file name")
                .to_string_lossy()
                .into_owned();
            (
                name,
                std::fs::read_to_string(&p).expect("a readable capture"),
            )
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(!out.is_empty(), "no captures found in {}", dir.display());
    out
}

/// Strip an Item Text back to what the game shows with **Advanced Mod Descriptions off**: no
/// `{ … Modifier … }` annotation lines, and no inline `(lo-hi)` bounds after a rolled value.
///
/// This is a belief about what that client setting does, not an observation — every real capture we
/// have was taken with the setting on. See `tests/fixtures/README.md`.
pub fn without_advanced_descriptions(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        if line.starts_with('{') {
            continue;
        }
        out.push_str(&strip_inline_bounds(line));
        out.push('\n');
    }
    out
}

/// Remove `(lo-hi)` where it directly follows a digit — `12(9-12) to 18(15-18)` becomes `12 to 18`.
/// Leaves `(Recently refers to the past 4 seconds)` alone, because no digit precedes it.
fn strip_inline_bounds(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < line.len() {
        if bytes[i] == b'(' && out.chars().next_back().is_some_and(|c| c.is_ascii_digit()) {
            if let Some(close) = line[i..].find(')') {
                let inner = &line[i + 1..i + close];
                let numeric = |s: &str| {
                    !s.is_empty()
                        && s.chars()
                            .all(|c| c.is_ascii_digit() || c == '.' || c == '-')
                };
                if numeric(inner) {
                    i += close + 1;
                    continue;
                }
            }
        }
        let ch = line[i..].chars().next().expect("in bounds");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}
