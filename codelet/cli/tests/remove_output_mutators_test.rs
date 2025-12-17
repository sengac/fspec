// Feature: spec/features/remove-output-mutators.feature
//
// Tests for CLEAN-005: Remove output mutators (highlighting and diff)
// These tests verify that output mutator modules are completely removed.

use std::fs;

/// Helper to find the workspace root (where the workspace Cargo.toml is)
fn workspace_root() -> std::path::PathBuf {
    // Integration tests run from workspace root, but let's be safe
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap());

    // If we're in cli/, go up one level to workspace root
    if manifest_dir.ends_with("cli") {
        manifest_dir.parent().unwrap().to_path_buf()
    } else {
        manifest_dir
    }
}

/// Scenario: Output mutator source files are removed
///
/// Verifies that highlight.rs and diff.rs modules no longer exist
/// and that lib.rs no longer declares these modules.
#[test]
fn test_output_mutator_source_files_are_removed() {
    let root = workspace_root();

    // @step Given the codelet project contains highlight.rs and diff.rs modules
    // (Precondition: We're testing the codelet project structure)

    // @step When the output mutators are removed
    // (This is verified by checking file absence)

    // @step Then codelet/cli/src/highlight.rs should not exist
    let highlight_path = root.join("cli/src/highlight.rs");
    assert!(
        !highlight_path.exists(),
        "highlight.rs should not exist after removal: {}",
        highlight_path.display()
    );

    // @step Then codelet/cli/src/diff.rs should not exist
    let diff_path = root.join("cli/src/diff.rs");
    assert!(
        !diff_path.exists(),
        "diff.rs should not exist after removal: {}",
        diff_path.display()
    );

    // @step Then codelet/cli/src/lib.rs should not contain 'pub mod diff' or 'pub mod highlight'
    let lib_path = root.join("cli/src/lib.rs");
    let lib_content = fs::read_to_string(&lib_path).expect("Should be able to read lib.rs");
    assert!(
        !lib_content.contains("pub mod diff"),
        "lib.rs should not contain 'pub mod diff'"
    );
    assert!(
        !lib_content.contains("pub mod highlight"),
        "lib.rs should not contain 'pub mod highlight'"
    );
}

/// Scenario: Interactive mode displays tool results without colored formatting
///
/// Verifies that the interactive module no longer imports or uses diff/highlight functions.
#[test]
fn test_interactive_mode_without_colored_formatting() {
    let root = workspace_root();

    // @step Given interactive mode is running
    // (Verified by checking interactive module code)

    // @step When the agent executes an Edit or Write tool
    // (Verified by checking that the rendering code is removed)

    // @step Then the tool result should display without ANSI color codes
    // @step Then the tool result should display without diff formatting prefixes
    let interactive_dir = root.join("cli/src/interactive");

    // Read all files in the interactive module directory
    let mut interactive_content = String::new();
    for entry in
        fs::read_dir(&interactive_dir).expect("Should be able to read interactive directory")
    {
        let entry = entry.expect("Should be able to read directory entry");
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "rs") {
            interactive_content.push_str(
                &fs::read_to_string(&path)
                    .unwrap_or_else(|_| panic!("Should be able to read {}", path.display())),
            );
        }
    }

    // Verify imports are removed
    assert!(
        !interactive_content.contains("use crate::diff::"),
        "interactive module should not import from crate::diff"
    );
    assert!(
        !interactive_content.contains("use crate::highlight::"),
        "interactive module should not import from crate::highlight"
    );

    // Verify function calls are removed
    assert!(
        !interactive_content.contains("render_diff_line"),
        "interactive module should not call render_diff_line"
    );
    assert!(
        !interactive_content.contains("highlight_bash_command"),
        "interactive module should not call highlight_bash_command"
    );
}

/// Scenario: Edit tool returns simplified plain text output
///
/// Verifies that edit.rs returns simplified output without diff formatting.
#[test]
fn test_edit_tool_simplified_output() {
    let root = workspace_root();

    // @step Given a file exists with content to edit
    // (Verified by checking edit.rs code structure)

    // @step When the Edit tool replaces 'old_string' with 'new_string'
    // (Verified by checking the output format in code)

    // @step Then the output should be 'Edited file: path (replaced old_string with new_string)'
    // @step Then the output should not contain diff prefixes like '+' or '-'
    let edit_path = root.join("tools/src/edit.rs");
    let edit_content = fs::read_to_string(&edit_path).expect("Should be able to read edit.rs");

    // Verify diff format is not used - check for the specific pattern
    assert!(
        !edit_content.contains(r#"format!("File: {file_path}\n- {old_string}\n+ {new_string}")"#),
        "edit.rs should not use diff format output"
    );

    // Verify simplified format is used (will pass after implementation)
    assert!(
        edit_content.contains("Edited file:") || edit_content.contains("edited"),
        "edit.rs should use simplified output format"
    );
}

/// Scenario: Write tool returns simplified plain text output
///
/// Verifies that write.rs returns simplified output without diff formatting.
#[test]
fn test_write_tool_simplified_output() {
    let root = workspace_root();

    // @step Given a file path is specified for writing
    // (Verified by checking write.rs code structure)

    // @step When the Write tool creates a file with 10 lines of content
    // (Verified by checking the output format in code)

    // @step Then the output should be 'Wrote file: path (10 lines)'
    // @step Then the output should not list each line with '+' prefix
    let write_path = root.join("tools/src/write.rs");
    let write_content = fs::read_to_string(&write_path).expect("Should be able to read write.rs");

    // Verify diff format is not used - lines with '+' prefix
    assert!(
        !write_content.contains(r#"format!("+ {line}")"#),
        "write.rs should not format lines with '+' prefix"
    );

    // Verify simplified format is used (will pass after implementation)
    assert!(
        write_content.contains("Wrote file:") || write_content.contains("wrote"),
        "write.rs should use simplified output format"
    );
}

/// Scenario: Tree-sitter dependencies are removed from Cargo.toml
///
/// Verifies that tree-sitter-highlight and tree-sitter-bash are removed
/// from both workspace and cli Cargo.toml files.
#[test]
fn test_tree_sitter_dependencies_removed() {
    let root = workspace_root();

    // @step Given the project has tree-sitter-highlight and tree-sitter-bash dependencies
    // (Precondition: checking Cargo.toml files)

    // @step When the output mutators are removed
    // (Verified by checking Cargo.toml contents)

    // @step Then codelet/Cargo.toml should not contain 'tree-sitter-highlight'
    let workspace_cargo_path = root.join("Cargo.toml");
    let workspace_cargo = fs::read_to_string(&workspace_cargo_path)
        .expect("Should be able to read workspace Cargo.toml");
    assert!(
        !workspace_cargo.contains("tree-sitter-highlight"),
        "Workspace Cargo.toml should not contain tree-sitter-highlight"
    );

    // @step Then codelet/Cargo.toml should not contain 'tree-sitter-bash'
    assert!(
        !workspace_cargo.contains("tree-sitter-bash"),
        "Workspace Cargo.toml should not contain tree-sitter-bash"
    );

    // @step Then codelet/cli/Cargo.toml should not contain 'tree-sitter-highlight'
    let cli_cargo_path = root.join("cli/Cargo.toml");
    let cli_cargo =
        fs::read_to_string(&cli_cargo_path).expect("Should be able to read cli Cargo.toml");
    assert!(
        !cli_cargo.contains("tree-sitter-highlight"),
        "CLI Cargo.toml should not contain tree-sitter-highlight"
    );

    // @step Then codelet/cli/Cargo.toml should not contain 'tree-sitter-bash'
    assert!(
        !cli_cargo.contains("tree-sitter-bash"),
        "CLI Cargo.toml should not contain tree-sitter-bash"
    );
}

/// Scenario: Project builds successfully without output mutator modules
///
/// This test verifies related test and example files are removed.
/// The actual build verification happens via cargo build.
#[test]
fn test_related_test_files_removed() {
    let root = workspace_root();

    // @step Given all output mutator files and dependencies have been removed
    // (Verified by checking test file absence)

    // @step When I run 'cargo build' in the codelet directory
    // (Build verification is done separately via cargo build)

    // @step Then the build should complete successfully with exit code 0
    // @step Then there should be no compilation errors related to missing modules

    // Verify related test files are removed (these are in the orphaned tests/ directory)
    let diff_integration_test = root.join("tests/diff_rendering_integration_test.rs");
    assert!(
        !diff_integration_test.exists(),
        "diff_rendering_integration_test.rs should be removed"
    );

    let diff_e2e_test = root.join("tests/diff_rendering_e2e_test.rs");
    assert!(
        !diff_e2e_test.exists(),
        "diff_rendering_e2e_test.rs should be removed"
    );

    let text_styling_test = root.join("tests/text_styling_test.rs");
    assert!(
        !text_styling_test.exists(),
        "text_styling_test.rs should be removed"
    );

    // Verify example files are removed
    let demo_diff = root.join("examples/demo_diff_rendering.rs");
    assert!(
        !demo_diff.exists(),
        "demo_diff_rendering.rs should be removed"
    );

    let test_styling = root.join("examples/test_text_styling.rs");
    assert!(
        !test_styling.exists(),
        "test_text_styling.rs should be removed"
    );
}
