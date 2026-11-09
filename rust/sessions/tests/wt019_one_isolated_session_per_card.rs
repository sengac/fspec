//! WT-019: a card is worked in at most one isolated session at a time.
//!
//! Feature: spec/features/a-card-is-worked-in-at-most-one-isolated-session-at-a-time.feature
//!
//! Real `SessionManager` sessions on a fresh temp git repo (repo-local
//! identity; git calls ignore the user's global and system config) whose
//! committed board holds STORY-001 and STORY-002. HOME and the data
//! directory are per-test temp dirs, so session and git-session manifests
//! never touch the user's home. Focus is set through
//! `SessionManager::set_session_focus`, the path the RPC and the TUI use.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use codelet_core::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use serial_test::serial;
use uuid::Uuid;

/// Send + Sync so test futures holding an error across an await stay Send.
type TestError = Box<dyn std::error::Error + Send + Sync>;
type TestResult = Result<(), TestError>;

const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");
const MODEL: &str = "anthropic/claude-opus-4-5";

fn git(cwd: &Path, args: &[&str]) -> Result<(), TestError> {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()?;
    if !out.status.success() {
        return Err(format!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )
        .into());
    }
    Ok(())
}

fn board() -> serde_json::Value {
    let unit = |id: &str| {
        serde_json::json!({"id": id, "title": id, "type": "story", "status": "specifying",
                           "createdAt": "2026-11-07T00:00:00.000Z",
                           "updatedAt": "2026-11-07T00:00:00.000Z"})
    };
    serde_json::json!({
        "meta": {"version": "0.7.1", "lastUpdated": "2026-11-07T00:00:00.000Z"},
        "workUnits": {"STORY-001": unit("STORY-001"), "STORY-002": unit("STORY-002")},
        "states": {"backlog": [], "specifying": ["STORY-001", "STORY-002"], "testing": [],
                   "implementing": [], "validating": [], "done": [], "blocked": []}
    })
}

/// A repo with a committed board, plus a hermetic manager (temp HOME and
/// data dir, seeded model cache, dummy credentials).
struct Fixture {
    repo: tempfile::TempDir,
    _data: tempfile::TempDir,
    _home: tempfile::TempDir,
    manager: Arc<SessionManager>,
}

impl Fixture {
    fn new() -> Result<Fixture, TestError> {
        let repo = tempfile::tempdir()?;
        let path = repo.path();
        git(path, &["init", "-q"])?;
        git(path, &["config", "user.email", "test@example.com"])?;
        git(path, &["config", "user.name", "Test User"])?;
        std::fs::write(path.join(".gitignore"), ".fspec/\nspec/.fspec-write.lock\n")?;
        std::fs::create_dir_all(path.join("spec"))?;
        std::fs::write(
            path.join("spec").join("work-units.json"),
            serde_json::to_string_pretty(&board())?,
        )?;
        git(path, &["add", "-A"])?;
        git(path, &["commit", "-q", "-m", "initial commit"])?;

        std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
        let data = tempfile::tempdir()?;
        let home = tempfile::tempdir()?;
        let cache = data.path().join("cache");
        std::fs::create_dir_all(&cache)?;
        std::fs::write(cache.join("models.json"), MODELS_FIXTURE)?;
        codelet_core::persistence::reset_stores_for_tests();
        codelet_test_helpers::isolated_user_directory()?;
        codelet_common::set_data_directory(data.path().to_path_buf())?;
        std::env::set_var("HOME", home.path());
        Ok(Fixture {
            repo,
            _data: data,
            _home: home,
            manager: SessionManager::new(),
        })
    }

    fn project(&self) -> Result<String, TestError> {
        Ok(self
            .repo
            .path()
            .to_str()
            .ok_or("repo path is not UTF-8")?
            .to_string())
    }

    async fn isolated(&self, name: &str) -> Result<SessionId, TestError> {
        let id = Uuid::new_v4().to_string();
        self.manager
            .create_isolated_session_with_id(&id, MODEL, &self.project()?, name)
            .await?;
        Ok(SessionId::new(id))
    }

    async fn plain(&self, name: &str) -> Result<SessionId, TestError> {
        let id = Uuid::new_v4().to_string();
        self.manager
            .create_session_with_id(&id, MODEL, &self.project()?, name)
            .await?;
        Ok(SessionId::new(id))
    }

    fn focus(&self, session: &SessionId, card: &str) -> Result<(), String> {
        self.manager
            .set_session_focus(session, Some(card), "test")
            .map(drop)
            .map_err(|error| error.to_string())
    }

    fn focused(&self, session: &SessionId) -> Result<Option<String>, TestError> {
        Ok(self
            .manager
            .get_work_unit_focus(session)?
            .context
            .map(|context| context.id))
    }
}

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_second_isolated_session_cannot_take_a_card_another_isolated_session_holds() -> TestResult
{
    // @step Given a temp git repo whose board has STORY-001 and STORY-002, and isolated sessions A and B on it
    let fixture = Fixture::new()?;
    let a = fixture.isolated("A").await?;
    let b = fixture.isolated("B").await?;

    // @step And A focuses STORY-001
    fixture.focus(&a, "STORY-001")?;

    // @step When B sets its focus to STORY-001
    let outcome = fixture.focus(&b, "STORY-001");

    // @step Then it fails naming STORY-001 and session A
    let error = outcome.err().ok_or("B took STORY-001 while A held it")?;
    assert!(error.contains("STORY-001"), "{error}");
    assert!(error.contains(&a.value), "{error}");

    // @step And B has no focus
    assert_eq!(fixture.focused(&b)?, None);
    assert_eq!(fixture.focused(&a)?, Some("STORY-001".to_string()));
    Ok(())
}

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn moving_closing_or_detaching_the_holder_releases_the_card() -> TestResult {
    // @step Given a temp git repo whose board has STORY-001 and STORY-002, and isolated sessions A and B on it
    let fixture = Fixture::new()?;
    let a = fixture.isolated("A").await?;
    let b = fixture.isolated("B").await?;

    // @step When A focuses STORY-001 and then STORY-002
    fixture.focus(&a, "STORY-001")?;
    fixture.focus(&a, "STORY-002")?;

    // @step Then B can focus STORY-001
    fixture.focus(&b, "STORY-001")?;

    // @step When B is detached from its worktree
    fixture.manager.detach_session_worktree(&b)?;

    // @step Then A can focus STORY-001
    fixture.focus(&a, "STORY-001")?;

    // @step When A is closed
    fixture.manager.destroy_session(&a.value)?;

    // @step Then a new isolated session C can focus STORY-001
    let c = fixture.isolated("C").await?;
    fixture.focus(&c, "STORY-001")?;
    assert_eq!(fixture.focused(&c)?, Some("STORY-001".to_string()));
    Ok(())
}

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_isolated_sessions_are_unaffected() -> TestResult {
    // @step Given a temp git repo whose board has STORY-001 and STORY-002, an isolated session A and a non-isolated session N on it
    let fixture = Fixture::new()?;
    let a = fixture.isolated("A").await?;
    let n = fixture.plain("N").await?;

    // @step And A focuses STORY-001
    fixture.focus(&a, "STORY-001")?;

    // @step When N sets its focus to STORY-001
    fixture.focus(&n, "STORY-001")?;

    // @step Then N focuses STORY-001
    assert_eq!(fixture.focused(&n)?, Some("STORY-001".to_string()));
    Ok(())
}
