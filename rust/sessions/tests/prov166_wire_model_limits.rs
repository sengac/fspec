//! PROV-166: session model limits travel on the wire as null when unavailable.
//!
//! Feature: spec/features/session-model-limits-wire.feature

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A Codex session on the catalogued `codex/gpt-6.1-sol` (400000 / 128000).
async fn codex_session() -> Result<
    (tempfile::TempDir, std::sync::Arc<SessionManager>, SessionId),
    Box<dyn std::error::Error>,
> {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_core::persistence::reset_stores_for_tests();
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    std::fs::create_dir_all(directory.path().join("cache"))?;
    std::fs::write(
        directory.path().join("cache/models.json"),
        include_str!("fixtures/prov129_codex_models.json").replace("gpt-5.4", "gpt-6.1-sol"),
    )?;
    let manager = SessionManager::new();
    let id = manager
        .create_session(
            "codex/gpt-6.1-sol",
            directory.path().to_str().ok_or("path")?,
        )
        .await?;
    Ok((directory, manager, SessionId { value: id }))
}

/// Restores the process environment and working directory on drop.
struct Environment {
    vars: Vec<(&'static str, Option<std::ffi::OsString>)>,
    cwd: std::path::PathBuf,
    _dirs: Vec<tempfile::TempDir>,
}

impl Drop for Environment {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.cwd);
        for (key, prior) in self.vars.drain(..) {
            match prior {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

/// A session on the custom provider model `acme/opus-4.7`, which declares
/// no context window or output limit (PROV-170).
async fn custom_session(
) -> Result<(Environment, std::sync::Arc<SessionManager>, SessionId), Box<dyn std::error::Error>> {
    let data = codelet_test_helpers::setup_temp_directory()?;
    let home = tempfile::tempdir()?;
    let project = tempfile::tempdir()?;
    let user_dir = tempfile::tempdir()?;
    let codex_home = tempfile::tempdir()?;
    let credentials = home.path().join(".fspec").join("credentials");
    std::fs::create_dir_all(&credentials)?;
    let providers = project.path().join(".fspec").join("providers");
    std::fs::create_dir_all(&providers)?;
    std::fs::write(
        providers.join("acme.json"),
        serde_json::json!({
            "name": "acme",
            "display_name": "Acme",
            "facade": "claude",
            "base_url": "https://api.example.test",
            "api_key_env_var": "ACME_API_KEY",
            "models": { "opus-4.7": { "id": "acme-opus-4-7" } }
        })
        .to_string(),
    )?;
    let settings: [(&'static str, std::ffi::OsString); 5] = [
        ("HOME", home.path().into()),
        ("FSPEC_HOME", credentials.clone().into()),
        ("FSPEC_USER_DIR", user_dir.path().into()),
        ("CODEX_HOME", codex_home.path().into()),
        ("ACME_API_KEY", "sk-acme-test".into()),
    ];
    let mut environment = Environment {
        vars: Vec::new(),
        cwd: std::env::current_dir()?,
        _dirs: Vec::new(),
    };
    for (key, value) in settings {
        environment.vars.push((key, std::env::var_os(key)));
        std::env::set_var(key, value);
    }
    std::env::set_current_dir(project.path())?;
    codelet_core::persistence::reset_stores_for_tests();
    codelet_common::set_data_directory(data.path().to_path_buf())?;
    std::fs::create_dir_all(data.path().join("cache"))?;
    std::fs::write(
        data.path().join("cache/models.json"),
        include_str!("fixtures/prov129_codex_models.json"),
    )?;
    let manager = SessionManager::new();
    let id = manager
        .create_session("acme/opus-4.7", project.path().to_str().ok_or("path")?)
        .await?;
    environment._dirs = vec![data, home, project, user_dir, codex_home];
    Ok((environment, manager, SessionId { value: id }))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn a_session_without_limits_reports_null_limits() -> TestResult {
    // @step Given a session whose custom provider model declares no limits
    // (PROV-176: a Codex model models.dev does not know is refused at
    // selection, so a custom provider model carries the unavailable limits.)
    let (_environment, manager, id) = custom_session().await?;
    let handle: &dyn SessionManagerHandle = manager.as_ref();

    // @step When its session model and model info are read through the session manager handle
    let model = handle.get_session_model(&id)?;
    let info = handle.get_model_info(&id)?;

    // @step Then the session model's context window, output limit and compaction threshold are null
    assert_eq!(
        (
            model.context_window,
            model.max_output_tokens,
            model.compaction_threshold
        ),
        (None, None, None)
    );

    // @step And the model info's context window and compaction threshold are null
    assert_eq!(
        (info.context_window, info.compaction_threshold),
        (None, None)
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn a_catalogued_session_reports_its_limits() -> TestResult {
    // @step Given a session whose Codex model is in the catalog with a 400000 context window and a 128000 output limit
    let (_directory, manager, id) = codex_session().await?;
    let handle: &dyn SessionManagerHandle = manager.as_ref();

    // @step When its session model is read through the session manager handle
    let model = handle.get_session_model(&id)?;

    // @step Then the context window is 400000 and the output limit is 128000
    assert_eq!(model.context_window, Some(400_000));
    assert_eq!(model.max_output_tokens, Some(128_000));

    // @step And the compaction threshold is present
    assert!(model
        .compaction_threshold
        .is_some_and(|threshold| threshold > 0));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn reading_the_model_of_a_session_that_does_not_exist_fails_naming_it() -> TestResult {
    // @step Given a session manager without the requested session
    let manager = SessionManager::new();
    let handle: &dyn SessionManagerHandle = manager.as_ref();
    let missing = SessionId {
        value: uuid::Uuid::new_v4().to_string(),
    };

    // @step When its session model is read
    let read = handle.get_session_model(&missing);

    // @step Then the read fails naming the session id
    // BUG-445: an unknown session is the lookup error, not null limits.
    let error = read.err().ok_or("an unknown session has no model")?;
    assert!(error.contains(&missing.value), "{error}");
    Ok(())
}
