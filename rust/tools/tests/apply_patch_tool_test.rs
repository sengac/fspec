#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/codex-apply-patch.feature
//!
//! Integration tests that call ApplyPatchTool::call() with real temp files
//! to verify end-to-end file creation, update, and deletion behavior.

use codelet_tools::apply_patch::{ApplyPatchArgs, ApplyPatchTool};
use rig::tool::Tool;
use std::fs;
use tempfile::TempDir;
use uuid::Uuid;

// ==========================================
// Scenario: Add a new file via apply_patch
// ==========================================

#[tokio::test]
async fn test_add_file_creates_file_on_disk() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("new_file.rs");
    let path_str = file_path.to_string_lossy();

    let tool = ApplyPatchTool::new(Uuid::nil());

    // @step Given a Codex session with the apply_patch tool registered
    // @step When the agent calls apply_patch with an Add File block for "/tmp/test/new_file.rs"
    let patch = format!(
        "*** Begin Patch\n\
         *** Add File: {path_str}\n\
         +fn main() {{\n\
         +    println!(\"hello\");\n\
         +}}\n\
         *** End Patch"
    );

    let result = tool.call(ApplyPatchArgs { patch }).await.unwrap();

    // @step Then the file "/tmp/test/new_file.rs" is created with the specified content
    assert!(file_path.exists(), "File should be created on disk");
    let content = fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("fn main() {"));
    assert!(content.contains("    println!(\"hello\");"));
    assert!(content.contains("}"));

    // @step And the tool returns a success message listing the created file
    assert!(
        result.contains("Created"),
        "Result should contain 'Created', got: {result}"
    );
    assert!(
        result.contains(&*path_str),
        "Result should contain file path, got: {result}"
    );
}

#[tokio::test]
async fn test_add_file_creates_parent_directories() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("nested").join("deep").join("file.rs");
    let path_str = file_path.to_string_lossy();

    let tool = ApplyPatchTool::new(Uuid::nil());
    let patch = format!(
        "*** Begin Patch\n\
         *** Add File: {path_str}\n\
         +// nested file\n\
         *** End Patch"
    );

    let result = tool.call(ApplyPatchArgs { patch }).await.unwrap();

    assert!(file_path.exists(), "Nested file should be created");
    assert!(result.contains("Created"));
}

// ==========================================
// Scenario: Update an existing file via apply_patch
// ==========================================

#[tokio::test]
async fn test_update_file_replaces_matching_lines() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("existing.rs");
    let path_str = file_path.to_string_lossy();

    // @step Given a Codex session with the apply_patch tool registered
    // @step And a file "/tmp/test/existing.rs" exists with known content
    fs::write(&file_path, "fn main() {\n    println!(\"old\");\n}\n").unwrap();

    let tool = ApplyPatchTool::new(Uuid::nil());

    // @step When the agent calls apply_patch with an Update File block containing context lines, removals, and additions
    let patch = format!(
        "*** Begin Patch\n\
         *** Update File: {path_str}\n\
         @@ fn main() {{\n\
         -    println!(\"old\");\n\
         +    println!(\"new\");\n\
         *** End Patch"
    );

    let result = tool.call(ApplyPatchArgs { patch }).await.unwrap();

    // @step Then the matching lines in "/tmp/test/existing.rs" are replaced
    let content = fs::read_to_string(&file_path).unwrap();
    assert!(
        content.contains("println!(\"new\")"),
        "File should contain the new line, got: {content}"
    );
    assert!(
        !content.contains("println!(\"old\")"),
        "File should not contain the old line, got: {content}"
    );

    // @step And unchanged context lines remain intact
    assert!(
        content.contains("fn main() {"),
        "Context line should be preserved, got: {content}"
    );
    assert!(content.contains("}"), "Closing brace should be preserved");

    // @step And the tool returns a success message listing the updated file
    assert!(
        result.contains("Updated"),
        "Result should contain 'Updated', got: {result}"
    );
    assert!(
        result.contains(&*path_str),
        "Result should contain file path, got: {result}"
    );
}

// ==========================================
// Scenario: Delete a file via apply_patch
// ==========================================

#[tokio::test]
async fn test_delete_file_removes_from_disk() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("to_delete.rs");
    let path_str = file_path.to_string_lossy();

    // @step Given a Codex session with the apply_patch tool registered
    // @step And a file "/tmp/test/to_delete.rs" exists
    fs::write(&file_path, "// will be deleted\n").unwrap();
    assert!(file_path.exists());

    let tool = ApplyPatchTool::new(Uuid::nil());

    // @step When the agent calls apply_patch with a Delete File block for "/tmp/test/to_delete.rs"
    let patch = format!(
        "*** Begin Patch\n\
         *** Delete File: {path_str}\n\
         *** End Patch"
    );

    let result = tool.call(ApplyPatchArgs { patch }).await.unwrap();

    // @step Then the file "/tmp/test/to_delete.rs" no longer exists on disk
    assert!(!file_path.exists(), "File should be deleted from disk");

    // @step And the tool returns a success message listing the deleted file
    assert!(
        result.contains("Deleted"),
        "Result should contain 'Deleted', got: {result}"
    );
    assert!(
        result.contains(&*path_str),
        "Result should contain file path, got: {result}"
    );
}

// ==========================================
// Scenario: Multi-file patch with mixed operations
// ==========================================

#[tokio::test]
async fn test_multi_file_patch_mixed_operations() {
    let temp_dir = TempDir::new().unwrap();
    let new_file = temp_dir.path().join("new.rs");
    let update_file = temp_dir.path().join("update_me.rs");
    let delete_file = temp_dir.path().join("delete_me.rs");

    let new_str = new_file.to_string_lossy();
    let update_str = update_file.to_string_lossy();
    let delete_str = delete_file.to_string_lossy();

    // @step Given a Codex session with the apply_patch tool registered
    // @step And a file "/tmp/test/update_me.rs" exists with known content
    fs::write(&update_file, "fn foo() {\n    old();\n}\n").unwrap();
    // @step And a file "/tmp/test/delete_me.rs" exists
    fs::write(&delete_file, "// to be deleted\n").unwrap();

    let tool = ApplyPatchTool::new(Uuid::nil());

    // @step When the agent calls apply_patch with Add, Update, and Delete blocks in a single patch
    let patch = format!(
        "*** Begin Patch\n\
         *** Add File: {new_str}\n\
         +// new file\n\
         *** Update File: {update_str}\n\
         @@ fn foo() {{\n\
         -    old();\n\
         +    new();\n\
         *** Delete File: {delete_str}\n\
         *** End Patch"
    );

    let result = tool.call(ApplyPatchArgs { patch }).await.unwrap();

    // @step Then the new file is created
    assert!(new_file.exists(), "New file should be created");
    let new_content = fs::read_to_string(&new_file).unwrap();
    assert!(new_content.contains("// new file"));

    // @step And the existing file is updated
    let updated_content = fs::read_to_string(&update_file).unwrap();
    assert!(
        updated_content.contains("new()"),
        "Updated file should contain new content, got: {updated_content}"
    );
    assert!(
        !updated_content.contains("old()"),
        "Updated file should not contain old content"
    );

    // @step And the deleted file is removed
    assert!(!delete_file.exists(), "Deleted file should be removed");

    // @step And the tool returns a success message listing all affected files
    assert!(result.contains("Created"), "Should mention created file");
    assert!(result.contains("Updated"), "Should mention updated file");
    assert!(result.contains("Deleted"), "Should mention deleted file");
    // Verify all three paths are in the output
    assert_eq!(
        result.lines().count(),
        3,
        "Should have exactly 3 result lines, got: {result}"
    );
}

// ==========================================
// Scenario: Malformed patch missing Begin Patch marker
// ==========================================

#[tokio::test]
async fn test_malformed_patch_returns_error() {
    let tool = ApplyPatchTool::new(Uuid::nil());

    // @step Given a Codex session with the apply_patch tool registered
    // @step When the agent calls apply_patch with text that does not start with "*** Begin Patch"
    let patch = "*** Add File: /tmp/foo.rs\n+hello\n*** End Patch".to_string();

    let result = tool.call(ApplyPatchArgs { patch }).await;

    // @step Then the tool returns an error mentioning the missing patch marker
    assert!(result.is_err(), "Should return an error");
    let err = format!("{:?}", result.unwrap_err());
    assert!(
        err.contains("Begin Patch"),
        "Error should mention 'Begin Patch', got: {err}"
    );
}

#[tokio::test]
async fn test_empty_patch_returns_error() {
    let tool = ApplyPatchTool::new(Uuid::nil());

    let patch = "*** Begin Patch\n*** End Patch".to_string();
    let result = tool.call(ApplyPatchArgs { patch }).await;

    assert!(result.is_err(), "Empty patch should return an error");
    let err = format!("{:?}", result.unwrap_err());
    assert!(
        err.contains("no file operations"),
        "Error should mention no operations, got: {err}"
    );
}

// ==========================================
// Scenario: Update File with non-matching context lines
// ==========================================

#[tokio::test]
async fn test_update_context_mismatch_returns_error_and_preserves_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("mismatch.rs");
    let path_str = file_path.to_string_lossy();

    // @step Given a Codex session with the apply_patch tool registered
    // @step And a file "/tmp/test/mismatch.rs" exists with known content
    let original_content = "fn main() {\n    println!(\"hello\");\n}\n";
    fs::write(&file_path, original_content).unwrap();

    let tool = ApplyPatchTool::new(Uuid::nil());

    // @step When the agent calls apply_patch with an Update File block whose context lines do not match the file
    let patch = format!(
        "*** Begin Patch\n\
         *** Update File: {path_str}\n\
         @@ fn nonexistent() {{\n\
         -    old();\n\
         +    new();\n\
         *** End Patch"
    );

    let result = tool.call(ApplyPatchArgs { patch }).await;

    // @step Then the tool returns an error describing the context mismatch
    assert!(
        result.is_err(),
        "Should return an error for context mismatch"
    );
    let err = format!("{:?}", result.unwrap_err());
    assert!(
        err.contains("Context mismatch") || err.contains("context") || err.contains("mismatch"),
        "Error should describe context mismatch, got: {err}"
    );

    // @step And the file "/tmp/test/mismatch.rs" is not modified
    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(
        content, original_content,
        "File should be unchanged after context mismatch error"
    );
}

// ==========================================
// Scenario: apply_patch tool is registered in Codex agent
// ==========================================

#[tokio::test]
async fn test_apply_patch_tool_definition() {
    // @step Given a CodexProvider configured with a valid model
    let tool = ApplyPatchTool::new(Uuid::nil());

    // @step When create_rig_agent is called
    // (tool definition is validated directly — full agent registration tested in codex provider tests)

    // @step Then the agent has an "apply_patch" tool in its tool set
    assert_eq!(ApplyPatchTool::NAME, "apply_patch");

    let def = tool.definition("".to_string()).await;
    assert_eq!(def.name, "apply_patch");
    assert!(!def.description.is_empty());
    assert!(
        def.description.contains("patch"),
        "Description should mention patch, got: {}",
        def.description
    );

    // @step And the agent does not have "Write" or "Edit" tools registered
    // (verified in rust/providers/src/codex/mod.rs::create_rig_agent_does_not_expose_non_native_glob_tool)
}

// ==========================================
// Scenario: Update file for non-existent path returns error
// ==========================================

#[tokio::test]
async fn test_update_nonexistent_file_returns_error() {
    let tool = ApplyPatchTool::new(Uuid::nil());

    let patch = "*** Begin Patch\n\
         *** Update File: /tmp/nonexistent_apply_patch_test_file.rs\n\
         @@ fn foo() {\n\
         -old\n\
         +new\n\
         *** End Patch"
        .to_string();

    let result = tool.call(ApplyPatchArgs { patch }).await;
    assert!(result.is_err(), "Updating non-existent file should error");
}

// ==========================================
// Scenario: Delete non-existent file returns error
// ==========================================

#[tokio::test]
async fn test_delete_nonexistent_file_returns_error() {
    let tool = ApplyPatchTool::new(Uuid::nil());

    let patch = "*** Begin Patch\n\
         *** Delete File: /tmp/nonexistent_apply_patch_test_file.rs\n\
         *** End Patch"
        .to_string();

    let result = tool.call(ApplyPatchArgs { patch }).await;
    assert!(result.is_err(), "Deleting non-existent file should error");
}
