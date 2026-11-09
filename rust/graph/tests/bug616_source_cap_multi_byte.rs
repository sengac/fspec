//! BUG-616: capping a definition's stored source never cuts a multi-byte
//! character, so the file keeps every entity and an unchanged file is not
//! re-extracted by an incremental index.
//!
//! Feature: spec/features/code-graph-source-cap-on-multi-byte-text.feature
//!
//! Every graph lives in a temp project. The fixtures are built so the byte
//! cap (4096) lands inside a character, and each fixture checks that itself.

use std::path::{Path, PathBuf};

use codelet_graph::ast_index::dispatch_ast_index;
use codelet_graph::registry;
use codelet_test_helpers::{setup_temp_directory, TestError, TestResult};
use serde_json::Value;

const BYTE_CAP: usize = 4096;

/// `fn wide_text`: fewer than 100 lines, longer than the byte cap, with a
/// 2-byte character across byte 4096.
fn wide_text_source() -> Result<String, TestError> {
    for pad in 0..60 {
        let mut text = format!("fn wide_text() -> usize {{{}\n", " ".repeat(pad));
        let mut line = 0;
        while text.len() <= BYTE_CAP + 200 {
            text.push_str(&format!("    let _a{line:02} = \"{}\";\n", "é".repeat(20)));
            line += 1;
        }
        text.push_str("    0\n}");
        if !text.is_char_boundary(BYTE_CAP) && text.lines().count() < 100 {
            return Ok(text);
        }
    }
    Err("no padding puts a 2-byte character across byte 4096".into())
}

/// A one-line `fn` of 5000 bytes, mostly 3-byte characters, whose last
/// character boundary at or before byte 4096 is byte 4095.
fn one_line_source() -> Result<String, TestError> {
    for pad in 0..3 {
        let mut text = format!("fn one_line_wide() {{{} let _s = \"", " ".repeat(pad));
        while text.len() < 5000 - 4 {
            text.push('→');
        }
        text.push_str("\"; }");
        if text.is_char_boundary(BYTE_CAP - 1) && !text.is_char_boundary(BYTE_CAP) {
            return Ok(text);
        }
    }
    Err("no padding puts byte 4095 on a character boundary".into())
}

fn project(files: &[(&str, &str)]) -> Result<(tempfile::TempDir, PathBuf), TestError> {
    let directory = setup_temp_directory()?;
    let root = directory.path().join("project");
    std::fs::create_dir_all(root.join("src"))?;
    for (path, text) in files {
        std::fs::write(root.join(path), text)?;
    }
    let root = root.canonicalize()?;
    Ok((directory, root))
}

fn parsed(output: &str) -> Result<Value, TestError> {
    Ok(serde_json::from_str(output)?)
}

async fn index(root: &Path, incremental: bool) -> Result<Value, TestError> {
    let output = parsed(&dispatch_ast_index(root, false, incremental).await)?;
    if let Some(error) = output.get("error") {
        return Err(format!("ast_index failed: {error}").into());
    }
    Ok(output)
}

async fn functions(root: &Path) -> Result<Vec<Value>, TestError> {
    let graph = registry::read_code_graph(root).await?;
    let output = codelet_graph::ast_dispatch::dispatch_ast_search(
        &graph,
        "",
        Some("Function"),
        Some(50),
        None,
        None,
        None,
        None,
    )
    .await;
    let value = parsed(&output)?;
    Ok(value["results"]
        .as_array()
        .ok_or_else(|| format!("no results in {output}"))?
        .clone())
}

fn named<'a>(rows: &'a [Value], name: &str) -> Result<&'a Value, TestError> {
    rows.iter()
        .find(|row| row["name"].as_str() == Some(name))
        .ok_or_else(|| format!("no function {name} in {rows:?}").into())
}

fn source_of(row: &Value) -> Result<&str, TestError> {
    row["source"]
        .as_str()
        .ok_or_else(|| format!("no source in {row}").into())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_definition_cut_inside_a_multi_byte_character_is_indexed_with_the_rest_of_its_file(
) -> TestResult {
    // @step Given a project whose Rust file holds fn wide_text, whose first 100 lines are longer than 4096 bytes with a 2-byte character across byte 4096, and fn after_wide
    let wide = wide_text_source()?;
    let file = format!("{wide}\n\nfn after_wide() -> usize {{\n    1\n}}\n");
    let (_directory, root) = project(&[("src/wide.rs", &file)])?;

    // @step When ast_index runs for the project
    index(&root, false).await?;

    // @step And ast_search lists the project's functions
    let rows = functions(&root).await?;

    // @step Then the results hold wide_text and after_wide
    let wide_row = named(&rows, "wide_text")?;
    named(&rows, "after_wide")?;

    // @step And wide_text's source is marked truncated, is at most 4096 bytes, ends at a complete line and is a prefix of its text
    let source = source_of(wide_row)?;
    assert_eq!(wide_row["truncated"], Value::Bool(true), "{wide_row}");
    assert!(source.len() <= BYTE_CAP, "{} bytes", source.len());
    assert!(
        wide.starts_with(source),
        "not a prefix of the text: {source}"
    );
    assert_eq!(
        wide.as_bytes().get(source.len()),
        Some(&b'\n'),
        "does not end at a complete line"
    );
    let next_line_end = wide[source.len() + 1..]
        .find('\n')
        .map(|offset| source.len() + 1 + offset)
        .ok_or("no line after the cut")?;
    assert!(
        next_line_end > BYTE_CAP,
        "the next complete line ({next_line_end} bytes) also fits the cap"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_one_line_definition_longer_than_the_byte_cap_keeps_whole_characters() -> TestResult {
    // @step Given a project whose Rust file holds a one-line fn whose 5000 bytes are mostly 3-byte characters
    let line = one_line_source()?;
    let (_directory, root) = project(&[("src/line.rs", &format!("{line}\n"))])?;

    // @step When ast_index runs for the project
    index(&root, false).await?;

    // @step And ast_search lists the project's functions
    let rows = functions(&root).await?;

    // @step Then the function's source is its first 4095 bytes, ending on a character boundary, and is marked truncated
    let row = named(&rows, "one_line_wide")?;
    assert_eq!(source_of(row)?, &line[..BYTE_CAP - 1]);
    assert_eq!(row["truncated"], Value::Bool(true), "{row}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unchanged_file_is_not_re_extracted_by_an_incremental_index() -> TestResult {
    // @step Given a project whose Rust file holds fn wide_text with a 2-byte character across byte 4096 of its first 100 lines, and a second file
    let wide = wide_text_source()?;
    let (_directory, root) = project(&[
        ("src/wide.rs", &format!("{wide}\n")),
        ("src/other.rs", "fn other_fn() -> usize {\n    2\n}\n"),
    ])?;

    // @step And ast_index has run for the project
    index(&root, false).await?;

    // @step When ast_index runs incrementally with no file changed
    let output = index(&root, true).await?;

    // @step Then it re-extracts 0 files
    assert_eq!(output["files_parsed"], Value::from(0), "{output}");
    Ok(())
}
