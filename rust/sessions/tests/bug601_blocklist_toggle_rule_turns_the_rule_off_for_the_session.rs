//! Feature: spec/features/blocklist-toggle-rule-turns-the-rule-off-for-the-session.feature
//!
//! BUG-601: Toggle Rule in the blocklist view only changed how the rule was
//! shown; the rule stayed enforced. A rule turned off is off only for the
//! session the view was opened from, and it is saved with that session, so it
//! stays off after a daemon restart and resume (product owner, 2026-11-08).
//!
//! The daemon is a real SessionManager over a fresh data directory. Its
//! system blocklist is written into a temporary user directory; commands and file reads
//! are checked by the same codelet-tools entry points the Bash and Read tools
//! call, under each session's id. Nothing is ever run.

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use codelet_tools::blocklist::{
    check_bash_command, check_file_path_semantic, clear_session_allowances,
    clear_session_disabled_rules,
};
use codelet_tools::tool_pause::{set_pause_handler, PauseResponse};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use uuid::Uuid;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CHECKOUT: &str = "git checkout main";
const CHECKOUT_BLOCKED: &str =
    "Blocked: git checkout can overwrite uncommitted changes. Use git switch <branch>.";
const HARD_RESET: &str = "echo bug601 git reset --hard";
const UNKNOWN_SESSION: &str = "00000000-0000-0000-0000-000000000601";

/// The system blocklist every scenario's daemon loads.
fn system_blocklist() -> serde_json::Value {
    serde_json::json!({
        "version": "1.0.0",
        "rules": [
            {
                "id": "git-checkout-block",
                "pattern": "^git\\s+checkout\\b",
                "action": "block",
                "reason": "git checkout can overwrite uncommitted changes.",
                "guidance": "Use git switch <branch>."
            },
            {
                "id": "git-hard-reset-prompt",
                "pattern": "git\\s+reset\\s+--hard",
                "action": "prompt",
                "reason": "Hard reset discards all uncommitted changes permanently"
            },
            {
                "id": "env-file-prompt",
                "pattern": "\\.env$",
                "action": "prompt",
                "reason": "Environment files often contain secrets and API keys"
            }
        ]
    })
}

/// Restores the environment a scenario changed and forgets every session's
/// turned-off rules and allowances.
struct Environment(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl Drop for Environment {
    fn drop(&mut self) {
        for (name, value) in &self.0 {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
        clear_session_allowances();
        clear_session_disabled_rules();
        codelet_core::persistence::reset_stores_for_tests();
    }
}

/// A daemon: a session manager whose start workspace is `project`, over a
/// fresh data directory, with the test blocklist in a temporary user
/// directory (FSPEC_USER_DIR) and a temporary HOME.
struct Daemon {
    manager: Arc<SessionManager>,
    project: tempfile::TempDir,
    _directories: Vec<tempfile::TempDir>,
    _environment: Environment,
}

impl Daemon {
    async fn start() -> TestResult<Self> {
        // The sessions run no agent loop and register no isolation.
        codelet_tools::facade::declare_unrestricted_tool_access(
            "bug601 checks the blocklist without a session manager callback",
        );
        let home = codelet_test_helpers::setup_temp_directory()?;
        let user = codelet_test_helpers::setup_temp_directory()?;
        let data = codelet_test_helpers::setup_temp_directory()?;
        let project = codelet_test_helpers::setup_temp_directory()?;
        // BUG-695: the system blocklist lives in the user directory, which
        // resolves through FSPEC_USER_DIR (BUG-640), not under HOME.
        std::fs::write(
            user.path().join("blocklist.json"),
            system_blocklist().to_string(),
        )?;
        std::fs::write(
            user.path().join("fspec-config.json"),
            serde_json::json!({ "rlcd": { "enabled": false } }).to_string(),
        )?;
        std::fs::create_dir_all(data.path().join("cache"))?;
        std::fs::write(
            data.path().join("cache/models.json"),
            include_str!("fixtures/rpc422_models.json"),
        )?;
        let names = ["HOME", "FSPEC_USER_DIR", "ANTHROPIC_API_KEY"];
        let environment = Environment(
            names
                .iter()
                .map(|name| (*name, std::env::var_os(name)))
                .collect(),
        );
        std::env::set_var("HOME", home.path());
        std::env::set_var("FSPEC_USER_DIR", user.path());
        std::env::set_var("ANTHROPIC_API_KEY", "offline");
        clear_session_allowances();
        clear_session_disabled_rules();
        codelet_core::persistence::reset_stores_for_tests();
        codelet_common::set_data_directory(data.path().to_owned())?;
        let manager = SessionManager::new();
        manager.set_default_project(project.path().to_owned())?;
        Ok(Self {
            manager,
            project,
            _directories: vec![home, user, data],
            _environment: environment,
        })
    }

    async fn session(&self) -> TestResult<SessionId> {
        let id = Uuid::new_v4().to_string();
        self.manager
            .create_session_with_id(
                &id,
                "anthropic/claude-sonnet-4",
                &self.project.path().to_string_lossy(),
                "bug601",
            )
            .await?;
        Ok(SessionId::new(id))
    }

    /// The daemon process ends (every in-memory state with it) and a new
    /// one starts over the same data directory.
    fn restart(&mut self) -> TestResult {
        clear_session_allowances();
        clear_session_disabled_rules();
        codelet_core::persistence::reset_stores_for_tests();
        self.manager = SessionManager::new();
        self.manager
            .set_default_project(self.project.path().to_owned())?;
        Ok(())
    }

    fn turn(&self, session: &SessionId, rule: &str, enabled: bool) -> Result<Vec<String>, String> {
        self.manager
            .blocklist_set_rule_enabled(session, rule, enabled)
    }

    fn turned_off(&self, session: &SessionId) -> Result<Vec<String>, String> {
        self.manager.blocklist_disabled_rules(session)
    }
}

fn uuid(session: &SessionId) -> TestResult<Uuid> {
    Ok(Uuid::parse_str(&session.value)?)
}

/// The outcome of checking `command` under `session`: `Ok` when allowed, the
/// error the agent would see when not.
fn run(session: &SessionId, command: &str) -> TestResult<Result<(), String>> {
    Ok(check_bash_command(command, uuid(session)?).map_err(|error| error.to_string()))
}

/// Counts how often `session` is asked; every ask is denied.
struct Asked {
    session: Uuid,
    count: Arc<AtomicUsize>,
}

impl Asked {
    fn new(session: &SessionId) -> TestResult<Self> {
        let session = uuid(session)?;
        let count = Arc::new(AtomicUsize::new(0));
        let counter = count.clone();
        set_pause_handler(
            session,
            Some(Arc::new(move |_| {
                counter.fetch_add(1, Ordering::SeqCst);
                PauseResponse::Denied
            })),
        );
        Ok(Self { session, count })
    }

    fn count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}

impl Drop for Asked {
    fn drop(&mut self) {
        set_pause_handler(self.session, None);
    }
}

fn read_env(session: &SessionId, project: &Path) -> TestResult<Result<(), String>> {
    let path = project.join(".env").to_string_lossy().into_owned();
    let check = check_file_path_semantic(&path, uuid(session)?, "Read", "read");
    let outcome = tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(check));
    Ok(outcome.map_err(|error| error.to_string()))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn turning_a_rule_off_lets_the_session_run_what_it_blocked_and_turning_it_on_blocks_it_again(
) -> TestResult {
    // @step Given session "s-1" in a daemon whose blocklist has rule "git-checkout-block" blocking "git checkout"
    let daemon = Daemon::start().await?;
    let s1 = daemon.session().await?;

    // @step And "git checkout main" run under "s-1" is blocked with "Blocked: git checkout can overwrite uncommitted changes. Use git switch <branch>."
    assert_eq!(run(&s1, CHECKOUT)?, Err(CHECKOUT_BLOCKED.to_string()));

    // @step When the user turns "git-checkout-block" off for "s-1"
    let answer = daemon.turn(&s1, "git-checkout-block", false)?;

    // @step Then the daemon answers that "s-1" has turned off "git-checkout-block"
    assert_eq!(answer, vec!["git-checkout-block".to_string()]);
    assert_eq!(daemon.turned_off(&s1)?, answer);

    // @step And "git checkout main" run under "s-1" is allowed
    assert_eq!(run(&s1, CHECKOUT)?, Ok(()));

    // @step When the user turns "git-checkout-block" on for "s-1"
    let answer = daemon.turn(&s1, "git-checkout-block", true)?;

    // @step Then the daemon answers that "s-1" has no rule turned off
    assert_eq!(answer, Vec::<String>::new());
    assert_eq!(daemon.turned_off(&s1)?, Vec::<String>::new());

    // @step And "git checkout main" run under "s-1" is blocked again
    assert_eq!(run(&s1, CHECKOUT)?, Err(CHECKOUT_BLOCKED.to_string()));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn a_turned_off_rule_that_asks_no_longer_asks() -> TestResult {
    // @step Given session "s-1" in a daemon whose blocklist asks before "git reset --hard" (rule "git-hard-reset-prompt") and before reading ".env" (rule "env-file-prompt")
    let daemon = Daemon::start().await?;
    let s1 = daemon.session().await?;
    let asked = Asked::new(&s1)?;
    assert_eq!(
        run(&s1, HARD_RESET)?,
        Err("Blocked: User denied access".to_string())
    );
    assert_eq!(
        read_env(&s1, daemon.project.path())?,
        Err("Blocked: User denied access".to_string())
    );
    assert_eq!(asked.count(), 2);

    // @step When the user turns "git-hard-reset-prompt" and "env-file-prompt" off for "s-1"
    daemon.turn(&s1, "git-hard-reset-prompt", false)?;
    let answer = daemon.turn(&s1, "env-file-prompt", false)?;
    assert_eq!(
        answer,
        vec![
            "env-file-prompt".to_string(),
            "git-hard-reset-prompt".to_string()
        ]
    );

    // @step And "s-1" runs "echo bug601 git reset --hard" and reads ".env"
    let command = run(&s1, HARD_RESET)?;
    let file = read_env(&s1, daemon.project.path())?;

    // @step Then both are allowed and "s-1" is not asked
    assert_eq!((command, file), (Ok(()), Ok(())));
    assert_eq!(asked.count(), 2);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn a_rule_turned_off_in_one_session_is_still_enforced_in_every_other_session() -> TestResult {
    // @step Given sessions "s-1" and "s-2" in one daemon whose blocklist has rule "git-checkout-block" blocking "git checkout"
    let daemon = Daemon::start().await?;
    let s1 = daemon.session().await?;
    let s2 = daemon.session().await?;

    // @step When the user turns "git-checkout-block" off for "s-1"
    daemon.turn(&s1, "git-checkout-block", false)?;
    assert_eq!(run(&s1, CHECKOUT)?, Ok(()));

    // @step Then "git checkout main" run under "s-2" is still blocked
    assert_eq!(run(&s2, CHECKOUT)?, Err(CHECKOUT_BLOCKED.to_string()));

    // @step And the daemon answers that "s-2" has no rule turned off
    assert_eq!(daemon.turned_off(&s2)?, Vec::<String>::new());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn turned_off_rules_stay_off_after_a_daemon_restart_and_resume() -> TestResult {
    // @step Given session "s-1" turned off "git-checkout-block"
    let mut daemon = Daemon::start().await?;
    let s1 = daemon.session().await?;
    daemon.turn(&s1, "git-checkout-block", false)?;

    // @step When the daemon restarts and "s-1" is resumed
    daemon.restart()?;
    // Nothing of the old process is left: the rule is enforced until the
    // session is back.
    assert_eq!(run(&s1, CHECKOUT)?, Err(CHECKOUT_BLOCKED.to_string()));
    daemon.manager.resume_session(&s1)?;

    // @step Then the daemon answers that "s-1" has turned off "git-checkout-block"
    assert_eq!(
        daemon.turned_off(&s1)?,
        vec!["git-checkout-block".to_string()]
    );

    // @step And "git checkout main" run under "s-1" is allowed
    assert_eq!(run(&s1, CHECKOUT)?, Ok(()));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn turning_off_a_rule_the_blocklist_does_not_have_fails_and_changes_nothing() -> TestResult {
    // @step Given session "s-1" with no rule turned off
    let daemon = Daemon::start().await?;
    let s1 = daemon.session().await?;

    // @step When the user turns "no-such-rule" off for "s-1"
    let answer = daemon.turn(&s1, "no-such-rule", false);

    // @step Then the daemon answers with the error 'the blocklist has no rule "no-such-rule"'
    assert_eq!(
        answer,
        Err("the blocklist has no rule \"no-such-rule\"".to_string())
    );

    // @step And the daemon answers that "s-1" has no rule turned off
    assert_eq!(daemon.turned_off(&s1)?, Vec::<String>::new());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn turning_a_rule_off_for_a_session_the_daemon_does_not_hold_fails() -> TestResult {
    // @step Given a daemon whose blocklist has rule "git-checkout-block" and no session "00000000-0000-0000-0000-000000000601"
    let daemon = Daemon::start().await?;
    let unknown = SessionId::new(UNKNOWN_SESSION.to_string());

    // @step When the user turns "git-checkout-block" off for "00000000-0000-0000-0000-000000000601"
    let answer = daemon.turn(&unknown, "git-checkout-block", false);

    // @step Then the daemon answers with the error "Session not found: 00000000-0000-0000-0000-000000000601"
    let expected = format!("Session not found: {UNKNOWN_SESSION}");
    assert_eq!(answer, Err(expected.clone()));

    // @step And reading the turned-off rules of "00000000-0000-0000-0000-000000000601" fails with the same error
    assert_eq!(daemon.turned_off(&unknown), Err(expected));
    assert_eq!(run(&unknown, CHECKOUT)?, Err(CHECKOUT_BLOCKED.to_string()));
    Ok(())
}

/// The daemon's RPC service over `daemon`'s session manager, in process.
fn rpc(daemon: &Daemon) -> TestResult<codelet_rpc::FspecServiceClient> {
    std::fs::create_dir_all(daemon.project.path().join("spec"))?;
    std::fs::write(
        daemon.project.path().join("spec/work-units.json"),
        r#"{"workUnits":{}}"#,
    )?;
    let watcher = Arc::new(codelet_core::work_units::WorkUnitsWatcher::new(
        daemon.project.path(),
    )?);
    let handle: Arc<dyn SessionManagerHandle> = daemon.manager.clone();
    let service = Arc::new(codelet_rpc::SharedFspecService::with_session_manager(
        watcher, handle,
    ));
    let transport =
        codelet_rpc_embedded::EmbeddedTransport::new(tokio::runtime::Handle::current(), service);
    Ok(transport.client())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn a_rule_turned_off_through_the_daemon_is_enforced_only_in_that_session_and_stays_off_after_a_daemon_restart(
) -> TestResult {
    // @step Given sessions "A" and "B" in a daemon whose blocklist has rule "git-checkout-block" blocking "git checkout"
    let mut daemon = Daemon::start().await?;
    let a = daemon.session().await?;
    let b = daemon.session().await?;
    let client = rpc(&daemon)?;

    // @step When the user turns "git-checkout-block" off for "A" through the daemon's RPC
    let answer = client
        .blocklist_set_rule_enabled(
            tarpc::context::current(),
            a.clone(),
            "git-checkout-block".to_string(),
            false,
        )
        .await??;
    assert_eq!(answer, vec!["git-checkout-block".to_string()]);

    // @step Then "git checkout main" run under "A" is allowed
    assert_eq!(run(&a, CHECKOUT)?, Ok(()));

    // @step And "git checkout main" run under "B" is still blocked
    assert_eq!(run(&b, CHECKOUT)?, Err(CHECKOUT_BLOCKED.to_string()));

    // @step When the daemon restarts and "A" is resumed through the daemon's RPC
    drop(client);
    daemon.restart()?;
    assert_eq!(run(&a, CHECKOUT)?, Err(CHECKOUT_BLOCKED.to_string()));
    let client = rpc(&daemon)?;
    client
        .resume_session(tarpc::context::current(), a.clone())
        .await??;

    // @step Then the daemon's RPC answers that "A" has turned off "git-checkout-block"
    let turned_off = client
        .blocklist_disabled_rules(tarpc::context::current(), a.clone())
        .await??;
    assert_eq!(turned_off, vec!["git-checkout-block".to_string()]);

    // @step And "git checkout main" run under "A" is allowed
    assert_eq!(run(&a, CHECKOUT)?, Ok(()));
    assert_eq!(run(&b, CHECKOUT)?, Err(CHECKOUT_BLOCKED.to_string()));
    Ok(())
}
