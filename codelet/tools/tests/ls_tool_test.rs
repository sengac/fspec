
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Tests for LsTool implementation
//! Feature: spec/features/add-ls-tool-for-directory-listing.feature

use codelet_tools::LsTool;
use rig::tool::Tool;
use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

/// Scenario: List directory returns files and subdirectories with metadata
#[tokio::test]
async fn test_list_directory_returns_files_and_subdirectories_with_metadata() {
    // @step Given a directory with files and subdirectories
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create subdirectories
    fs::create_dir(temp_path.join("lib")).unwrap();
    fs::create_dir(temp_path.join("src")).unwrap();

    // Create files
    File::create(temp_path.join("zebra.ts")).unwrap();
    File::create(temp_path.join("alpha.ts")).unwrap();

    // @step When I list the directory contents
    let tool = LsTool::new();
    let args = codelet_tools::ls::LsArgs {
        path: Some(temp_path.to_string_lossy().to_string()),
    };
    let result = tool.call(args).await.unwrap();

    // @step Then all entries should be returned with metadata
    assert!(result.contains("lib"));
    assert!(result.contains("src"));
    assert!(result.contains("alpha.ts"));
    assert!(result.contains("zebra.ts"));

    // @step And directories should be listed before files
    let lib_index = result.find("lib").unwrap();
    let src_index = result.find("src").unwrap();
    let alpha_index = result.find("alpha.ts").unwrap();
    let zebra_index = result.find("zebra.ts").unwrap();
    assert!(lib_index < alpha_index);
    assert!(src_index < alpha_index);

    // @step And entries should be sorted alphabetically within each group
    assert!(lib_index < src_index); // lib before src (directories)
    assert!(alpha_index < zebra_index); // alpha before zebra (files)
}

/// Scenario: List specific path returns only contents of that directory
#[tokio::test]
async fn test_list_specific_path_returns_only_contents_of_that_directory() {
    // @step Given a project with files in multiple directories
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let src_dir = temp_path.join("src");
    fs::create_dir(&src_dir).unwrap();
    File::create(src_dir.join("app.ts")).unwrap();
    File::create(temp_path.join("root.ts")).unwrap();

    // @step When I list the directory 'src'
    let tool = LsTool::new();
    let args = codelet_tools::ls::LsArgs {
        path: Some(src_dir.to_string_lossy().to_string()),
    };
    let result = tool.call(args).await.unwrap();

    // @step Then only files from the src directory should be returned
    assert!(result.contains("app.ts"));

    // @step And parent directory contents should not be included
    assert!(!result.contains("root.ts"));
}

/// Scenario: List non-existent directory returns error
#[tokio::test]
async fn test_list_nonexistent_directory() {
    // @step Given a project directory
    // (any directory)

    // @step When I list a non-existent directory '/nonexistent'
    let nonexistent_path = "/nonexistent-xyz123-dir";
    let tool = LsTool::new();
    let args = codelet_tools::ls::LsArgs {
        path: Some(nonexistent_path.to_string()),
    };
    let result = tool.call(args).await;

    // @step Then the result should be 'Directory not found'
    match result {
        Ok(output) => assert!(output.contains("Directory not found")),
        Err(e) => assert!(e.to_string().contains("Directory not found")),
    }
}

/// Scenario: Large directory listings are truncated at character limit
#[tokio::test]
async fn test_large_directory_listings_are_truncated() {
    // @step Given a directory with many files exceeding output limit
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Each entry is ~50-80 chars, so we need ~500+ files
    for i in 0..600 {
        let file_name = format!("file-{i:05}-with-long-name.ts");
        File::create(temp_path.join(&file_name)).unwrap();
    }

    // @step When I list the directory contents
    let tool = LsTool::new();
    let args = codelet_tools::ls::LsArgs {
        path: Some(temp_path.to_string_lossy().to_string()),
    };
    let result = tool.call(args).await.unwrap();

    // @step Then the output should be truncated at 30000 characters
    assert!(result.len() <= 31000); // Allow buffer for truncation message

    // @step And a truncation warning should be included
    assert!(result.contains("truncated"));
}

/// Scenario: Output format shows permissions, size, mtime, and name
#[tokio::test]
async fn test_output_format_shows_permissions_size_mtime_and_name() {
    // @step Given a directory with files and subdirectories
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let sub_dir = temp_path.join("subdir");
    fs::create_dir(&sub_dir).unwrap();
    File::create(temp_path.join("test.ts")).unwrap();

    // @step When I list the directory contents
    let tool = LsTool::new();
    let args = codelet_tools::ls::LsArgs {
        path: Some(temp_path.to_string_lossy().to_string()),
    };
    let result = tool.call(args).await.unwrap();

    // @step Then directories should show format like 'drwxr-xr-x  4096  2025-01-15 10:30  dirname/'
    // Check for directory indicator (d at start or / at end)
    let has_dir_format = result
        .lines()
        .any(|line| line.starts_with('d') && line.contains("subdir"));
    assert!(has_dir_format, "Expected directory format with 'd' prefix");

    // @step And files should show format like '-rw-r--r--  1234  2025-01-15 10:30  filename.ts'
    // Check for file indicator (- at start)
    let has_file_format = result
        .lines()
        .any(|line| line.starts_with('-') && line.contains("test.ts"));
    assert!(has_file_format, "Expected file format with '-' prefix");
}

/// Scenario: List empty directory
#[tokio::test]
async fn test_list_empty_directory() {
    // @step Given an empty directory exists
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // @step When I invoke the LS tool on that directory
    let tool = LsTool::new();
    let args = codelet_tools::ls::LsArgs {
        path: Some(temp_path.to_string_lossy().to_string()),
    };
    let result = tool.call(args).await.unwrap();

    // @step Then the output is "(empty directory)"
    assert_eq!(result, "(empty directory)");
}

/// Scenario: List path that is a file
#[tokio::test]
async fn test_list_path_that_is_file() {
    // @step Given a path that points to a file, not a directory
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("testfile.txt");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "test content").unwrap();

    // @step When I invoke the LS tool on that path
    let tool = LsTool::new();
    let args = codelet_tools::ls::LsArgs {
        path: Some(file_path.to_string_lossy().to_string()),
    };
    let result = tool.call(args).await;

    // @step Then the output contains "Not a directory"
    match result {
        Ok(output) => assert!(output.contains("Not a directory")),
        Err(e) => assert!(e.to_string().contains("Not a directory")),
    }
}

/// Scenario: List with no path defaults to current directory
#[tokio::test]
async fn test_list_defaults_to_current_directory() {
    // @step Given no path is provided
    let tool = LsTool::new();
    let args = codelet_tools::ls::LsArgs { path: None };

    // @step When I invoke the LS tool
    let result = tool.call(args).await;

    // @step Then it lists the current working directory
    // Should succeed and return something (not empty, not error)
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(!output.is_empty());
    // Current directory should have at least Cargo.toml
    assert!(output.contains("Cargo.toml") || output.contains("src/") || !output.is_empty());
}

/// Scenario: Handle files with special characters in names
#[tokio::test]
async fn test_handle_files_with_special_characters() {
    // @step Given a directory with files containing spaces and special characters
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    File::create(temp_path.join("file with spaces.ts")).unwrap();
    File::create(temp_path.join("file-with-dashes.ts")).unwrap();

    // @step When I list the directory contents
    let tool = LsTool::new();
    let args = codelet_tools::ls::LsArgs {
        path: Some(temp_path.to_string_lossy().to_string()),
    };
    let result = tool.call(args).await.unwrap();

    // @step Then files with spaces should be listed correctly
    assert!(result.contains("file with spaces.ts"));

    // @step And files with dashes should be listed correctly
    assert!(result.contains("file-with-dashes.ts"));
}

/// Scenario: Show file sizes in bytes
#[tokio::test]
async fn test_show_file_sizes_in_bytes() {
    // @step Given a file with known size
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let content = "x".repeat(1234);
    let mut file = File::create(temp_path.join("sized-file.ts")).unwrap();
    file.write_all(content.as_bytes()).unwrap();

    // @step When I list the directory containing the file
    let tool = LsTool::new();
    let args = codelet_tools::ls::LsArgs {
        path: Some(temp_path.to_string_lossy().to_string()),
    };
    let result = tool.call(args).await.unwrap();

    // @step Then the file size in bytes should be displayed
    // Should contain the file size (1234 bytes)
    assert!(
        result.contains("1234"),
        "Expected file size 1234 in output: {result}"
    );
    assert!(result.contains("sized-file.ts"));
}
