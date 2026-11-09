// Feature: spec/features/ast-extractor-edge-quality.feature
use super::*;

#[test]
fn test_go_method_receiver_calls_produce_edges() {
    // @step Given a Go file "command.go" with a method "func (c *Command) Find(args []string)" that calls "stripFlags(args, c)"
    let go_source = r#"package cobra

type Command struct {
    Name string
}

func stripFlags(args []string, c *Command) []string {
    return args
}

func (c *Command) Find(args []string) {
    result := stripFlags(args, c)
    _ = result
}
"#;

    // @step And "stripFlags" is a package-level function in "command.go"
    // @step When I run the Go AST extractor
    let entities = index_one("command.go", go_source);

    // @step Then a Calls edge should exist from "Find" to "stripFlags"
    let calls = named_edges(&entities, "Calls");
    assert!(
        calls
            .iter()
            .any(|(from, to)| from.ends_with("::Find") && to == "command.go::stripFlags"),
        "Method receiver body should produce Calls edge from Find to stripFlags. All Calls: {calls:?}"
    );

    // @step And "stripFlags" should not appear in dead code uncalled functions
    // (verified by the presence of the Calls edge)
}

#[test]
fn test_go_type_references_produce_typeref_edges() {
    // @step Given a Go file "command.go" with a function "func Find(cmd *Command) error"
    let go_source = r#"package cobra

type Command struct {
    Name string
}

func Find(cmd *Command) error {
    return nil
}
"#;

    // @step And "Command" is a struct declared in the same file
    // @step When I run the Go AST extractor
    let entities = index_one("command.go", go_source);

    // @step Then a TypeRef edge should exist from "Find" to "Command"
    assert!(
        has_named_edge(
            &entities,
            "TypeRef",
            "command.go::Find",
            "command.go::Command"
        ),
        "Should have TypeRef edge from Find to Command. All TypeRef: {:?}",
        named_edges(&entities, "TypeRef")
    );

    // @step And "Command" should not appear in dead code unreferenced types
    // (verified by the presence of the TypeRef edge)
}

#[test]
fn test_python_type_annotations_produce_typeref_edges() {
    // @step Given a Python file "core.py" with a function "def process(ctx: Context) -> None"
    let py_source = r#"class Context:
    pass

def process(ctx: Context) -> None:
    return
"#;

    // @step And "Context" is a class declared in the same file
    // @step When I run the Python AST extractor
    let entities = index_one("core.py", py_source);

    // @step Then a TypeRef edge should exist from "process" to "Context"
    assert!(
        has_named_edge(&entities, "TypeRef", "core.py::process", "core.py::Context"),
        "Should have TypeRef edge from process to Context. All TypeRef: {:?}",
        named_edges(&entities, "TypeRef")
    );
}

#[test]
fn test_go_import_map_populated_for_cross_file_calls() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a Go project with files "main.go" and "utils/helpers.go" in separate packages
    let main_source = r#"package main

import "./utils"

func main() {
    utils.Helper()
}
"#;
    write_test_file(project_dir, "main.go", main_source);

    // @step And "main.go" imports the local "./utils" package
    let util_source = r#"package utils

func Helper() {
    return
}
"#;
    write_test_file(project_dir, "utils/helpers.go", util_source);

    // @step When I run the Go AST extractor with the known_files set
    let entities = index(project_dir).entities;

    // @step Then an Imports edge should exist from "main.go" to the utils package file
    assert!(
        has_named_edge(&entities, "Imports", "main.go", "utils/helpers.go"),
        "Should have Imports edge for local ./utils import. All Imports: {:?}",
        named_edges(&entities, "Imports")
    );

    // @step And the import_map should contain the package name for cross-file call resolution
    // The package name `utils` is bound: utils.Helper() resolves across files.
    assert!(
        has_named_edge(
            &entities,
            "Calls",
            "main.go::main",
            "utils/helpers.go::Helper"
        ),
        "utils.Helper() should resolve through the import. All Calls: {:?}",
        named_edges(&entities, "Calls")
    );
}
