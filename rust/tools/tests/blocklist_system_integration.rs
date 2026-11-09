//! BLOCK-001 — the blocklist system end to end (BUG-628).
//!
//! Feature: spec/features/blocklist-system-integration-tests.feature
//!
//! Each test drives the real Write or Read tool for one session, the way an
//! agent turn does: the stage permissions (BLOCK-006) and the sensitive path
//! prompt with its session memory (BLOCK-005, BLOCK-007) run inside the
//! tool call.
//!
//! The system configs are read from `$HOME/.fspec`, so every test points
//! `HOME` at a fresh temp dir holding an empty system blocklist (the default
//! template is never installed) and no system stage permissions (the
//! default ACDD permissions apply), and isolates the user directory so RLCD
//! is disabled. The blocklist root, the session allowances and the HOME
//! variable are process-global, so the tests are `#[serial]`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use codelet_test_helpers::{OrFail, TestResult};
use codelet_tools::blocklist::{
    clear_session_allowances, init_blocklist, is_session_allowed, BlocklistAction, BlocklistConfig,
    BlocklistRule,
};
use codelet_tools::facade::{
    declare_unrestricted_tool_access, set_block_notification_callback,
    set_get_work_unit_stage_callback,
};
use codelet_tools::read::ReadArgs;
use codelet_tools::stage_gate::SessionStage;
use codelet_tools::tool_pause::{
    set_pause_handler, PauseHandler, PauseKind, PauseRequest, PauseResponse,
};
use codelet_tools::write::WriteArgs;
use codelet_tools::{ReadTool, ToolError, WriteTool};
use rig::tool::Tool;
use serial_test::serial;
use uuid::Uuid;

/// The session every test's tool calls run in.
const SESSION: Uuid = Uuid::from_u128(0x0628_0000_0000_0000_0000_0000_0000_0001);

/// The stage the work-unit stage callback reports for [`SESSION`].
static STAGE: Mutex<Option<SessionStage>> = Mutex::new(None);

/// Every block notification emitted: (session id, action, reason).
static NOTIFICATIONS: Mutex<Vec<(String, String, String)>> = Mutex::new(Vec::new());

fn stage_of(session_id: String) -> Result<Option<SessionStage>, String> {
    if session_id != SESSION.to_string() {
        return Err(format!("unknown session {session_id}"));
    }
    STAGE
        .lock()
        .map(|stage| stage.clone())
        .map_err(|error| error.to_string())
}

fn record_notification(session_id: String, action: String, reason: String) {
    if let Ok(mut notifications) = NOTIFICATIONS.lock() {
        notifications.push((session_id, action, reason));
    }
}

fn set_stage(stage: Option<SessionStage>) -> TestResult {
    *STAGE.lock().map_err(|error| error.to_string())? = stage;
    Ok(())
}

fn notifications() -> Result<Vec<(String, String, String)>, String> {
    NOTIFICATIONS
        .lock()
        .map(|notifications| notifications.clone())
        .map_err(|error| error.to_string())
}

/// A temp HOME with an empty system blocklist, an isolated user directory
/// and this process's tool callbacks. On drop it restores HOME and resets
/// the session's blocklist, pause handler, allowances and stage.
struct Environment {
    prior_home: Option<std::ffi::OsString>,
    home: tempfile::TempDir,
    project: tempfile::TempDir,
}

impl Environment {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        codelet_test_helpers::isolated_user_directory()?;
        declare_unrestricted_tool_access("blocklist system integration tests drive tools directly");
        set_get_work_unit_stage_callback(stage_of);
        set_block_notification_callback(record_notification);
        let home = tempfile::tempdir()?;
        let empty = BlocklistConfig {
            version: "1.0.0".to_string(),
            rules: Vec::new(),
        };
        std::fs::create_dir_all(home.path().join(".fspec"))?;
        std::fs::write(
            home.path().join(".fspec").join("blocklist.json"),
            serde_json::to_string_pretty(&empty)?,
        )?;
        let prior_home = std::env::var_os("HOME");
        std::env::set_var("HOME", home.path());
        let project = tempfile::tempdir()?;
        NOTIFICATIONS
            .lock()
            .map_err(|error| error.to_string())?
            .clear();
        clear_session_allowances();
        init_blocklist(Some(project.path()));
        Ok(Self {
            prior_home,
            home,
            project,
        })
    }

    fn home(&self) -> &Path {
        self.home.path()
    }

    fn project(&self) -> PathBuf {
        self.project.path().to_path_buf()
    }

    /// Write `<project>/.fspec/blocklist.json` holding `rules`.
    fn project_blocklist(&self, rules: Vec<BlocklistRule>) -> TestResult {
        let dir = self.project.path().join(".fspec");
        std::fs::create_dir_all(&dir)?;
        let config = BlocklistConfig {
            version: "1.0.0".to_string(),
            rules,
        };
        std::fs::write(
            dir.join("blocklist.json"),
            serde_json::to_string_pretty(&config)?,
        )?;
        Ok(())
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        set_pause_handler(SESSION, None);
        clear_session_allowances();
        init_blocklist(None);
        if let Ok(mut stage) = STAGE.lock() {
            *stage = None;
        }
        match self.prior_home.take() {
            Some(home) => std::env::set_var("HOME", home),
            None => std::env::remove_var("HOME"),
        }
    }
}

async fn write(path: &Path) -> Result<String, ToolError> {
    WriteTool::new(SESSION)
        .call(WriteArgs {
            file_path: path.to_string_lossy().to_string(),
            content: "export const auth = true;\n".to_string(),
        })
        .await
}

async fn read(path: &Path) -> Result<String, ToolError> {
    ReadTool::new(SESSION)
        .call(ReadArgs {
            file_path: path.to_string_lossy().to_string(),
            offset: None,
            limit: None,
            pdf_mode: None,
        })
        .await
}

/// Scenario: Stage permissions block and allow based on work unit state
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn stage_permissions_block_and_allow_based_on_work_unit_state() -> TestResult {
    let environment = Environment::new()?;
    let project = environment.project();
    let impl_file = project.join("src").join("auth.ts");
    let test_file = project.join("src").join("__tests__").join("auth.test.ts");

    // @step Given a work unit is in "testing" stage
    set_stage(Some(SessionStage {
        stage: "testing".to_string(),
        project_root: project.clone(),
    }))?;

    // @step When the AI tries to write to "src/auth.ts"
    let refused = write(&impl_file).await;

    // @step Then the write should be blocked by stage permissions
    let message = "Blocked: Cannot write impl files in testing stage. Write tests first, then \
                   move to implementing stage. Use: fspec update-work-unit-status <workUnitId> implementing";
    match &refused {
        Err(ToolError::Blocked { tool, message: got }) => {
            assert_eq!(*tool, "Write");
            assert_eq!(got, message);
        }
        other => return Err(format!("expected a stage block, got {other:?}").into()),
    }
    assert!(!impl_file.exists(), "a refused write writes nothing");

    // @step And a block notification is emitted for the session
    assert_eq!(
        notifications()?,
        vec![(
            SESSION.to_string(),
            format!("Write {}", impl_file.display()),
            message.to_string(),
        )]
    );

    // @step When the AI tries to write to "src/__tests__/auth.test.ts"
    let written = write(&test_file).await;

    // @step Then the write should be allowed
    assert_eq!(
        written?,
        format!("Successfully wrote to {}", test_file.display())
    );
    assert!(test_file.exists());

    // @step When the work unit is moved to "implementing" stage
    set_stage(Some(SessionStage {
        stage: "implementing".to_string(),
        project_root: project.clone(),
    }))?;

    // @step And the AI tries to write to "src/auth.ts"
    let written = write(&impl_file).await;

    // @step Then the write should now be allowed
    assert_eq!(
        written?,
        format!("Successfully wrote to {}", impl_file.display())
    );
    assert_eq!(notifications()?.len(), 1, "no further block notification");
    Ok(())
}

/// Scenario: Sensitive path prompts with session memory
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn sensitive_path_prompts_with_session_memory() -> TestResult {
    let environment = Environment::new()?;
    let ssh = environment.home().join(".ssh");
    std::fs::create_dir_all(&ssh)?;
    for name in ["config", "known_hosts", "id_rsa"] {
        std::fs::write(ssh.join(name), format!("{name} contents\n"))?;
    }
    let pauses: Arc<Mutex<Vec<PauseRequest>>> = Arc::new(Mutex::new(Vec::new()));
    let answer = Arc::new(Mutex::new(PauseResponse::AllowSession));
    let handler: PauseHandler = {
        let pauses = Arc::clone(&pauses);
        let answer = Arc::clone(&answer);
        Arc::new(move |request| {
            if let Ok(mut pauses) = pauses.lock() {
                pauses.push(request);
            }
            answer
                .lock()
                .map(|answer| *answer)
                .unwrap_or(PauseResponse::Denied)
        })
    };
    let pause_count = || pauses.lock().map(|pauses| pauses.len()).unwrap_or(0);

    // @step Given a blocklist rule exists prompting for "~/.ssh" access
    environment.project_blocklist(vec![BlocklistRule {
        id: "ssh-prompt".to_string(),
        pattern: r"/\.ssh/".to_string(),
        action: BlocklistAction::Prompt,
        reason: "SSH directory contains private keys".to_string(),
        guidance: None,
    }])?;
    set_pause_handler(SESSION, Some(handler.clone()));

    // @step When the AI tries to read "~/.ssh/config"
    let config = read(&ssh.join("config")).await;

    // @step Then the read pauses for an Allow Once, Allow Session or Deny choice
    {
        let pauses = pauses.lock().map_err(|error| error.to_string())?;
        let pause = pauses.first().or_fail("the read pauses")?;
        assert_eq!(pause.kind, PauseKind::Triple);
        assert_eq!(pause.tool_name, "Read");
        assert_eq!(pause.message, "SSH directory contains private keys");
        assert_eq!(
            pause.details.as_deref(),
            Some(ssh.join("config").to_string_lossy().as_ref())
        );
    }

    // @step When the user selects "Allow Session"
    assert!(
        is_session_allowed(SESSION, "ssh-prompt"),
        "the Allow Session answer is kept for the session"
    );

    // @step Then the file should be read successfully
    assert!(config?.contains("config contents"));

    // @step When the AI tries to read "~/.ssh/known_hosts"
    let known_hosts = read(&ssh.join("known_hosts")).await;

    // @step Then no prompt should appear due to session allowance
    assert!(known_hosts?.contains("known_hosts contents"));
    assert_eq!(pause_count(), 1, "the session allowance skips the pause");

    // @step When the TUI is restarted
    clear_session_allowances();
    init_blocklist(Some(&environment.project()));
    set_pause_handler(SESSION, Some(handler));
    *answer.lock().map_err(|error| error.to_string())? = PauseResponse::AllowOnce;

    // @step And the AI tries to read "~/.ssh/id_rsa"
    let id_rsa = read(&ssh.join("id_rsa")).await;

    // @step Then a prompt should appear again
    assert_eq!(pause_count(), 2, "the restart forgot the allowance");
    assert!(id_rsa?.contains("id_rsa contents"));
    Ok(())
}
