//! BUG-466 — session lookups and listings report failures.
//!
//! Feature: spec/features/session-lookups-and-listings-report-failures.feature

use codelet_test_helpers::TestResult;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{Action, EmbeddedFspecBackend, FspecBackend};
use codelet_rpc::SharedFspecService;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use serial_test::serial;

mod common;
use common::harness::AppTestHarness;

#[path = "common/session_storage_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use fixture::StorageFixture;

fn git(cwd: &std::path::Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let out = Command::new("git").args(args).current_dir(cwd).output()?;
    if !out.status.success() {
        return Err(format!("git {args:?}: {}", String::from_utf8_lossy(&out.stderr)).into());
    }
    Ok(())
}

fn screen(harness: &mut AppTestHarness) -> String {
    let area = Rect::new(0, 0, 200, 50);
    let mut buffer = Buffer::empty(area);
    harness.app.render(area, &mut buffer);
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
#[serial]
fn a_failed_provider_credential_lookup_is_reported() -> TestResult {
    // @step Given a providers directory containing an invalid "broken.json"
    let base = tempfile::tempdir()?;
    std::fs::create_dir_all(base.path().join("credentials"))?;
    std::fs::create_dir_all(base.path().join("providers"))?;
    std::fs::write(base.path().join("providers/broken.json"), "{ not json")?;
    std::env::set_var("FSPEC_HOME", base.path().join("credentials"));
    // @step When the session manager looks up the "anthropic" provider credential
    let manager = codelet_sessions::SessionManager::new();
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let looked_up = handle.get_provider_credential("anthropic");
    std::env::remove_var("FSPEC_HOME");
    // @step Then the lookup fails naming the invalid provider config
    let error = looked_up.err().ok_or("a credential was looked up")?;
    assert!(error.contains("broken.json"), "{error}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_active_session_lookup_is_refused_without_a_session_manager() -> TestResult {
    // @step Given an RPC service with no session manager attached
    let directory = tempfile::tempdir()?;
    std::fs::create_dir_all(directory.path().join("spec"))?;
    std::fs::write(
        directory.path().join("spec/work-units.json"),
        "{\"workUnits\":{}}",
    )?;
    let watcher = Arc::new(WorkUnitsWatcher::new(directory.path())?);
    let service = Arc::new(SharedFspecService::new(watcher));
    let backend = EmbeddedFspecBackend::new(tokio::runtime::Handle::current(), service);
    // @step When the active session is read over the embedded transport
    let read = backend.get_active_session().await;
    // @step Then the read fails saying no session manager is attached
    let error = read.err().ok_or("an active session was read")?.to_string();
    assert!(error.contains("no session manager is attached"), "{error}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial]
async fn a_worktree_whose_changes_cannot_be_read_fails_the_worktree_listing() -> TestResult {
    // @step Given an isolated session worktree containing a file that cannot be read
    let fixture = StorageFixture::new().await?;
    let home = tempfile::tempdir()?;
    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", home.path());
    let repo = tempfile::tempdir()?;
    git(repo.path(), &["init"])?;
    git(repo.path(), &["config", "user.email", "test@example.com"])?;
    git(repo.path(), &["config", "user.name", "Test User"])?;
    std::fs::write(repo.path().join("hello.txt"), "hello\n")?;
    git(repo.path(), &["add", "."])?;
    git(repo.path(), &["commit", "-m", "initial commit"])?;
    let id = uuid::Uuid::new_v4().to_string();
    let created = fixture
        .manager
        .create_isolated_session_with_id(
            &id,
            "anthropic/claude-opus-4-5",
            repo.path().to_str().ok_or("repo path is utf8")?,
            "BUG-466 isolated",
        )
        .await;
    let worktree = repo.path().join(".fspec/worktrees").join(&id);
    let secret = worktree.join("secret.txt");
    let listed = match created {
        Ok(_) => {
            std::fs::write(&secret, "hidden\n")?;
            std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o000))?;
            // @step When the session worktrees are listed
            let handle: &dyn SessionManagerHandle = fixture.manager.as_ref();
            Ok(handle.list_session_worktrees())
        }
        Err(error) => Err(error),
    };
    if secret.exists() {
        std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o644))?;
    }
    match previous_home {
        Some(previous) => std::env::set_var("HOME", previous),
        None => std::env::remove_var("HOME"),
    }
    // @step Then the listing fails instead of reporting the worktree as clean
    let listed = listed?;
    assert!(
        listed.is_err(),
        "an unreadable worktree was listed: {listed:?}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_session_listing_is_shown_when_the_resume_view_opens() -> TestResult {
    // @step Given a backend whose session listing fails with "sessions directory is unreadable"
    let mut harness = AppTestHarness::empty();
    harness.drain_pending().await?;
    harness
        .mock
        .fail_list_sessions("sessions directory is unreadable");
    // @step When the resume view opens
    harness.app.dispatch(Action::SlashCommandSelected(
        harness.app.current_session(),
        codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction::Resume,
    ));
    harness.drain_pending().await?;
    // @step Then an error dialog shows "sessions directory is unreadable"
    let screen = screen(&mut harness);
    assert!(
        screen.contains("sessions directory is unreadable"),
        "{screen}"
    );
    Ok(())
}

#[test]
fn backend_defaults_refuse_instead_of_answering() -> TestResult {
    // @step Given the FspecBackend trait defaults for profile, continue, goal, default-model and thinking-default calls and send_input_with_thinking
    let backend_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/transport/backend");
    let mut source = String::new();
    for entry in std::fs::read_dir(&backend_dir)? {
        source.push_str(&std::fs::read_to_string(entry?.path())?);
    }
    // @step When their default bodies are read
    let methods = [
        "save_profile",
        "delete_profile",
        "rename_profile",
        "set_default_model",
        "set_thinking_level_default",
        "set_continue_state",
        "get_continue_state",
        "set_goal_state",
        "get_goal_state",
        "send_input_with_thinking",
    ];
    // @step Then every default returns an error instead of a value
    for method in methods {
        let start = source
            .find(&format!("async fn {method}("))
            .ok_or(format!("{method} is not declared"))?;
        let rest = &source[start..];
        let body_end = rest
            .find("\n    }\n")
            .ok_or(format!("{method} has no body"))?;
        let body = &rest[..body_end];
        assert!(
            body.contains("is not supported by this backend"),
            "{method} default answers instead of refusing: {body}"
        );
    }
    Ok(())
}
