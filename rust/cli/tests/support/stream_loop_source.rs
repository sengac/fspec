//! BUG-302: the interactive stream loop is split across
//! `src/interactive/stream_loop.rs` and `src/interactive/stream_loop/*.rs`.
//! Source-shape guards read the whole module tree: the facade first, then the
//! module files sorted by name, each followed by a newline.

use std::path::Path;

#[allow(dead_code, clippy::expect_used)] // a missing source file is a test setup failure
pub fn stream_loop_source() -> String {
    let interactive = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/interactive");
    let mut source = std::fs::read_to_string(interactive.join("stream_loop.rs"))
        .expect("stream_loop.rs must be readable");
    source.push('\n');
    let mut modules: Vec<_> = std::fs::read_dir(interactive.join("stream_loop"))
        .expect("stream_loop/ must be readable")
        .map(|entry| entry.expect("stream_loop entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .collect();
    modules.sort();
    for module in modules {
        source.push_str(&std::fs::read_to_string(&module).expect("stream_loop module readable"));
        source.push('\n');
    }
    source
}

/// BUG-306: one module of the split stream loop, for guards whose ordering
/// claims hold within the module that owns the behaviour.
#[allow(dead_code, clippy::expect_used)] // a missing source file is a test setup failure
pub fn stream_loop_module(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/interactive/stream_loop")
        .join(format!("{name}.rs"));
    std::fs::read_to_string(&path).expect("stream_loop module readable")
}
