//! Feature: spec/features/approve-for-session-covers-only-the-session-that-asked.feature
//!
//! BUG-606: "Approve for session" was one daemon-wide set of allowances, so
//! approving a rule in one session allowed it in every session. It covers
//! only the session that asked (product owner, 2026-11-07, decision 7).
//!
//! Each session answers its approvals through a pause handler that records
//! how often it was asked and chooses Approve for session. The blocklist is
//! the default template, installed into a temporary HOME; the RLCD workflow
//! gate talks to the in-process RLCD mock.

#[path = "../../tools/tests/rlcd_mock.rs"]
mod rlcd_mock;

use codelet_sessions::SessionManager;
use codelet_tools::blocklist::{check_bash_command, check_file_path, clear_session_allowances};
use codelet_tools::rlcd::{run_rlcd_gate, GateVerdict};
use codelet_tools::tool_pause::{set_pause_handler, PauseResponse};
use rlcd_mock::MockRlcd;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use uuid::Uuid;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Matches the default blocklist's `git-hard-reset-prompt` rule; never run.
const PROMPTED_COMMAND: &str = "echo bug606 git reset --hard";

/// Restores the environment variables a scenario changed.
struct Environment(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl Environment {
    fn set(values: &[(&'static str, std::ffi::OsString)]) -> Self {
        let previous = values
            .iter()
            .map(|(name, _)| (*name, std::env::var_os(name)))
            .collect();
        for (name, value) in values {
            std::env::set_var(name, value);
        }
        Self(previous)
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        for (name, value) in &self.0 {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
        clear_session_allowances();
    }
}

/// A temporary HOME (default blocklist) and an fspec user dir whose RLCD
/// section is `rlcd`.
fn environment(rlcd: serde_json::Value) -> TestResult<(Environment, Vec<tempfile::TempDir>)> {
    // BUG-447: these sessions run no agent loop and register no isolation.
    codelet_tools::facade::declare_unrestricted_tool_access(
        "bug606 checks approvals without a session manager callback",
    );
    let home = codelet_test_helpers::setup_temp_directory()?;
    let user = codelet_test_helpers::setup_temp_directory()?;
    std::fs::write(
        user.path().join("fspec-config.json"),
        serde_json::json!({ "rlcd": rlcd }).to_string(),
    )?;
    let environment = Environment::set(&[
        ("HOME", home.path().as_os_str().to_owned()),
        ("FSPEC_USER_DIR", user.path().as_os_str().to_owned()),
    ]);
    clear_session_allowances();
    Ok((environment, vec![home, user]))
}

/// A session's approvals: every ask is counted and answered Approve for session.
struct Asker {
    session: Uuid,
    asked: Arc<AtomicUsize>,
}

impl Asker {
    fn new(session: Uuid) -> Self {
        let asked = Arc::new(AtomicUsize::new(0));
        let counter = asked.clone();
        set_pause_handler(
            session,
            Some(Arc::new(move |_| {
                counter.fetch_add(1, Ordering::SeqCst);
                PauseResponse::AllowSession
            })),
        );
        Self { session, asked }
    }

    fn asked(&self) -> usize {
        self.asked.load(Ordering::SeqCst)
    }

    fn run(&self) -> TestResult {
        check_bash_command(PROMPTED_COMMAND, self.session).map_err(|error| error.to_string())?;
        Ok(())
    }

    fn read_env(&self, project: &std::path::Path) -> TestResult {
        let path = project.join(".env");
        check_file_path(&path.to_string_lossy(), self.session)
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

impl Drop for Asker {
    fn drop(&mut self) {
        set_pause_handler(self.session, None);
    }
}

/// A session manager over a fresh data directory, and its project.
async fn manager() -> TestResult<(Arc<SessionManager>, tempfile::TempDir, tempfile::TempDir)> {
    let data = codelet_test_helpers::setup_temp_directory()?;
    std::fs::create_dir_all(data.path().join("cache"))?;
    std::fs::write(
        data.path().join("cache/models.json"),
        include_str!("fixtures/rpc422_models.json"),
    )?;
    codelet_core::persistence::reset_stores_for_tests();
    codelet_common::set_data_directory(data.path().to_owned())?;
    std::env::set_var("ANTHROPIC_API_KEY", "offline");
    let project = codelet_test_helpers::setup_temp_directory()?;
    Ok((SessionManager::new(), data, project))
}

async fn session(manager: &SessionManager, project: &std::path::Path) -> TestResult<Uuid> {
    let id = Uuid::new_v4();
    manager
        .create_session_with_id(
            &id.to_string(),
            "anthropic/claude-sonnet-4",
            &project.to_string_lossy(),
            "bug606",
        )
        .await?;
    Ok(id)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn approve_for_session_on_a_command_prompt_covers_only_the_session_that_asked() -> TestResult
{
    // @step Given sessions "A" and "B" in one daemon, where "A" ran "echo bug606 git reset --hard", was asked for approval and chose Approve for session
    let (_environment, _directories) = environment(serde_json::json!({"enabled": false}))?;
    let a = Asker::new(Uuid::new_v4());
    let b = Asker::new(Uuid::new_v4());
    a.run()?;
    assert_eq!(a.asked(), 1);

    // @step When "A" runs "echo bug606 git reset --hard" again
    a.run()?;

    // @step Then "A" is not asked
    assert_eq!(a.asked(), 1);

    // @step When "B" runs "echo bug606 git reset --hard"
    b.run()?;

    // @step Then "B" is asked for approval
    assert_eq!(b.asked(), 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn approve_for_session_on_a_sensitive_file_covers_only_the_sub_agent_that_asked() -> TestResult
{
    // @step Given a bot "P" with sub-agents "S1" and "S2", where "S1" read ".env", was asked for approval and chose Approve for session
    let (_environment, _directories) = environment(serde_json::json!({"enabled": false}))?;
    let (manager, _data, project) = manager().await?;
    let bot = session(&manager, project.path()).await?;
    let first = session(&manager, project.path()).await?;
    let second = session(&manager, project.path()).await?;
    manager.add_supervisor(first, bot)?;
    manager.add_supervisor(second, bot)?;
    let (p, s1, s2) = (Asker::new(bot), Asker::new(first), Asker::new(second));
    s1.read_env(project.path())?;
    assert_eq!(s1.asked(), 1);

    // @step When "P" and "S2" read ".env"
    p.read_env(project.path())?;
    s2.read_env(project.path())?;

    // @step Then each of them is asked for approval
    assert_eq!((p.asked(), s2.asked()), (1, 1));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn approve_for_session_on_the_rlcd_workflow_gate_covers_only_the_session_that_asked(
) -> TestResult {
    // @step Given sessions "A" and "B" whose update-work-unit-status is held by the RLCD workflow gate, where "A" was asked and chose Approve for session
    let mock = MockRlcd::ready("gate-model-x").start().await;
    mock.set_answers(
        r#"{"model":"gate-model-x","answers":{"gate":{"type":"choice","choice":"ask_user","confidence":0.5,"probabilities":{"proceed":0.4,"hold":0.1,"ask_user":0.5}}},"usage":{"input_tokens":12,"output_tokens":0}}"#,
    );
    let (_environment, directories) = environment(serde_json::json!({"url": mock.base_url()}))?;
    let project = directories[0].path().to_string_lossy().into_owned();
    let args = serde_json::json!({"workUnitId": "AUTH-005", "status": "validating"}).to_string();
    let gate = |session: Uuid| run_rlcd_gate(session, "update-work-unit-status", &args, &project);
    let a = Asker::new(Uuid::new_v4());
    let b = Asker::new(Uuid::new_v4());
    assert_eq!(gate(a.session).await, GateVerdict::Execute);
    assert_eq!(a.asked(), 1);

    // @step When "A" runs update-work-unit-status again
    let again = gate(a.session).await;

    // @step Then "A" is not asked
    assert_eq!((again, a.asked()), (GateVerdict::Execute, 1));

    // @step When "B" runs update-work-unit-status
    let other = gate(b.session).await;

    // @step Then "B" is asked for approval
    assert_eq!((other, b.asked()), (GateVerdict::Execute, 1));
    mock.stop().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn closing_a_session_forgets_its_session_approvals() -> TestResult {
    // @step Given session "A" approved "echo bug606 git reset --hard" for the session
    let (_environment, _directories) = environment(serde_json::json!({"enabled": false}))?;
    let (manager, _data, project) = manager().await?;
    let id = session(&manager, project.path()).await?;
    let a = Asker::new(id);
    a.run()?;
    assert_eq!(a.asked(), 1);

    // @step When "A" is closed
    manager.destroy_session(&id.to_string())?;

    // @step Then a command run under "A"'s id is asked for approval again
    let after = Asker::new(id);
    after.run()?;
    assert_eq!(after.asked(), 1);
    Ok(())
}
