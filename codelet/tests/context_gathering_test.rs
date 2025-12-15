//! Feature: spec/features/context-gathering.feature
//!
//! Tests for CLI-016: Context Gathering with CLAUDE.md Discovery

use codelet::session::context_gathering::{discover_claude_md, gather_environment_info};
use std::fs;
use tempfile::TempDir;

/// Scenario: Discover CLAUDE.md in current directory
#[test]
fn test_discover_claude_md_in_current_directory() {
    // @step Given a CLAUDE.md file exists in the current working directory
    let temp_dir = TempDir::new().unwrap();
    let claude_md_path = temp_dir.path().join("CLAUDE.md");
    fs::write(
        &claude_md_path,
        "# Project Context\n\nThis is test content.",
    )
    .unwrap();

    // @step When the CLI searches for CLAUDE.md
    let content = discover_claude_md(Some(temp_dir.path()));

    // @step Then the CLAUDE.md content should be found
    assert!(content.is_some());
    let content = content.unwrap();
    assert!(content.contains("Project Context"));
    assert!(content.contains("This is test content"));
}

/// Scenario: Discover CLAUDE.md in parent directory
#[test]
fn test_discover_claude_md_in_parent_directory() {
    // @step Given no CLAUDE.md file exists in the current working directory
    // @step And a CLAUDE.md file exists in a parent directory
    let temp_dir = TempDir::new().unwrap();
    let parent_claude_md = temp_dir.path().join("CLAUDE.md");
    fs::write(&parent_claude_md, "# Parent Project Context").unwrap();

    // Create child directory without CLAUDE.md
    let child_dir = temp_dir.path().join("subdir");
    fs::create_dir(&child_dir).unwrap();

    // @step When the CLI searches for CLAUDE.md starting from child directory
    let content = discover_claude_md(Some(&child_dir));

    // @step Then the CLAUDE.md content from the parent directory should be found
    assert!(content.is_some());
    let content = content.unwrap();
    assert!(content.contains("Parent Project Context"));
}

/// Scenario: Discover AGENTS.md as fallback
#[test]
fn test_discover_agents_md_as_fallback() {
    // @step Given no CLAUDE.md file exists in any parent directory
    // @step And an AGENTS.md file exists in the current working directory
    let temp_dir = TempDir::new().unwrap();
    let agents_md_path = temp_dir.path().join("AGENTS.md");
    fs::write(
        &agents_md_path,
        "# Agent Configuration\n\nAgent rules here.",
    )
    .unwrap();

    // @step When the CLI searches for context files
    let content = discover_claude_md(Some(temp_dir.path()));

    // @step Then the AGENTS.md content should be found
    assert!(content.is_some());
    let content = content.unwrap();
    assert!(content.contains("Agent Configuration"));
}

/// Scenario: CLAUDE.md takes priority over AGENTS.md
#[test]
fn test_claude_md_takes_priority_over_agents_md() {
    // @step Given both CLAUDE.md and AGENTS.md exist
    let temp_dir = TempDir::new().unwrap();
    let claude_md_path = temp_dir.path().join("CLAUDE.md");
    let agents_md_path = temp_dir.path().join("AGENTS.md");
    fs::write(&claude_md_path, "# CLAUDE content").unwrap();
    fs::write(&agents_md_path, "# AGENTS content").unwrap();

    // @step When the CLI searches for context files
    let content = discover_claude_md(Some(temp_dir.path()));

    // @step Then the CLAUDE.md content should be preferred
    assert!(content.is_some());
    let content = content.unwrap();
    assert!(content.contains("CLAUDE content"));
    assert!(!content.contains("AGENTS content"));
}

/// Scenario: No context file found
#[test]
fn test_no_context_file_found() {
    // @step Given no CLAUDE.md or AGENTS.md file exists
    let temp_dir = TempDir::new().unwrap();

    // @step When the CLI searches for context files
    let content = discover_claude_md(Some(temp_dir.path()));

    // @step Then no content should be found
    assert!(content.is_none());
}

/// Scenario: Gather environment information
#[test]
fn test_gather_environment_info() {
    // @step When the CLI gathers environment information
    let env_info = gather_environment_info();

    // @step Then it should contain the platform (OS)
    assert!(!env_info.platform.is_empty());

    // @step And it should contain the architecture
    assert!(!env_info.arch.is_empty());

    // @step And it should contain the current working directory
    assert!(env_info.cwd.is_some());
}

/// Scenario: Environment info formatting
#[test]
fn test_environment_info_formatting() {
    // @step Given environment information is gathered
    let env_info = gather_environment_info();

    // @step When formatted as a system reminder
    let formatted = env_info.to_reminder_content();

    // @step Then it should contain labeled sections
    assert!(formatted.contains("Platform:"));
    assert!(formatted.contains("Architecture:"));
    assert!(formatted.contains("Working directory:"));
}

/// Scenario: Environment info contains user (when available)
#[test]
fn test_environment_info_user() {
    let env_info = gather_environment_info();

    // User should be Some on most systems
    if env_info.user.is_some() {
        let formatted = env_info.to_reminder_content();
        assert!(formatted.contains("User:"));
    }
}

/// Scenario: Environment info contains shell (when available)
#[test]
fn test_environment_info_shell() {
    let env_info = gather_environment_info();

    // Shell should be Some on Unix systems
    if env_info.shell.is_some() {
        let formatted = env_info.to_reminder_content();
        assert!(formatted.contains("Shell:"));
    }
}

/// Scenario: EnvironmentInfo has expected platform values
#[test]
fn test_environment_info_platform_values() {
    let env_info = gather_environment_info();

    // Platform should be one of the known values
    let known_platforms = ["linux", "macos", "windows", "freebsd", "openbsd", "netbsd"];
    assert!(
        known_platforms
            .iter()
            .any(|p| env_info.platform.contains(p)),
        "Platform '{}' not in expected values",
        env_info.platform
    );
}

/// Scenario: CLAUDE.md content is read completely
#[test]
fn test_claude_md_read_completely() {
    let temp_dir = TempDir::new().unwrap();
    let claude_md_path = temp_dir.path().join("CLAUDE.md");

    // Create multi-section content
    let long_content = r#"# Project Overview

This is the first section.

## Architecture

This is the architecture section.

## Guidelines

- Rule 1
- Rule 2
- Rule 3

## Notes

Final notes here.
"#;

    fs::write(&claude_md_path, long_content).unwrap();

    let content = discover_claude_md(Some(temp_dir.path()));

    // All sections should be present
    assert!(content.is_some());
    let content = content.unwrap();
    assert!(content.contains("Project Overview"));
    assert!(content.contains("Architecture"));
    assert!(content.contains("Guidelines"));
    assert!(content.contains("Rule 3"));
    assert!(content.contains("Final notes"));
}

/// Scenario: Handle nested directory structure
#[test]
fn test_nested_directory_claude_md_discovery() {
    let temp_dir = TempDir::new().unwrap();

    // Create nested structure: root/a/b/c
    let dir_a = temp_dir.path().join("a");
    let dir_b = dir_a.join("b");
    let dir_c = dir_b.join("c");
    fs::create_dir_all(&dir_c).unwrap();

    // Put CLAUDE.md in dir_a (middle of hierarchy)
    let claude_md_path = dir_a.join("CLAUDE.md");
    fs::write(&claude_md_path, "# Middle Level Context").unwrap();

    // Search from dir_c (deepest level)
    let content = discover_claude_md(Some(&dir_c));

    assert!(content.is_some());
    let content = content.unwrap();
    assert!(content.contains("Middle Level Context"));
}

/// Scenario: Use current directory when no path specified
#[test]
fn test_discover_claude_md_uses_current_dir() {
    // When called with None, should use std::env::current_dir()
    // This test just verifies it doesn't panic
    let _ = discover_claude_md(None);
}
