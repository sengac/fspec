//! Feature: spec/features/legacy-model-selection-atomicity.feature
#![cfg(not(feature = "noop"))]

use std::ffi::OsString;
use std::sync::{atomic::Ordering, Arc};

use codelet_napi::{session_set_model, session_set_model_profile};
use codelet_sessions::{background_session::BackgroundSession, SessionManager};
use serial_test::serial;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Snapshot = (
    Option<String>,
    Option<String>,
    String,
    [u32; 3],
    Option<(String, u64)>,
    Option<String>,
);

struct Fixture {
    _directory: tempfile::TempDir,
    environment: Vec<(&'static str, Option<OsString>)>,
    session: Arc<BackgroundSession>,
}

impl Fixture {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        let environment = ["FSPEC_USER_DIR", "OPENAI_API_KEY", "OPENAI_BASE_URL"]
            .into_iter()
            .map(|key| (key, std::env::var_os(key)))
            .collect();
        std::env::set_var("FSPEC_USER_DIR", directory.path());
        std::env::set_var("OPENAI_API_KEY", "offline");
        std::env::set_var("OPENAI_BASE_URL", "http://127.0.0.1:1/v1");
        codelet_common::set_data_directory(directory.path().to_owned())?;
        codelet_core::persistence::reset_stores_for_tests();
        let cache = directory.path().join("cache");
        std::fs::create_dir_all(&cache)?;
        std::fs::write(
            cache.join("models.json"),
            include_str!("../../sessions/tests/fixtures/prov129_codex_models.json"),
        )?;
        std::fs::write(
            directory.path().join("fspec-config.json"),
            serde_json::to_vec(&serde_json::json!({
                "providers": {"openai": {"profiles": {"offline": {
                    "baseUrl": "http://127.0.0.1:1/v1", "apiKey": "offline",
                    "contextWindow": 64000, "customModels": [{"id": "A", "maxOutputTokens": 4096}]
                }}}}
            }))?,
        )?;
        let id = Uuid::new_v4().to_string();
        let manager = SessionManager::instance();
        manager
            .create_session_with_id(
                &id,
                "openai:offline/A",
                &directory.path().to_string_lossy(),
                "atomicity",
            )
            .await?;
        let session = manager.get_session(&id)?;
        session
            .inner
            .lock()
            .await
            .provider_manager_mut()
            .set_compaction_threshold_override(Some(("tokens".into(), 20000)));
        session.set_model_limits(64000, 4096, 20000);
        Ok(Self {
            _directory: directory,
            environment,
            session,
        })
    }

    async fn snapshot(&self) -> Result<Snapshot, Box<dyn std::error::Error>> {
        let inner = self.session.inner.lock().await;
        let provider = inner.provider_manager();
        Ok((
            self.session
                .provider_id
                .read()
                .map_err(|error| error.to_string())?
                .clone(),
            self.session
                .model_id
                .read()
                .map_err(|error| error.to_string())?
                .clone(),
            provider.selected_model_string().unwrap_or_default(),
            [
                self.session.cached_context_window.load(Ordering::Acquire),
                self.session
                    .cached_max_output_tokens
                    .load(Ordering::Acquire),
                self.session
                    .cached_compaction_threshold
                    .load(Ordering::Acquire),
            ],
            provider
                .compaction_threshold_override()
                .map(|(kind, value)| (kind.to_owned(), value)),
            provider.facade_override().map(str::to_owned),
        ))
    }

    async fn set(&self, profile: bool, reject: bool) -> napi::Result<()> {
        let provider = if reject {
            "bug226-unknown-provider"
        } else if profile {
            "openai"
        } else {
            "codex"
        };
        if profile {
            session_set_model_profile(
                self.session.id.to_string(),
                provider.into(),
                "B".into(),
                Some(96000),
                Some(8192),
                Some("openai".into()),
                Some("tokens".into()),
                Some(30000),
                Some("offline".into()),
            )
            .await
        } else {
            session_set_model(
                self.session.id.to_string(),
                provider.into(),
                "gpt-5.4".into(),
                Some(96000),
                Some(8192),
                Some("tokens".into()),
                Some(30000),
            )
            .await
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = SessionManager::instance().destroy_session(&self.session.id.to_string());
        codelet_core::persistence::reset_stores_for_tests();
        for (key, value) in &self.environment {
            if let Some(value) = value {
                std::env::set_var(key, value);
            } else {
                std::env::remove_var(key);
            }
        }
    }
}

#[tokio::test]
#[serial]
async fn rejected_ordinary_selection_preserves_model_state() -> TestResult {
    // @step Given a legacy session with accepted identity limits and threshold override
    let fixture = Fixture::new().await?;
    let before = fixture.snapshot().await?;
    // @step When the ordinary setter rejects an unknown provider with replacement limits
    let result = fixture.set(false, true).await;
    assert!(result.is_err(), "ordinary setter must genuinely reject");
    // @step Then identity live selection cached limits and override are unchanged
    assert_eq!(fixture.snapshot().await?, before);
    Ok(())
}

#[tokio::test]
#[serial]
async fn rejected_profile_selection_preserves_model_state() -> TestResult {
    // @step Given a legacy session with accepted identity limits and threshold override
    let fixture = Fixture::new().await?;
    let before = fixture.snapshot().await?;
    // @step When the profile setter rejects an unknown provider with replacement limits
    let result = fixture.set(true, true).await;
    assert!(
        result
            .as_ref()
            .is_err_and(|error| error.reason.contains("not supported")),
        "actual direct selection rejection: {result:?}"
    );
    // @step Then identity live selection cached limits and override are unchanged
    assert_eq!(fixture.snapshot().await?, before);
    Ok(())
}

#[tokio::test]
#[serial]
async fn successful_legacy_setters_publish_identity_limits_and_override_together() -> TestResult {
    // @step Given a legacy session with accepted identity limits and threshold override
    let fixture = Fixture::new().await?;
    // @step When each legacy setter accepts a replacement model
    for profile in [false, true] {
        fixture.set(profile, false).await?;
        // @step Then labels live selection cached limits and override describe the replacement
        let state = fixture.snapshot().await?;
        assert_eq!(
            state.0.as_deref(),
            Some(if profile { "openai" } else { "codex" })
        );
        assert_eq!(
            state.1.as_deref(),
            Some(if profile { "B" } else { "gpt-5.4" })
        );
        assert!(state.2.ends_with(if profile { "/B" } else { "/gpt-5.4" }));
        assert_eq!(state.3, [96000, 8192, 30000]);
        assert_eq!(state.4, Some(("tokens".into(), 30000)));
    }
    Ok(())
}
