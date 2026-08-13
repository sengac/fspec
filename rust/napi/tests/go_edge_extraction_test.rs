// Feature: spec/features/calls-imports-typeref-edges-go.feature
//
// Integration tests for Go Imports and Calls edge extraction.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use codelet_napi::graph::ast_pipeline::ast_go_extractor::extract_go;

mod graph_test_helpers;
use graph_test_helpers::{build_known_files, find_edges, write_test_file};

// ============================================================================
// Scenario: Extract Imports edges from Go import statements with external filtering
// ============================================================================
#[test]
fn test_go_extract_imports_with_external_filtering() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a Go file with `import "github.com/spf13/cobra"` and `import "./internal/util"`
    let go_source = r#"package cmd

import (
	"fmt"
	"github.com/spf13/cobra"
	"./internal/util"
)

func Execute() {
	fmt.Println("hello")
}
"#;
    write_test_file(project_dir, "cmd/root.go", go_source);
    write_test_file(
        project_dir,
        "internal/util/helpers.go",
        "package util\nfunc Helper() {}\n",
    );

    let known_files = build_known_files(project_dir);

    // @step When the Go extractor processes the source file
    let entities =
        extract_go(go_source, "cmd/root.go", &known_files).expect("Go extraction should succeed");

    // @step Then an Imports edge should be emitted for the local `./internal/util` import
    let local_imports = find_edges(&entities, "Imports", Some("cmd-root-go"), Some("internal"));
    assert!(
        !local_imports.is_empty(),
        "Should have Imports edge for local ./internal/util import. All Imports: {:?}",
        find_edges(&entities, "Imports", None, None)
    );

    // @step And the external `github.com/spf13/cobra` import should NOT produce an edge
    let cobra_imports = find_edges(&entities, "Imports", None, Some("cobra"));
    assert!(
        cobra_imports.is_empty(),
        "External cobra import should NOT produce an edge"
    );
}

// ============================================================================
// Scenario: Extract Calls edges from Go function calls
// ============================================================================
#[test]
fn test_go_extract_calls_from_function_calls() {
    let go_source = r#"package cmd

func Execute() {
	initConfig()
}

func initConfig() {
	return
}
"#;
    let known_files = HashSet::new();

    // @step Given a Go file with function `Execute()` that calls `initConfig()`
    // @step And `initConfig` is defined in the same file
    // @step When the Go extractor processes the source file
    let entities =
        extract_go(go_source, "cmd/root.go", &known_files).expect("Go extraction should succeed");

    // @step Then a Calls edge should be emitted from `Execute` to `initConfig`
    let calls = find_edges(&entities, "Calls", Some("Execute"), Some("initConfig"));
    assert!(
        !calls.is_empty(),
        "Should have Calls edge from Execute to initConfig. All Calls: {:?}",
        find_edges(&entities, "Calls", None, None)
    );
}
