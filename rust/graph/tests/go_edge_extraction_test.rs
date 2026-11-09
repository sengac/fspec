// Feature: spec/features/calls-imports-typeref-edges-go.feature
//
// Integration tests for Go Imports and Calls edge extraction.
// Every scenario indexes a temp project with the real pipeline (walk, fact
// extraction, global resolve pass); no edge is hand-written.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::ast_pipeline::resolve::{Reason, RefKindCount};
use codelet_graph::ast_pipeline::{index_tree, Indexed};
use codelet_graph::graph_entities::GraphEntity;
use codelet_test_helpers::{TestError, TestResult};

pub mod code_graph_fixture;
use code_graph_fixture::{callees_of, edges, project, slug_of};

/// Index a temp project holding `files` with the real pipeline; keep the
/// returned directory alive.
fn index_project(files: &[(&str, &str)]) -> Result<(tempfile::TempDir, Indexed), TestError> {
    let (directory, root) = project(files)?;
    let indexed = index_tree(&root, false, 1);
    Ok((directory, indexed))
}

/// The files `from` has Imports edges to, sorted.
fn imports_of(all: &[GraphEntity], from: &str) -> Vec<String> {
    let mut found: Vec<String> = edges(all, "Imports")
        .into_iter()
        .filter(|(f, _, _)| f == from)
        .map(|(_, to, _)| to)
        .collect();
    found.sort();
    found
}

/// The count of unresolved imports with `reason`.
fn unresolved_imports(indexed: &Indexed, reason: Reason) -> u64 {
    indexed
        .unresolved
        .get(&RefKindCount::Imports)
        .and_then(|reasons| reasons.get(&reason))
        .copied()
        .unwrap_or(0)
}

// ============================================================================
// Scenario: Extract Imports edges from Go import statements with external filtering
// ============================================================================
#[test]
fn test_go_extract_imports_with_external_filtering() -> TestResult {
    // @step Given a Go file with `import "github.com/spf13/cobra"` and `import "./internal/util"`
    let go_source = r#"package cmd

import (
	"fmt"
	"github.com/spf13/cobra"
	"./internal/util"
)

func Execute() {
	fmt.Println("hello")
	util.Helper()
	_ = cobra.Command{}
}
"#;
    // A relative import path is relative to the importing file's
    // directory: `./internal/util` from cmd/ is the package in
    // cmd/internal/util.
    // @step When the Go extractor processes the source file
    let (_directory, indexed) = index_project(&[
        ("cmd/root.go", go_source),
        (
            "cmd/internal/util/helpers.go",
            "package util\n\nfunc Helper() {}\n",
        ),
    ])?;
    let all = &indexed.entities;

    // @step Then an Imports edge should be emitted for the local `./internal/util` import
    assert_eq!(
        imports_of(all, "cmd/root.go"),
        ["cmd/internal/util/helpers.go"],
        "Imports: {:?}",
        edges(all, "Imports")
    );

    // @step And the external `github.com/spf13/cobra` import should NOT produce an edge
    // `fmt` and `github.com/spf13/cobra` are outside the root.
    assert_eq!(
        unresolved_imports(&indexed, Reason::External),
        2,
        "{:?}",
        indexed.unresolved
    );
    Ok(())
}

// ============================================================================
// Scenario: Extract Calls edges from Go function calls
// ============================================================================
#[test]
fn test_go_extract_calls_from_function_calls() -> TestResult {
    let go_source = r#"package cmd

func Execute() {
	initConfig()
}

func initConfig() {
	return
}
"#;

    // @step Given a Go file with function `Execute()` that calls `initConfig()`
    // @step And `initConfig` is defined in the same file
    // @step When the Go extractor processes the source file
    let (_directory, indexed) = index_project(&[("cmd/root.go", go_source)])?;
    let all = &indexed.entities;

    // @step Then a Calls edge should be emitted from `Execute` to `initConfig`
    let execute = slug_of(all, "Function", "cmd/root.go::Execute")?;
    assert_eq!(
        callees_of(all, &execute),
        ["cmd/root.go::initConfig"],
        "Calls: {:?}",
        edges(all, "Calls")
    );
    Ok(())
}
