use super::*;

// RPC-407: build_service now mutates the process-global blocklist
// root (init_blocklist). Every test that calls build_service must be
// #[serial] so it can't race blocklist_init_tests (or each other).
use serial_test::serial;

/// Strip `#` line-comments from a Cargo.toml source string. Used by
/// the RPC-044 boundary regression so that prose like
/// `# RPC-067: shared no-codelet-napi dependency-rule helpers`
/// (added in dev-dependency comments) doesn't false-positive the
/// substring check.
fn strip_cargo_comments(src: &str) -> String {
    src.lines()
        .map(|line| match line.find('#') {
            Some(i) => &line[..i],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

mod build_service_cwd;
mod build_service_persistence;
mod part_1;
