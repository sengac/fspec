//! BUG-478 — `fspec astgrep` finishes on the workspace root.
//!
//! Feature: spec/features/astgrep-finishes-on-the-workspace-root.feature

use ast_grep_language::{LanguageExt, SupportLang};
use codelet_tools::AstGrepTool;
use serde_json::json;
use std::path::Path;
use std::time::{Duration, Instant};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Run AstGrep's search (what `fspec astgrep` calls) and return its output.
async fn search(pattern: &str, path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let output = AstGrepTool::new(uuid::Uuid::nil())
        .execute(json!({"pattern": pattern, "language": "rust", "path": path.to_string_lossy()}))
        .await?;
    if output.is_error {
        return Err(output.content.into());
    }
    Ok(output.content)
}

/// A Rust source file of `functions` small functions followed by `pub fn <marker>() {}`.
fn rust_source(functions: usize, marker: &str) -> String {
    let mut source = String::new();
    for index in 0..functions {
        source.push_str(&format!(
            "pub fn helper_{index}(value: u32) -> u32 {{\n    let doubled = value * 2;\n    \
             if doubled > 10 {{ doubled - 1 }} else {{ doubled + {index} }}\n}}\n\n"
        ));
    }
    source.push_str(&format!("pub fn {marker}() {{}}\n"));
    source
}

#[tokio::test]
async fn a_gitignored_target_directory_is_not_scanned() -> TestResult {
    // @step Given a git repository whose rust/.gitignore lists target/
    let repo = tempfile::tempdir()?;
    let rust = repo.path().join("rust");
    std::fs::create_dir_all(repo.path().join(".git"))?;
    std::fs::create_dir_all(rust.join("src"))?;
    std::fs::create_dir_all(rust.join("target/debug/build"))?;
    std::fs::write(rust.join(".gitignore"), "target/\n")?;

    // @step And rust/src/lib.rs and rust/target/debug/build/gen.rs both define "pub fn marker() {}"
    std::fs::write(rust.join("src/lib.rs"), "pub fn marker() {}\n")?;
    std::fs::write(
        rust.join("target/debug/build/gen.rs"),
        "pub fn marker() {}\n",
    )?;

    // @step When astgrep searches rust/ for "pub fn marker() {}"
    let output = search("pub fn marker() {}", &rust).await?;

    // @step Then it reports rust/src/lib.rs
    assert!(output.contains("src/lib.rs:1:1:"), "{output}");

    // @step And it reports nothing under rust/target
    assert!(!output.contains("target"), "{output}");
    Ok(())
}

/// Parse every `.rs` file in `dir` and visit every node once; returns the time taken.
fn parse_and_walk(dir: &Path) -> Result<Duration, Box<dyn std::error::Error>> {
    let started = Instant::now();
    let mut nodes = 0usize;
    for entry in std::fs::read_dir(dir)? {
        let source = std::fs::read_to_string(entry?.path())?;
        nodes += SupportLang::Rust.ast_grep(&source).root().dfs().count();
    }
    if nodes == 0 {
        return Err("the generated files have no nodes".into());
    }
    Ok(started.elapsed())
}

#[tokio::test]
async fn searching_a_source_tree_costs_about_as_much_as_parsing_it_once() -> TestResult {
    // @step Given a directory of 20 Rust files with 200 functions each
    let dir = tempfile::tempdir()?;
    for file in 0..20 {
        let source = rust_source(200, &format!("marker_{file}"));
        std::fs::write(dir.path().join(format!("file_{file}.rs")), source)?;
    }

    // The best of three runs of each, so a busy machine slows both alike.
    let mut parse = Duration::MAX;
    let mut searching = Duration::MAX;
    let mut output = String::new();
    for _ in 0..3 {
        // @step When the time to parse and walk every file once is measured
        parse = parse.min(parse_and_walk(dir.path())?);

        // @step And astgrep searches the directory for one function
        let started = Instant::now();
        output = search("pub fn marker_7() {}", dir.path()).await?;
        searching = searching.min(started.elapsed());
    }

    // @step Then it finds that function
    assert_eq!(output.lines().count(), 1, "{output}");
    assert!(output.contains("file_7.rs:"), "{output}");

    // @step And the search takes less than four times as long as parsing and walking the files
    assert!(
        searching < parse * 4,
        "search took {searching:?}, parsing and walking the files took {parse:?}"
    );
    Ok(())
}
