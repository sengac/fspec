//! Source-reading helpers shared by the RPC-070 and RPC-073 shape tests
//! (`rpc070_create_session_no_panic.rs`, `rpc073_clear_history_no_panic.rs`).
//! A sub-directory module under `tests/` is not its own test binary.

use std::path::PathBuf;

/// Workspace root (one level above `rust/fspec/`).
fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("project root walk-up")
}

fn handle_impl_path() -> PathBuf {
    workspace_root()
        .join("rust")
        .join("sessions")
        .join("src")
        .join("handle_impl.rs")
}

pub fn read_handle_impl_src() -> String {
    let p = handle_impl_path();
    codelet_test_helpers::source_composition::read_tree_or_file(&p)
        .unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

/// Strip `//`, `///`, `//!` line comments and `/* … */` block comments
/// from a Rust source body so source-shape assertions aren't fooled by
/// the panic-prone idiom appearing inside rustdoc that *describes* the
/// pattern. Mirrors `rust/fspec/tests/common/mod.rs::strip_comments`
/// — duplicated here so this test binary stays standalone (no
/// `mod common`).
pub fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' {
            match chars.peek() {
                Some('/') => {
                    chars.next();
                    for ch in chars.by_ref() {
                        if ch == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                    continue;
                }
                Some('*') => {
                    chars.next();
                    let mut prev = ' ';
                    for ch in chars.by_ref() {
                        if prev == '*' && ch == '/' {
                            break;
                        }
                        prev = ch;
                    }
                    continue;
                }
                _ => {}
            }
        }
        out.push(c);
    }
    out
}

/// Slice out the body of the specified `fn NAME(` or `fn NAME<...>(`
/// definition inside the `impl ... SessionManagerHandle for
/// SessionManager { ... }` block (or a free function). Returns the
/// substring from `fn NAME` up to the matching closing brace at
/// indentation level four (i.e. the function's own closing brace).
/// Light-weight brace-counter — sufficient for the shape assertions
/// below, which only care about substring presence.
pub fn extract_method_body<'src>(src: &'src str, fn_name: &str) -> &'src str {
    let needle = format!("fn {fn_name}");
    let mut search = 0usize;
    let start = loop {
        let rel = src[search..]
            .find(&needle)
            .unwrap_or_else(|| panic!("`{needle}` not found in handle_impl.rs"));
        let abs = search + rel;
        // Make sure the next character after `fn NAME` is `(` (no
        // generics) or `<` (generics) — i.e. not an unrelated identifier
        // prefix like `fn loop_block_on_helper`.
        let after = src[abs + needle.len()..].chars().next().unwrap_or(' ');
        if after == '(' || after == '<' {
            break abs;
        }
        search = abs + needle.len();
    };
    // Find the opening brace of the body.
    let body_start_rel = src[start..]
        .find('{')
        .unwrap_or_else(|| panic!("opening brace of `fn {fn_name}` not found"));
    let body_start = start + body_start_rel;
    // Walk braces to find the matching close.
    let bytes = src.as_bytes();
    let mut depth: i32 = 0;
    let mut i = body_start;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &src[start..=i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("did not find matching `}}` for `fn {fn_name}` body");
}
