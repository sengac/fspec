//! KGRAPH-073: the code graph belongs to one canonical root, its writes are
//! serialized in the process and fenced across processes by a lock file.
//!
//! Feature: spec/features/the-code-graph-belongs-to-the-session-project-root.feature
//!
//! Every graph lives in a temp project; nothing here reads or writes the
//! repository's graph or the user's home. "Another process" holding the
//! writer lock is played by a second open file description taking the OS
//! lock, which conflicts exactly like a lock held by another process.

use std::path::{Path, PathBuf};

use codelet_graph::ast_index::dispatch_ast_index;
use codelet_graph::registry;
use codelet_test_helpers::{
    deadlock_guard, pending_until, setup_temp_directory, TestError, TestResult,
};
use serde_json::Value;

fn project(functions: &[&str]) -> Result<(tempfile::TempDir, PathBuf), TestError> {
    let directory = setup_temp_directory()?;
    let root = directory.path().join("project");
    std::fs::create_dir_all(root.join("src"))?;
    let source: String = functions
        .iter()
        .map(|name| format!("def {name}():\n    return 1\n\n"))
        .collect();
    std::fs::write(root.join("src/module.py"), source)?;
    let root = root.canonicalize()?;
    Ok((directory, root))
}

fn parsed(output: &str) -> Result<Value, TestError> {
    Ok(serde_json::from_str(output)?)
}

fn function_names(output: &str) -> Result<Vec<String>, TestError> {
    let value = parsed(output)?;
    let mut names: Vec<String> = value["results"]
        .as_array()
        .ok_or_else(|| format!("no results in {output}"))?
        .iter()
        .filter_map(|row| row["name"].as_str().map(str::to_owned))
        .collect();
    names.sort();
    Ok(names)
}

async fn search(root: &Path, query: &str) -> Result<String, TestError> {
    let graph = registry::read_code_graph(root).await?;
    Ok(codelet_graph::ast_dispatch::dispatch_ast_search(
        &graph,
        query,
        Some("Function"),
        Some(50),
        None,
        None,
        None,
        None,
    )
    .await)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_write_fails_while_another_process_holds_the_writer_lock() -> TestResult {
    let (_directory, root) = project(&["held_fn"])?;

    // @step Given another process holds the writer lock of project P's code graph
    let lock_path = registry::code_graph_lock_path(&root);
    let parent = lock_path.parent().ok_or("lock path has no parent")?;
    std::fs::create_dir_all(parent)?;
    let holder = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)?;
    holder.try_lock()?;

    // @step When ast_index runs for project P
    let output = dispatch_ast_index(&root, false, false).await;

    // @step Then it fails with an error naming the lock file
    let error = parsed(&output)?["error"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("expected an error, got {output}"))?;
    assert!(
        error.contains(&lock_path.display().to_string()),
        "error does not name {}: {error}",
        lock_path.display()
    );

    // @step And project P has no code graph
    assert!(!registry::code_graph_path(&root).exists());
    assert!(!registry::code_graph_available(&root));
    holder.unlock()?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_reset_waits_for_a_running_reader() -> TestResult {
    // @step Given project P has been indexed
    let (_directory, root) = project(&["reader_fn"])?;
    let first = dispatch_ast_index(&root, false, false).await;
    assert!(parsed(&first)?.get("error").is_none(), "{first}");

    // @step And a reader holds project P's graph
    let reader = registry::read_code_graph(&root).await?;

    // @step When ast_index runs for project P with reset
    let mut reset = Box::pin(dispatch_ast_index(&root, true, false));

    // @step Then the index stays pending while the reader holds the graph
    pending_until(
        "the reset waits on the graph's write side",
        &mut reset,
        || registry::code_graph_writers_waiting(&root) == 1,
    )
    .await?;

    // @step And the index completes once the reader is done
    drop(reader);
    let output = deadlock_guard("the reset after the reader", reset).await?;
    assert!(parsed(&output)?.get("error").is_none(), "{output}");
    assert_eq!(
        function_names(&search(&root, "reader").await?)?,
        ["reader_fn"]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_index_runs_on_one_root_in_one_process_both_succeed() -> TestResult {
    // @step Given project P holds source files
    let (_directory, root) = project(&["first_fn", "second_fn"])?;

    // @step When two ast_index runs for project P start at the same time
    let (one, two) = deadlock_guard("two index runs", async {
        tokio::join!(
            dispatch_ast_index(&root, false, false),
            dispatch_ast_index(&root, false, false)
        )
    })
    .await?;

    // @step Then both runs succeed
    assert!(parsed(&one)?.get("error").is_none(), "{one}");
    assert!(parsed(&two)?.get("error").is_none(), "{two}");
    assert_eq!(parsed(&one)?["root"], root.display().to_string());

    // @step And project P's graph holds P's functions
    assert_eq!(
        function_names(&search(&root, "_fn").await?)?,
        ["first_fn", "second_fn"]
    );
    Ok(())
}

fn rust_sources(directory: &Path, found: &mut Vec<PathBuf>) -> TestResult {
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            rust_sources(&path, found)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
    Ok(())
}

#[test]
fn graph_code_never_reads_the_process_working_directory() -> TestResult {
    // @step Given the codelet-graph sources and the GraphSearch handler sources
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut sources = Vec::new();
    rust_sources(&manifest.join("src"), &mut sources)?;
    sources.push(manifest.join("../agent-loop/src/graph_search_handler.rs"));
    assert!(sources.len() > 10, "found only {sources:?}");

    // @step When they are scanned for current_dir
    let mut offenders = Vec::new();
    for source in &sources {
        for (number, line) in std::fs::read_to_string(source)?.lines().enumerate() {
            if line.contains("current_dir") {
                offenders.push(format!("{}:{}: {line}", source.display(), number + 1));
            }
        }
    }

    // @step Then no call to current_dir is found
    assert!(offenders.is_empty(), "{offenders:#?}");
    Ok(())
}
