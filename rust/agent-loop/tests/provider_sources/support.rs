//! BUG-344: source readers shared by the RPC-082 / RPC-083 wiring tests.
//!
//! The provider turn code lives in `src/dispatch.rs` (the `run_with_provider!`
//! macro), `src/agent_loop/provider_openai.rs` (`run_openai_turn`) and
//! `src/agent_loop/provider_custom.rs` (`run_custom_turn`).

use std::path::PathBuf;

/// A never-called closure per provider: it compiles only when the provider
/// exposes `create_rig_agent(session_id, preamble: Option<&str>, thinking)`.
macro_rules! preamble_signature_witness {
    ($($provider:ty),*) => {$(
        let _ = |provider: &$provider,
                 session_id: uuid::Uuid,
                 preamble: Option<&str>,
                 thinking: Option<serde_json::Value>| {
            provider.create_rig_agent(session_id, preamble, thinking)
        };
    )*};
}

/// The contents of a file under `rust/agent-loop/src`.
pub fn read_source(file: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(file);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

/// The `{ ... }` block that starts at the first `{` at or after `start`.
fn braced_block(src: &str, start: usize) -> &str {
    let open = start
        + src[start..]
            .find('{')
            .expect("a `{` must follow the marker");
    let mut depth = 0i32;
    for (offset, byte) in src.as_bytes()[open..].iter().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &src[open..=open + offset];
                }
            }
            _ => {}
        }
    }
    panic!("unbalanced braces after byte {start}");
}

/// The body of the `macro_rules!` macro named `name` (every arm).
pub fn macro_body<'a>(src: &'a str, name: &str) -> &'a str {
    let marker = format!("macro_rules! {name}");
    let start = src
        .find(&marker)
        .unwrap_or_else(|| panic!("`{marker}` must exist"));
    braced_block(src, start)
}

/// The body of the function named `name`.
pub fn function_body<'a>(src: &'a str, name: &str) -> &'a str {
    let marker = format!("fn {name}(");
    let start = src
        .find(&marker)
        .unwrap_or_else(|| panic!("`fn {name}` must exist"));
    let signature_end = start + src[start..].find(") ->").expect("function return type");
    braced_block(src, signature_end)
}

/// Assert that every needle occurs in `body`, each after the previous one.
pub fn assert_in_order(body: &str, needles: &[&str]) {
    let mut from = 0;
    for needle in needles {
        let at = body[from..]
            .find(needle)
            .unwrap_or_else(|| panic!("`{needle}` must follow the previous call in:\n{body}"));
        from += at + needle.len();
    }
}
