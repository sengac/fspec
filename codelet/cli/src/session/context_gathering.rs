//! Context Gathering for CLI-016
//!
//! Discovers CLAUDE.md/AGENTS.md files and gathers environment information
//! for injection as system reminders.
//!
//! This module implements:
//! 1. CLAUDE.md/AGENTS.md discovery by searching current + parent directories
//! 2. Environment info gathering (platform, arch, shell, user, cwd)
//! 3. GIT-034: Isolation context for worktree sessions
//!
//! The discovered context is injected via the system_reminders module.

use std::path::Path;
use tracing::warn;

/// Context file names to search for (in priority order)
const CONTEXT_FILES: [&str; 2] = ["CLAUDE.md", "AGENTS.md"];

/// GIT-034: Isolation context for worktree sessions
///
/// When a session runs in an isolated worktree, this context provides
/// information about the isolation state so the AI can inform users
/// about the worktree location and merge/discard options.
#[derive(Debug, Clone, Default)]
pub struct IsolationContext {
    /// Whether the session is running in isolated mode
    pub is_isolated: bool,
    /// Relative path to the worktree (e.g., ".fspec/worktrees/abc123/")
    pub worktree_path: Option<String>,
    /// Short SHA of the base commit (first 8 chars)
    pub base_commit: Option<String>,
}

/// Environment information gathered for system reminder injection
#[derive(Debug, Clone)]
pub struct EnvironmentInfo {
    /// Operating system (e.g., "linux", "macos", "windows")
    pub platform: String,
    /// CPU architecture (e.g., "x86_64", "aarch64")
    pub arch: String,
    /// Shell (e.g., "/bin/bash", "C:\Windows\System32\cmd.exe")
    pub shell: Option<String>,
    /// Username
    pub user: Option<String>,
    /// Current working directory
    pub cwd: Option<String>,
    /// Current date in ISO 8601 format (YYYY-MM-DD), using local time
    pub date: String,
    /// GIT-034: Isolation context for worktree sessions
    pub isolation: Option<IsolationContext>,
}

impl EnvironmentInfo {
    /// Format environment info as system reminder content
    pub fn to_reminder_content(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Platform: {}", self.platform));
        lines.push(format!("Architecture: {}", self.arch));

        if let Some(ref shell) = self.shell {
            lines.push(format!("Shell: {shell}"));
        }

        if let Some(ref user) = self.user {
            lines.push(format!("User: {user}"));
        }

        if let Some(ref cwd) = self.cwd {
            lines.push(format!("Working directory: {cwd}"));
        }

        // GIT-034: Add isolation fields when session is isolated
        // These appear between Working directory and Date
        if let Some(ref isolation) = self.isolation {
            if isolation.is_isolated {
                lines.push("Isolation: ACTIVE".to_string());
                
                if let Some(ref path) = isolation.worktree_path {
                    lines.push(format!("Worktree: {path}"));
                }
                
                if let Some(ref commit) = isolation.base_commit {
                    // Display short SHA (first 8 chars)
                    let short_sha = if commit.len() > 8 {
                        &commit[..8]
                    } else {
                        commit
                    };
                    lines.push(format!("Base commit: {short_sha}"));
                }
            }
        }

        // TUI-064: Add current date in ISO 8601 format (YYYY-MM-DD)
        // This appears last as specified in architecture notes
        lines.push(format!("Date: {}", self.date));

        lines.join("\n")
    }
}

/// Discover CLAUDE.md or AGENTS.md by searching current and parent directories.
///
/// Search order:
/// 1. Current directory for CLAUDE.md
/// 2. Current directory for AGENTS.md
/// 3. Parent directory for CLAUDE.md
/// 4. Parent directory for AGENTS.md
/// 5. Continue up to filesystem root
///
/// # Arguments
/// * `start_path` - Directory to start searching from. If None, uses current working directory.
///
/// # Returns
/// * `Option<String>` - File content if found, None otherwise
pub fn discover_claude_md(start_path: Option<&Path>) -> Option<String> {
    let start = match start_path {
        Some(p) => p.to_path_buf(),
        None => match std::env::current_dir() {
            Ok(cwd) => cwd,
            Err(e) => {
                warn!("Failed to get current directory: {}", e);
                return None;
            }
        },
    };

    let mut current = Some(start.as_path());

    while let Some(dir) = current {
        // Skip directories named "spec" — CLAUDE.md and AGENTS.md placed there are
        // fspec workflow docs intended for CLI-mode agents (e.g. spec/CLAUDE.md,
        // spec/AGENTS.md).  Loading them into the codelet agent context conflicts
        // with the Fspec tool integration which expects the tool-based workflow, not
        // the CLI-command workflow.  All other directory names in the upward walk
        // are unaffected.
        let is_spec_dir = dir
            .file_name()
            .map(|name| name == "spec")
            .unwrap_or(false);

        if !is_spec_dir {
            // Check for each context file in priority order
            for filename in &CONTEXT_FILES {
                let file_path = dir.join(filename);
                if file_path.exists() {
                    match std::fs::read_to_string(&file_path) {
                        Ok(content) => {
                            return Some(content);
                        }
                        Err(e) => {
                            warn!("Failed to read {}: {}", file_path.display(), e);
                            // Continue searching - maybe another file exists
                        }
                    }
                }
            }
        }

        // Move to parent directory
        current = dir.parent();
    }

    None
}

/// Gather environment information for system reminder injection.
///
/// Collects:
/// - Platform (OS)
/// - Architecture
/// - Shell (from SHELL env var on Unix, COMSPEC on Windows)
/// - Username (from USER on Unix, USERNAME on Windows)
/// - Current working directory
/// - Current date in ISO 8601 format (YYYY-MM-DD) using local time
///
/// # Returns
/// * `EnvironmentInfo` - Gathered environment information
pub fn gather_environment_info() -> EnvironmentInfo {
    let platform = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string();

    // Get shell - SHELL on Unix, COMSPEC on Windows
    let shell = std::env::var("SHELL")
        .ok()
        .or_else(|| std::env::var("COMSPEC").ok());

    // Get username - USER on Unix, USERNAME on Windows
    let user = std::env::var("USER")
        .ok()
        .or_else(|| std::env::var("USERNAME").ok());

    // Get current working directory
    let cwd = std::env::current_dir()
        .ok()
        .map(|p| p.to_string_lossy().to_string());

    // TUI-064: Get current date in ISO 8601 format using local time
    // This ensures AI agents know today's date and don't rely on training data
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();

    EnvironmentInfo {
        platform,
        arch,
        shell,
        user,
        cwd,
        date,
        isolation: None, // GIT-034: No isolation by default
    }
}

/// GIT-034: Gather environment information with isolation context.
///
/// Same as `gather_environment_info()` but also includes isolation context
/// for worktree sessions. When the session is isolated, the reminder will
/// include Isolation, Worktree path, and Base commit fields.
///
/// # Arguments
/// * `isolation` - Optional isolation context for worktree sessions
///
/// # Returns
/// * `EnvironmentInfo` - Gathered environment information with isolation context
pub fn gather_environment_info_with_isolation(isolation: Option<&IsolationContext>) -> EnvironmentInfo {
    let mut info = gather_environment_info();
    info.isolation = isolation.cloned();
    info
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_info_to_reminder_content() {
        let info = EnvironmentInfo {
            platform: "linux".to_string(),
            arch: "x86_64".to_string(),
            shell: Some("/bin/bash".to_string()),
            user: Some("testuser".to_string()),
            cwd: Some("/home/testuser/project".to_string()),
            date: "2026-02-14".to_string(),
            isolation: None,
        };

        let content = info.to_reminder_content();

        assert!(content.contains("Platform: linux"));
        assert!(content.contains("Architecture: x86_64"));
        assert!(content.contains("Shell: /bin/bash"));
        assert!(content.contains("User: testuser"));
        assert!(content.contains("Working directory: /home/testuser/project"));
        // @step Then the environment information should contain "Date:" followed by a date
        assert!(content.contains("Date: 2026-02-14"));
    }

    #[test]
    fn test_environment_info_to_reminder_content_minimal() {
        let info = EnvironmentInfo {
            platform: "windows".to_string(),
            arch: "x86_64".to_string(),
            shell: None,
            user: None,
            cwd: None,
            date: "2026-02-14".to_string(),
            isolation: None,
        };

        let content = info.to_reminder_content();

        assert!(content.contains("Platform: windows"));
        assert!(content.contains("Architecture: x86_64"));
        assert!(!content.contains("Shell:"));
        assert!(!content.contains("User:"));
        assert!(!content.contains("Working directory:"));
        // @step Then the environment information should contain "Date:" followed by a date
        assert!(content.contains("Date: 2026-02-14"));
    }

    #[test]
    fn test_gather_environment_info_has_platform() {
        let info = gather_environment_info();
        assert!(!info.platform.is_empty());
    }

    #[test]
    fn test_gather_environment_info_has_arch() {
        let info = gather_environment_info();
        assert!(!info.arch.is_empty());
    }

    /// Feature: spec/features/work-unit-context-backend.feature
    /// Scenario: Current date appears in environment information
    #[test]
    fn test_gather_environment_info_has_date_in_iso_format() {
        // @step When the CLI starts an interactive session
        let info = gather_environment_info();

        // @step Then the environment information should contain "Date:" followed by a date in YYYY-MM-DD format
        assert!(!info.date.is_empty(), "Date field should not be empty");

        // Verify ISO 8601 format (YYYY-MM-DD)
        // Note: using expect() here as the regex pattern is compile-time constant and known valid
        let date_regex = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$")
            .expect("valid regex pattern");
        assert!(
            date_regex.is_match(&info.date),
            "Date '{}' should be in YYYY-MM-DD format",
            info.date
        );
    }

    /// Feature: spec/features/work-unit-context-backend.feature
    /// Scenario: Current date appears in environment information
    #[test]
    fn test_gather_environment_info_date_is_local_date() {
        // @step When the CLI starts an interactive session
        let info = gather_environment_info();

        // @step And the date should be the system's local date
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        assert_eq!(
            info.date, today,
            "Date should match today's local date, got '{}' expected '{}'",
            info.date, today
        );
    }

    /// Feature: spec/features/work-unit-context-backend.feature
    /// Scenario: Current date appears in environment information
    #[test]
    fn test_date_appears_after_working_directory_in_reminder() {
        let info = EnvironmentInfo {
            platform: "linux".to_string(),
            arch: "x86_64".to_string(),
            shell: Some("/bin/bash".to_string()),
            user: Some("testuser".to_string()),
            cwd: Some("/home/testuser/project".to_string()),
            date: "2026-02-14".to_string(),
            isolation: None,
        };

        let content = info.to_reminder_content();
        let lines: Vec<&str> = content.lines().collect();

        // Find indices of Working directory and Date lines
        let cwd_index = lines.iter().position(|l| l.starts_with("Working directory:"));
        let date_index = lines.iter().position(|l| l.starts_with("Date:"));

        assert!(cwd_index.is_some(), "Working directory line should exist");
        assert!(date_index.is_some(), "Date line should exist");

        // @step Then the date field should appear after the working directory
        assert!(
            date_index.unwrap() > cwd_index.unwrap(),
            "Date should appear after Working directory in the output"
        );
    }

    /// Feature: spec/features/work-unit-context-backend.feature
    /// Scenario: AI must use environment date not training data
    ///
    /// This test documents that the date is available in environment info for AI to use.
    /// The actual AI behavior (using the date) is tested through the environment info
    /// being present and accessible in the system reminder content.
    #[test]
    fn test_environment_date_available_for_ai() {
        // @step Given the environment information contains "Date: 2026-02-14"
        let info = EnvironmentInfo {
            platform: "linux".to_string(),
            arch: "x86_64".to_string(),
            shell: Some("/bin/bash".to_string()),
            user: Some("testuser".to_string()),
            cwd: Some("/home/testuser/project".to_string()),
            date: "2026-02-14".to_string(),
            isolation: None,
        };

        let content = info.to_reminder_content();

        // @step When the AI needs to reference today's date
        // The AI receives this content as a system reminder
        // We verify the date is prominently displayed

        // @step Then the AI must use the date from the environment information
        assert!(
            content.contains("Date: 2026-02-14"),
            "Environment info must contain Date field for AI to use"
        );

        // @step And the AI must not guess or use dates from training data
        // This is ensured by having the date clearly labeled in the environment info
        // The AI's system prompt instructs it to use this date
        let date_line = content.lines().find(|l| l.starts_with("Date:"));
        assert!(
            date_line.is_some(),
            "Date line must be clearly labeled for AI to identify"
        );
        assert!(
            date_line.unwrap().starts_with("Date: "),
            "Date must be in clear 'Date: YYYY-MM-DD' format"
        );
    }

    /// Feature: spec/features/work-unit-context-backend.feature
    /// Scenario: Resumed session gets fresh environment info with current date
    ///
    /// This test verifies that when environment info is re-gathered (as happens on
    /// session resume), it gets the current date, not stale data.
    #[test]
    fn test_environment_info_gathers_fresh_date_each_time() {
        // @step Given a session was created yesterday with "Date: 2026-02-13" in environment info
        // The old environment info had date "2026-02-13"
        let old_info = EnvironmentInfo {
            platform: "linux".to_string(),
            arch: "x86_64".to_string(),
            shell: Some("/bin/bash".to_string()),
            user: Some("testuser".to_string()),
            cwd: Some("/home/testuser/project".to_string()),
            date: "2026-02-13".to_string(),
            isolation: None,
        };
        let old_content = old_info.to_reminder_content();
        assert!(
            old_content.contains("Date: 2026-02-13"),
            "Old environment should have old date"
        );

        // @step When I resume that session today on 2026-02-14
        // Resume calls inject_context_reminders() which gathers fresh environment info
        // We verify that gather_environment_info() returns fresh data

        // @step Then the environment information should be reinjected with fresh data
        let fresh_info = gather_environment_info();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();

        // @step And the environment information should contain "Date: 2026-02-14"
        // (or whatever today's actual date is)
        assert_eq!(
            fresh_info.date, today,
            "Fresh environment info should have today's date"
        );

        // @step And the AI should see today's date, not yesterday's
        let fresh_content = fresh_info.to_reminder_content();
        assert!(
            fresh_content.contains(&format!("Date: {today}")),
            "Fresh content should have today's date"
        );
        // The old date should not be present in fresh content
        if today != "2026-02-13" {
            assert!(
                !fresh_content.contains("Date: 2026-02-13"),
                "Fresh content should NOT have old date"
            );
        }
    }

    // GIT-034: Tests for isolation context in environment reminder
    
    /// Feature: spec/features/ai-system-reminder-includes-isolation-state-and-worktree-path.feature
    /// Scenario: Isolated session environment reminder includes isolation fields
    #[test]
    fn test_isolation_context_included_in_reminder() {
        // @step Given a session is created with isolated mode enabled
        let info = EnvironmentInfo {
            platform: "linux".to_string(),
            arch: "x86_64".to_string(),
            shell: Some("/bin/bash".to_string()),
            user: Some("testuser".to_string()),
            cwd: Some("/home/testuser/project".to_string()),
            date: "2026-02-20".to_string(),
            isolation: Some(IsolationContext {
                is_isolated: true,
                worktree_path: Some(".fspec/worktrees/abc-123/".to_string()),
                base_commit: Some("7a8b9c0def123456".to_string()),
            }),
        };

        // @step When the environment system-reminder is generated
        let content = info.to_reminder_content();

        // @step Then the reminder should contain "Isolation: ACTIVE"
        assert!(
            content.contains("Isolation: ACTIVE"),
            "Isolated session should have Isolation: ACTIVE. Got:\n{content}"
        );

        // @step And the reminder should contain "Worktree: .fspec/worktrees/abc-123/"
        assert!(
            content.contains("Worktree: .fspec/worktrees/abc-123/"),
            "Isolated session should have worktree path. Got:\n{content}"
        );

        // @step And the reminder should contain "Base commit: 7a8b9c0d"
        assert!(
            content.contains("Base commit: 7a8b9c0d"),
            "Isolated session should have short base commit. Got:\n{content}"
        );
    }

    /// Feature: spec/features/ai-system-reminder-includes-isolation-state-and-worktree-path.feature
    /// Scenario: Non-isolated session environment reminder excludes isolation fields
    #[test]
    fn test_non_isolated_excludes_isolation_fields() {
        // @step Given a session is created with isolated mode disabled
        let info = EnvironmentInfo {
            platform: "linux".to_string(),
            arch: "x86_64".to_string(),
            shell: Some("/bin/bash".to_string()),
            user: Some("testuser".to_string()),
            cwd: Some("/home/testuser/project".to_string()),
            date: "2026-02-20".to_string(),
            isolation: None,
        };

        // @step When the environment system-reminder is generated
        let content = info.to_reminder_content();

        // @step Then the reminder should NOT contain "Isolation:"
        assert!(
            !content.contains("Isolation:"),
            "Non-isolated session should NOT have Isolation field. Got:\n{content}"
        );

        // @step And the reminder should NOT contain "Worktree:"
        assert!(
            !content.contains("Worktree:"),
            "Non-isolated session should NOT have Worktree field. Got:\n{content}"
        );

        // @step And the reminder should NOT contain "Base commit:"
        assert!(
            !content.contains("Base commit:"),
            "Non-isolated session should NOT have Base commit field. Got:\n{content}"
        );

        // @step And the reminder should contain "Working directory:"
        assert!(
            content.contains("Working directory:"),
            "Non-isolated session should have Working directory. Got:\n{content}"
        );
    }

    /// GIT-034: Test that gather_environment_info_with_isolation includes isolation context
    #[test]
    fn test_gather_with_isolation_includes_context() {
        let isolation = IsolationContext {
            is_isolated: true,
            worktree_path: Some(".fspec/worktrees/test-session/".to_string()),
            base_commit: Some("abcdef12".to_string()),
        };

        let info = gather_environment_info_with_isolation(Some(&isolation));
        
        assert!(info.isolation.is_some(), "Isolation context should be set");
        let iso = info.isolation.as_ref().unwrap();
        assert!(iso.is_isolated, "is_isolated should be true");
        assert_eq!(
            iso.worktree_path.as_ref().unwrap(),
            ".fspec/worktrees/test-session/"
        );
        assert_eq!(iso.base_commit.as_ref().unwrap(), "abcdef12");
    }

    /// GIT-034: Test that gather_environment_info_with_isolation(None) has no isolation
    #[test]
    fn test_gather_with_isolation_none_has_no_context() {
        let info = gather_environment_info_with_isolation(None);
        
        assert!(info.isolation.is_none(), "Isolation context should be None");
        
        // Content should not contain isolation fields
        let content = info.to_reminder_content();
        assert!(!content.contains("Isolation:"), "Should not have Isolation field");
        assert!(!content.contains("Worktree:"), "Should not have Worktree field");
        assert!(!content.contains("Base commit:"), "Should not have Base commit field");
    }

    /// GIT-034: Test that isolation fields appear in correct order
    #[test]
    fn test_isolation_fields_order() {
        let info = EnvironmentInfo {
            platform: "linux".to_string(),
            arch: "x86_64".to_string(),
            shell: Some("/bin/bash".to_string()),
            user: Some("testuser".to_string()),
            cwd: Some("/home/testuser/project".to_string()),
            date: "2026-02-20".to_string(),
            isolation: Some(IsolationContext {
                is_isolated: true,
                worktree_path: Some(".fspec/worktrees/test/".to_string()),
                base_commit: Some("12345678".to_string()),
            }),
        };

        let content = info.to_reminder_content();
        let lines: Vec<&str> = content.lines().collect();

        let cwd_pos = lines.iter().position(|l| l.starts_with("Working directory:"));
        let isolation_pos = lines.iter().position(|l| l.starts_with("Isolation:"));
        let worktree_pos = lines.iter().position(|l| l.starts_with("Worktree:"));
        let commit_pos = lines.iter().position(|l| l.starts_with("Base commit:"));
        let date_pos = lines.iter().position(|l| l.starts_with("Date:"));

        // Isolation should appear after Working directory
        assert!(
            isolation_pos.unwrap() > cwd_pos.unwrap(),
            "Isolation should appear after Working directory"
        );

        // Worktree should appear after Isolation
        assert!(
            worktree_pos.unwrap() > isolation_pos.unwrap(),
            "Worktree should appear after Isolation"
        );

        // Base commit should appear after Worktree
        assert!(
            commit_pos.unwrap() > worktree_pos.unwrap(),
            "Base commit should appear after Worktree"
        );

        // Date should appear last
        assert!(
            date_pos.unwrap() > commit_pos.unwrap(),
            "Date should appear after Base commit (last)"
        );
    }

    /// GIT-034: Test that base commit displays short SHA
    #[test]
    fn test_base_commit_short_sha() {
        let info = EnvironmentInfo {
            platform: "linux".to_string(),
            arch: "x86_64".to_string(),
            shell: None,
            user: None,
            cwd: None,
            date: "2026-02-20".to_string(),
            isolation: Some(IsolationContext {
                is_isolated: true,
                worktree_path: Some(".fspec/worktrees/test/".to_string()),
                base_commit: Some("abcdef1234567890abcdef".to_string()), // Long SHA
            }),
        };

        let content = info.to_reminder_content();
        
        // Should display short SHA (first 8 chars)
        assert!(
            content.contains("Base commit: abcdef12"),
            "Should display short SHA (8 chars). Got:\n{content}"
        );
        
        // Should NOT display full SHA
        assert!(
            !content.contains("abcdef1234567890abcdef"),
            "Should NOT display full SHA"
        );
    }

    // INIT-017: spec/ directory exclusion tests
    //
    // spec/CLAUDE.md and spec/AGENTS.md contain instructions for using fspec as a
    // CLI tool.  When the codelet agent is running, fspec is used via the Fspec tool
    // (tool-based workflow), so those files must never be loaded into context — they
    // would conflict with the tool-based instructions already in the system prompt.

    /// INIT-017: discover_claude_md must skip a directory named "spec"
    #[test]
    fn test_discover_claude_md_skips_spec_directory() {
        use std::fs;
        use tempfile::TempDir;

        // Build:  <tmpdir>/spec/CLAUDE.md   ← must be ignored
        //         <tmpdir>/AGENTS.md        ← must be returned
        let tmp = TempDir::new().expect("create tempdir");
        let spec_dir = tmp.path().join("spec");
        fs::create_dir_all(&spec_dir).expect("create spec/");

        fs::write(spec_dir.join("CLAUDE.md"), "# spec/CLAUDE.md content — CLI instructions").expect("write spec/CLAUDE.md");
        fs::write(tmp.path().join("AGENTS.md"), "# AGENTS.md root content").expect("write AGENTS.md");

        // Start search from within spec/ — simulates an agent launched there
        let result = discover_claude_md(Some(&spec_dir));

        assert!(
            result.is_some(),
            "Should find AGENTS.md in parent after skipping spec/"
        );
        let content = result.unwrap();
        assert!(
            content.contains("AGENTS.md root content"),
            "Should return root AGENTS.md content, not spec/CLAUDE.md. Got:\n{content}"
        );
        assert!(
            !content.contains("CLI instructions"),
            "Should NOT return spec/CLAUDE.md content. Got:\n{content}"
        );
    }

    /// INIT-017: spec/AGENTS.md must also be skipped
    #[test]
    fn test_discover_claude_md_skips_spec_agents_md() {
        use std::fs;
        use tempfile::TempDir;

        // Build:  <tmpdir>/spec/AGENTS.md   ← must be ignored
        //         <tmpdir>/CLAUDE.md         ← must be returned
        let tmp = TempDir::new().expect("create tempdir");
        let spec_dir = tmp.path().join("spec");
        fs::create_dir_all(&spec_dir).expect("create spec/");

        fs::write(spec_dir.join("AGENTS.md"), "# spec/AGENTS.md — CLI instructions").expect("write spec/AGENTS.md");
        fs::write(tmp.path().join("CLAUDE.md"), "# Root CLAUDE.md").expect("write root CLAUDE.md");

        let result = discover_claude_md(Some(&spec_dir));

        assert!(result.is_some(), "Should find root CLAUDE.md");
        let content = result.unwrap();
        assert!(
            content.contains("Root CLAUDE.md"),
            "Should return root CLAUDE.md, not spec/AGENTS.md. Got:\n{content}"
        );
        assert!(
            !content.contains("CLI instructions"),
            "Should NOT return spec/AGENTS.md content. Got:\n{content}"
        );
    }

    /// INIT-017: when started from root (not inside spec/), root files still load
    #[test]
    fn test_discover_claude_md_still_loads_root_agents_md_from_root() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("create tempdir");
        fs::write(tmp.path().join("AGENTS.md"), "# Root AGENTS.md").expect("write AGENTS.md");

        // Only a spec/ dir with its own CLAUDE.md — root search should still find root file
        let spec_dir = tmp.path().join("spec");
        fs::create_dir_all(&spec_dir).expect("create spec/");
        fs::write(spec_dir.join("CLAUDE.md"), "# spec/CLAUDE.md").expect("write spec/CLAUDE.md");

        // Start from project root (not spec/)
        let result = discover_claude_md(Some(tmp.path()));

        assert!(result.is_some(), "Should find root AGENTS.md");
        let content = result.unwrap();
        assert!(
            content.contains("Root AGENTS.md"),
            "Should return root AGENTS.md. Got:\n{content}"
        );
    }

    /// INIT-017: when no root file exists either, returns None gracefully
    #[test]
    fn test_discover_claude_md_returns_none_when_only_spec_files_exist() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("create tempdir");
        let spec_dir = tmp.path().join("spec");
        fs::create_dir_all(&spec_dir).expect("create spec/");
        fs::write(spec_dir.join("CLAUDE.md"), "# spec/CLAUDE.md").expect("write spec/CLAUDE.md");
        fs::write(spec_dir.join("AGENTS.md"), "# spec/AGENTS.md").expect("write spec/AGENTS.md");

        // No root-level file exists; filesystem root will be reached without finding anything
        let result = discover_claude_md(Some(&spec_dir));

        // We cannot assert None because parent dirs up to fs root may have one,
        // but we CAN assert the spec/ files were not returned
        if let Some(content) = result {
            assert!(
                !content.contains("spec/CLAUDE.md"),
                "spec/CLAUDE.md content must never be returned"
            );
            assert!(
                !content.contains("spec/AGENTS.md"),
                "spec/AGENTS.md content must never be returned"
            );
        }
        // None is also acceptable
    }
}
