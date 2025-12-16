//! Test for AstGrep rig::tool::Tool implementation

use codelet_tools::AstGrepTool;
use rig::tool::Tool;
use std::fs;
use tempfile::TempDir;

/// Test that the rig::tool::Tool interface correctly passes the language parameter
#[tokio::test]
async fn test_astgrep_rig_tool_language_parameter() {
    // Create a temp directory with a Rust file
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("test.rs");
    fs::write(&file, "fn hello() { println!(\"hello\"); }").unwrap();

    // Use the rig::tool::Tool interface
    let tool = AstGrepTool::new();
    let args = codelet_tools::astgrep::AstGrepArgs {
        pattern: "fn $NAME".to_string(),
        language: "rust".to_string(),
        path: Some(temp_dir.path().to_string_lossy().to_string()),
    };

    // This should NOT return "language parameter is required"
    let result = tool.call(args).await;

    assert!(result.is_ok(), "Expected Ok, got error: {:?}", result.err());
    let content = result.unwrap();
    assert!(!content.contains("language parameter is required"), 
        "Bug: language parameter was not passed correctly. Got: {}", content);
    assert!(content.contains("test.rs") || content.contains("hello"),
        "Expected to find the function. Got: {}", content);
}
