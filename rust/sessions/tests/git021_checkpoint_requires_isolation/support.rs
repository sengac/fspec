//! COV-057: offline harness for the isolated-session checkpoint scenarios.
//!
//! Precedent: rust/sessions/tests/iso001_support/mod.rs — a fresh git
//! repository, the offline models.dev fixture seeded into a hermetic data
//! dir, dummy credentials, and HOME redirected so the git-session manifest
//! (`~/.fspec/git-sessions`) stays inside the temp dir. Isolated sessions are
//! created through the real `SessionManager::create_isolated_session_with_id`
//! (real worktree under `<repo>/.fspec/worktrees/<session-id>`).

use super::*;

use std::path::Path;
use std::process::Command;

use codelet_sessions::background_session::BackgroundSession;

/// Trimmed offline models.dev catalog shared with the ISO-001 suite.
pub const MODELS_FIXTURE: &str = include_str!("../fixtures/prov101_models.json");

/// Run `git` in `cwd`, returning trimmed stdout or the stderr as an error.
pub fn git(cwd: &Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let out = Command::new("git").args(args).current_dir(cwd).output()?;
    if !out.status.success() {
        return Err(format!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(out.stdout)?.trim().to_owned())
}

/// Hermetic environment for isolated sessions; restores HOME on drop.
pub struct IsolatedEnv {
    pub manager: Arc<SessionManager>,
    pub repo: tempfile::TempDir,
    _data: tempfile::TempDir,
    previous_home: Option<std::ffi::OsString>,
}

impl Drop for IsolatedEnv {
    fn drop(&mut self) {
        match &self.previous_home {
            Some(home) => std::env::set_var("HOME", home),
            None => std::env::remove_var("HOME"),
        }
    }
}

impl IsolatedEnv {
    /// Fresh repo (branch `main`, committed `README.md`) plus a manager
    /// rooted in a hermetic data dir.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let data = tempfile::tempdir()?;
        codelet_common::set_data_directory(data.path().to_path_buf())?;
        codelet_core::persistence::reset_stores_for_tests();
        std::fs::create_dir_all(data.path().join("cache"))?;
        std::fs::write(data.path().join("cache/models.json"), MODELS_FIXTURE)?;
        std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
        let previous_home = std::env::var_os("HOME");
        let home = data.path().join("home");
        std::fs::create_dir_all(&home)?;
        std::env::set_var("HOME", &home);

        let repo = tempfile::tempdir()?;
        git(repo.path(), &["init", "-b", "main"])?;
        git(repo.path(), &["config", "user.email", "test@example.com"])?;
        git(repo.path(), &["config", "user.name", "Test User"])?;
        std::fs::write(repo.path().join("README.md"), "original readme\n")?;
        git(repo.path(), &["add", "."])?;
        git(repo.path(), &["commit", "-m", "initial commit"])?;
        Ok(Self {
            manager: SessionManager::new(),
            repo,
            _data: data,
            previous_home,
        })
    }

    /// Create a real isolated (worktree-backed) session on the repo.
    pub async fn isolated_session(
        &self,
    ) -> Result<Arc<BackgroundSession>, Box<dyn std::error::Error>> {
        let id = uuid::Uuid::new_v4().to_string();
        let project = self.repo.path().to_str().ok_or("repo path is not utf8")?;
        self.manager
            .create_isolated_session_with_id(&id, "anthropic/claude-opus-4-5", project, "git021")
            .await?;
        let session = self.manager.get_session(&id)?;
        let worktree = session.worktree_path().ok_or("session is not isolated")?;
        assert!(worktree.is_dir(), "worktree {} missing", worktree.display());
        Ok(session)
    }
}

/// The worktree directory of an isolated session.
pub fn worktree(session: &BackgroundSession) -> Result<PathBuf, Box<dyn std::error::Error>> {
    session
        .worktree_path()
        .ok_or_else(|| "session is not isolated".into())
}

/// The SHA stored at `refs/fspec-checkpoints/<session-id>/<label>`.
pub fn checkpoint_ref(
    session: &BackgroundSession,
    label: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let reference = format!("refs/fspec-checkpoints/{}/{label}", session.id);
    git(&worktree(session)?, &["rev-parse", "--verify", &reference])
}
