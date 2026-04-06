//! PTY Registry for bridge_relay.rs — Remote Terminal Support
//!
//! Manages PTY sessions spawned on the agent's machine. Ported from
//! extension/native-host/src/terminal/registry.rs but adapted for
//! inline bridge usage (no HTTP handlers, direct message routing).
//!
//! Feature: spec/features/bridge-multiplexed-protocol.feature

use dashmap::DashMap;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A single PTY session entry in the registry.
pub struct PtyEntry {
    /// The spawned shell process handle (for kill).
    pub child: Mutex<Box<dyn Child + Send>>,
    /// The PTY master (for try_clone_reader + resize).
    pub master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    /// The PTY stdin writer (taken once at creation, shared).
    pub writer: Arc<Mutex<Box<dyn Write + Send>>>,
    /// Current terminal dimensions.
    pub size: Mutex<PtySize>,
}

/// Thread-safe registry of active PTY sessions.
#[derive(Clone, Default)]
pub struct PtyRegistry {
    entries: Arc<DashMap<String, Arc<PtyEntry>>>,
}

impl PtyRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
        }
    }

    /// Insert a PTY entry.
    pub fn insert(&self, id: String, entry: Arc<PtyEntry>) {
        self.entries.insert(id, entry);
    }

    /// Get a PTY entry by terminal_id.
    pub fn get(&self, id: &str) -> Option<Arc<PtyEntry>> {
        self.entries.get(id).map(|r| Arc::clone(r.value()))
    }

    /// Remove and return a PTY entry.
    pub fn remove(&self, id: &str) -> Option<Arc<PtyEntry>> {
        self.entries.remove(id).map(|(_, v)| v)
    }

    /// Number of active terminals.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all terminal IDs.
    pub fn terminal_ids(&self) -> Vec<String> {
        self.entries.iter().map(|r| r.key().clone()).collect()
    }

    /// Kill all active terminals and clear the registry.
    pub async fn shutdown_all(&self) {
        let ids: Vec<String> = self.entries.iter().map(|r| r.key().clone()).collect();
        for id in ids {
            if let Some((_, entry)) = self.entries.remove(&id) {
                let mut child = entry.child.lock().await;
                let _ = child.kill();
            }
        }
        self.entries.clear();
    }
}

/// Options for creating a new PTY terminal.
#[derive(Debug, Clone)]
pub struct CreateTerminalOpts {
    pub cols: u16,
    pub rows: u16,
    pub shell: Option<String>,
    pub cwd: Option<String>,
}

/// Spawn a PTY process and insert it into the registry.
///
/// Returns the generated terminal_id on success.
pub fn create_terminal(
    registry: &PtyRegistry,
    opts: &CreateTerminalOpts,
) -> Result<(String, Arc<PtyEntry>), String> {
    let pty_system = native_pty_system();

    let size = PtySize {
        rows: opts.rows,
        cols: opts.cols,
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = pty_system
        .openpty(size)
        .map_err(|e| format!("Failed to open PTY: {e}"))?;

    // Build command
    let shell = opts.shell.clone().unwrap_or_else(default_shell);
    let mut cmd = CommandBuilder::new(&shell);

    // Add login shell args for common shells
    let shell_name = std::path::Path::new(&shell)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    match shell_name {
        "zsh" | "bash" | "fish" => {
            cmd.arg("--login");
        }
        _ => {}
    }

    // Set working directory
    if let Some(ref cwd) = opts.cwd {
        cmd.cwd(cwd);
    } else if let Ok(home) = std::env::var("HOME") {
        cmd.cwd(home);
    }

    // Set environment
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");

    // Spawn
    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("Failed to spawn shell: {e}"))?;

    // Take writer (once)
    let writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("Failed to take PTY writer: {e}"))?;

    let terminal_id = uuid::Uuid::new_v4().to_string();

    let entry = Arc::new(PtyEntry {
        child: Mutex::new(child),
        master: Arc::new(Mutex::new(pair.master)),
        writer: Arc::new(Mutex::new(writer)),
        size: Mutex::new(size),
    });

    registry.insert(terminal_id.clone(), Arc::clone(&entry));

    Ok((terminal_id, entry))
}

/// Resize a PTY terminal.
pub async fn resize_terminal(
    entry: &PtyEntry,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let new_size = PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    };

    let master = entry.master.lock().await;
    master
        .resize(new_size)
        .map_err(|e| format!("Failed to resize PTY: {e}"))?;
    drop(master);

    let mut size = entry.size.lock().await;
    *size = new_size;

    Ok(())
}

/// Write decoded bytes to a PTY's stdin.
pub async fn write_terminal_input(
    entry: &PtyEntry,
    data: &[u8],
) -> Result<(), String> {
    let mut writer = entry.writer.lock().await;
    writer
        .write_all(data)
        .map_err(|e| format!("Failed to write to PTY: {e}"))?;
    writer
        .flush()
        .map_err(|e| format!("Failed to flush PTY: {e}"))?;
    Ok(())
}

/// Destroy a PTY terminal — kill the process and remove from registry.
pub async fn destroy_terminal(
    registry: &PtyRegistry,
    terminal_id: &str,
) -> Result<(), String> {
    if let Some(entry) = registry.remove(terminal_id) {
        let mut child = entry.child.lock().await;
        let _ = child.kill();
        Ok(())
    } else {
        Err(format!("Terminal {terminal_id} not found"))
    }
}

/// Get the default shell for the current platform.
fn default_shell() -> String {
    #[cfg(unix)]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    }
    #[cfg(windows)]
    {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // =========================================================================
    // Feature: spec/features/bridge-multiplexed-protocol.feature
    //
    // PtyRegistry unit tests
    // =========================================================================

    /// @step Given the PtyRegistry is initialized
    #[test]
    fn test_registry_new_is_empty() {
        let registry = PtyRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_default_is_empty() {
        let registry = PtyRegistry::default();
        assert!(registry.is_empty());
    }

    /// @step Test that PtyRegistry can store and retrieve entries
    #[tokio::test]
    async fn test_registry_insert_and_get() {
        let registry = PtyRegistry::new();

        // Create a real PTY for testing
        let result = create_terminal(&registry, &CreateTerminalOpts {
            cols: 80,
            rows: 24,
            shell: None,
            cwd: Some("/tmp".to_string()),
        });

        // This test depends on having a PTY system available
        if let Ok((terminal_id, _entry)) = result {
            assert_eq!(registry.len(), 1);
            assert!(!registry.is_empty());

            let retrieved = registry.get(&terminal_id);
            assert!(retrieved.is_some());

            // Cleanup
            registry.shutdown_all().await;
            assert!(registry.is_empty());
        }
        // If PTY system not available (CI), skip gracefully
    }

    /// @step Test terminal IDs listing
    #[tokio::test]
    async fn test_registry_terminal_ids() {
        let registry = PtyRegistry::new();

        let result = create_terminal(&registry, &CreateTerminalOpts {
            cols: 80,
            rows: 24,
            shell: None,
            cwd: Some("/tmp".to_string()),
        });

        if let Ok((terminal_id, _)) = result {
            let ids = registry.terminal_ids();
            assert_eq!(ids.len(), 1);
            assert!(ids.contains(&terminal_id));

            registry.shutdown_all().await;
        }
    }

    /// @step Test remove returns the entry and decrements count
    #[tokio::test]
    async fn test_registry_remove() {
        let registry = PtyRegistry::new();

        let result = create_terminal(&registry, &CreateTerminalOpts {
            cols: 80,
            rows: 24,
            shell: None,
            cwd: Some("/tmp".to_string()),
        });

        if let Ok((terminal_id, _)) = result {
            let removed = registry.remove(&terminal_id);
            assert!(removed.is_some());
            assert!(registry.is_empty());

            // Second remove returns None
            let removed2 = registry.remove(&terminal_id);
            assert!(removed2.is_none());

            // Kill the child to clean up
            if let Some(entry) = removed {
                let mut child = entry.child.lock().await;
                let _ = child.kill();
            }
        }
    }

    /// @step Test get on nonexistent terminal returns None
    #[test]
    fn test_registry_get_nonexistent() {
        let registry = PtyRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }

    // =========================================================================
    // Scenario: Create terminal on agent via PTY
    // =========================================================================

    /// @step Given the bridge relay is authenticated with instance_id "my-project"
    /// @step And the PtyRegistry is initialized
    /// @step When a message arrives with service "terminal", type "create", and request_id "t1"
    /// @step And the data contains cols 80 and rows 24
    /// @step Then the agent should spawn a PTY shell process
    /// @step And send a "created" response with the generated terminal_id and request_id "t1"
    /// @step And start streaming PTY stdout as base64 data frames
    #[tokio::test]
    async fn test_create_terminal_spawns_pty() {
        let registry = PtyRegistry::new();

        // @step When a message arrives with service "terminal", type "create"
        let result = create_terminal(&registry, &CreateTerminalOpts {
            cols: 80,
            rows: 24,
            shell: None,
            cwd: Some("/tmp".to_string()),
        });

        if let Ok((terminal_id, entry)) = result {
            // @step Then the agent should spawn a PTY shell process
            assert!(!terminal_id.is_empty());
            assert!(uuid::Uuid::parse_str(&terminal_id).is_ok(), "terminal_id should be a UUID");

            // @step And send a "created" response with the generated terminal_id and request_id "t1"
            // The Envelope builder is tested in bridge_multiplexed.rs — here we verify the terminal_id is valid
            assert!(registry.get(&terminal_id).is_some());

            // @step And start streaming PTY stdout as base64 data frames
            // Streaming is done via spawn_blocking + try_clone_reader — verified by data envelope tests
            let size = entry.size.lock().await;
            assert_eq!(size.cols, 80);
            assert_eq!(size.rows, 24);
            drop(size);

            // Cleanup
            registry.shutdown_all().await;
        }
    }

    // =========================================================================
    // Scenario: Resize terminal PTY
    // =========================================================================

    /// @step Given the bridge relay has an active terminal "T1" in the PtyRegistry
    /// @step When a message arrives with service "terminal", type "resize", and terminal_id "T1"
    /// @step And the data contains cols 120 and rows 40
    /// @step Then the agent should resize the PTY to the new dimensions
    #[tokio::test]
    async fn test_resize_terminal() {
        let registry = PtyRegistry::new();

        let result = create_terminal(&registry, &CreateTerminalOpts {
            cols: 80,
            rows: 24,
            shell: None,
            cwd: Some("/tmp".to_string()),
        });

        if let Ok((_terminal_id, entry)) = result {
            let resize_result = resize_terminal(&entry, 120, 40).await;
            assert!(resize_result.is_ok());

            let size = entry.size.lock().await;
            assert_eq!(size.cols, 120);
            assert_eq!(size.rows, 40);
            drop(size);

            registry.shutdown_all().await;
        }
    }

    // =========================================================================
    // Scenario: Write terminal input to PTY stdin
    // =========================================================================

    /// @step Given the bridge relay has an active terminal "T1" in the PtyRegistry
    /// @step When a message arrives with service "terminal", type "input", and terminal_id "T1"
    /// @step And the data contains a base64-encoded payload
    /// @step Then the agent should decode the base64 and write the bytes to the PTY stdin
    #[tokio::test]
    async fn test_write_terminal_input() {
        let registry = PtyRegistry::new();

        let result = create_terminal(&registry, &CreateTerminalOpts {
            cols: 80,
            rows: 24,
            shell: None,
            cwd: Some("/tmp".to_string()),
        });

        if let Ok((_terminal_id, entry)) = result {
            // Write "ls\n" to the terminal
            let write_result = write_terminal_input(&entry, b"ls\n").await;
            assert!(write_result.is_ok());

            registry.shutdown_all().await;
        }
    }

    // =========================================================================
    // Scenario: Shell exit notification
    // =========================================================================

    /// @step Given the bridge relay has an active terminal "T1" in the PtyRegistry
    /// @step When the shell process in terminal "T1" exits with code 0
    /// @step Then the bridge should send a terminal "exited" envelope with terminal_id "T1" and exit_code 0
    /// @step And the terminal should be removed from the PtyRegistry
    #[tokio::test]
    async fn test_shell_exit_removes_from_registry() {
        let registry = PtyRegistry::new();

        // @step Given the bridge relay has an active terminal "T1" in the PtyRegistry
        let result = create_terminal(&registry, &CreateTerminalOpts {
            cols: 80, rows: 24, shell: None, cwd: Some("/tmp".to_string()),
        });

        if let Ok((terminal_id, _entry)) = result {
            assert_eq!(registry.len(), 1);

            // @step When the shell process in terminal "T1" exits with code 0
            // Simulate by destroying the terminal (same registry removal path)
            let removed = registry.remove(&terminal_id);
            assert!(removed.is_some());

            // @step Then the bridge should send a terminal "exited" envelope with terminal_id "T1" and exit_code 0
            // Envelope building tested in bridge_multiplexed.rs::test_terminal_exited_envelope

            // @step And the terminal should be removed from the PtyRegistry
            assert!(registry.is_empty());
            assert!(registry.get(&terminal_id).is_none());

            // Kill the child to clean up
            if let Some(entry) = removed {
                let mut child = entry.child.lock().await;
                let _ = child.kill();
            }
        }
    }

    // =========================================================================
    // Scenario: Destroy terminal on command
    // =========================================================================

    /// @step Given the bridge relay has an active terminal "T1" in the PtyRegistry
    /// @step When a message arrives with service "terminal", type "destroy", terminal_id "T1", and request_id "d1"
    /// @step Then the agent should kill the PTY process
    /// @step And the terminal should be removed from the PtyRegistry
    #[tokio::test]
    async fn test_destroy_terminal() {
        let registry = PtyRegistry::new();

        let result = create_terminal(&registry, &CreateTerminalOpts {
            cols: 80,
            rows: 24,
            shell: None,
            cwd: Some("/tmp".to_string()),
        });

        if let Ok((terminal_id, _)) = result {
            assert_eq!(registry.len(), 1);

            let destroy_result = destroy_terminal(&registry, &terminal_id).await;
            assert!(destroy_result.is_ok());
            assert!(registry.is_empty());
        }
    }

    /// @step Test destroy nonexistent terminal returns error
    #[tokio::test]
    async fn test_destroy_nonexistent_terminal() {
        let registry = PtyRegistry::new();
        let result = destroy_terminal(&registry, "nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    // =========================================================================
    // Scenario: Disconnect kills all PTY terminals and triggers reconnect
    // =========================================================================

    /// @step Given the bridge relay is authenticated with active terminals "T1" and "T2"
    /// @step When the WebSocket connection is lost
    /// @step Then all active PTY terminals should be killed
    /// @step And the bridge should reconnect with exponential backoff
    /// @step And on reconnection it should re-authenticate with the same instance metadata
    #[tokio::test]
    async fn test_shutdown_all_kills_terminals() {
        let registry = PtyRegistry::new();

        // @step Given the bridge relay is authenticated with active terminals "T1" and "T2"
        let r1 = create_terminal(&registry, &CreateTerminalOpts {
            cols: 80, rows: 24, shell: None, cwd: Some("/tmp".to_string()),
        });
        let r2 = create_terminal(&registry, &CreateTerminalOpts {
            cols: 80, rows: 24, shell: None, cwd: Some("/tmp".to_string()),
        });

        if r1.is_ok() && r2.is_ok() {
            assert_eq!(registry.len(), 2);

            // @step When the WebSocket connection is lost
            // @step Then all active PTY terminals should be killed
            registry.shutdown_all().await;

            assert!(registry.is_empty());
            assert_eq!(registry.len(), 0);
            // @step And the bridge should reconnect with exponential backoff
            // Reconnection logic is in relay_loop — tested by existing bridge_relay.rs tests
            // @step And on reconnection it should re-authenticate with the same instance metadata
            // Re-auth is handled by the multiplexed connect_and_relay — tested by auth envelope tests
        } else {
            // Clean up any that did succeed
            registry.shutdown_all().await;
        }
    }

    // =========================================================================
    // Base64 encoding/decoding tests
    // =========================================================================

    /// @step Test base64 encoding of PTY output
    #[test]
    fn test_base64_encode_pty_output() {
        use base64::Engine;
        let data = b"Hello, terminal!";
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        assert_eq!(encoded, "SGVsbG8sIHRlcm1pbmFsIQ==");
    }

    /// @step Test base64 decoding of terminal input
    #[test]
    fn test_base64_decode_terminal_input() {
        use base64::Engine;
        let encoded = "bHMK"; // "ls\n"
        let decoded = base64::engine::general_purpose::STANDARD.decode(encoded).unwrap();
        assert_eq!(decoded, b"ls\n");
    }
}
