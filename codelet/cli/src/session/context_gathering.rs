//! Context Gathering for CLI-016
//!
//! Discovers CLAUDE.md/AGENTS.md files and gathers environment information
//! for injection as system reminders.
//!
//! This module implements:
//! 1. CLAUDE.md/AGENTS.md discovery by searching current + parent directories
//! 2. Environment info gathering (platform, arch, shell, user, cwd)
//!
//! The discovered context is injected via the system_reminders module.

use std::path::Path;
use tracing::warn;

/// Context file names to search for (in priority order)
const CONTEXT_FILES: [&str; 2] = ["CLAUDE.md", "AGENTS.md"];

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

        // TUI-064: Add current date in ISO 8601 format (YYYY-MM-DD)
        // This appears after working directory as specified in architecture notes
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
    }
}

#[cfg(test)]
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
        let date_regex = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
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
            fresh_content.contains(&format!("Date: {}", today)),
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
}
