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

    EnvironmentInfo {
        platform,
        arch,
        shell,
        user,
        cwd,
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
        };

        let content = info.to_reminder_content();

        assert!(content.contains("Platform: linux"));
        assert!(content.contains("Architecture: x86_64"));
        assert!(content.contains("Shell: /bin/bash"));
        assert!(content.contains("User: testuser"));
        assert!(content.contains("Working directory: /home/testuser/project"));
    }

    #[test]
    fn test_environment_info_to_reminder_content_minimal() {
        let info = EnvironmentInfo {
            platform: "windows".to_string(),
            arch: "x86_64".to_string(),
            shell: None,
            user: None,
            cwd: None,
        };

        let content = info.to_reminder_content();

        assert!(content.contains("Platform: windows"));
        assert!(content.contains("Architecture: x86_64"));
        assert!(!content.contains("Shell:"));
        assert!(!content.contains("User:"));
        assert!(!content.contains("Working directory:"));
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
}
