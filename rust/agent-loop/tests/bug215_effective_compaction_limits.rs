//! Feature: spec/features/compaction-builder-effective-limits.feature

#[path = "support/bug211_budget.rs"]
pub mod budget;
#[path = "support/bug210_completion_metrics.rs"]
pub mod metrics;

use codelet_agent_loop::generate_compaction_handler::{
    register_compactor_sub_agent_handler, register_generate_compaction_handler,
};
use codelet_tools::{GenerateCompactionArgs, GenerateCompactionTool};
use rig::client::CompletionClient;
use rig::completion::CompletionRequestBuilder;
use rig::tool::Tool;

struct Environment(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl Environment {
    fn capture() -> Self {
        Self(
            [
                "OPENAI_API_KEY",
                "OPENAI_BASE_URL",
                "OPENAI_MODEL",
                "OPENAI_CONTEXT_WINDOW",
                "OPENAI_MAX_OUTPUT_TOKENS",
            ]
            .into_iter()
            .map(|key| (key, std::env::var_os(key)))
            .collect(),
        )
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        for (key, previous) in &self.0 {
            match previous {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

async fn exercise(
    generated: bool,
    provider: &str,
    capacity: Option<usize>,
    output: Option<usize>,
    valid: bool,
) -> metrics::TestResult {
    let _environment = Environment::capture();
    std::env::set_var("OPENAI_MODEL", "o3");
    std::env::set_var("OPENAI_CONTEXT_WINDOW", "200000");
    std::env::set_var("OPENAI_MAX_OUTPUT_TOKENS", "3072");
    let fixture = metrics::Fixture::new().await?;
    budget::exploration(&fixture.server, fixture.session.id, metrics::DAG, false).await;
    let expected_limits = {
        let mut inner = fixture.session.inner.lock().await;
        inner.provider_manager_mut().set_model_direct_with_profile(
            provider,
            "o3",
            Some("offline-compaction"),
            capacity,
            output,
            (provider != "openai").then(|| "openai".to_string()),
        )?;
        let limits = codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(inner.provider_manager().context_window() as u64),
            output_reservation: Some(inner.provider_manager().max_output_tokens() as u64),
        };
        let client: rig::providers::openai::CompletionsClient =
            rig::providers::openai::CompletionsClient::builder()
                .api_key("offline-selected-request")
                .base_url(fixture.server.uri())
                .build()?;
        let model = client.completion_model("o3");
        inner.install_selected_preparer(
            codelet_providers::selected_request::SelectedRequestPreparer::new(
                model.clone(),
                CompletionRequestBuilder::new(model, "unused")
                    .max_tokens(limits.output_reservation.ok_or("resolved output")?)
                    .build(),
                limits,
            ),
        );
        let mut manifest = codelet_core::persistence::load_session(fixture.session.id)?;
        manifest.provider = inner
            .provider_manager()
            .selected_model_string()
            .ok_or("profile selection")?;
        codelet_core::persistence::save_session(&manifest)?;
        register_compactor_sub_agent_handler(
            fixture.session.id,
            &inner,
            std::env::temp_dir(),
            fixture.session.compaction_in_progress.clone(),
        );
        limits
    };
    let original = codelet_core::persistence::load_session(fixture.session.id)?;
    let original_messages = codelet_core::persistence::get_session_messages_full(&original)?;
    let result = if generated {
        let queue = {
            let inner = fixture.session.inner.lock().await;
            register_generate_compaction_handler(
                fixture.session.id,
                &inner,
                std::env::temp_dir(),
                fixture.session.clone(),
                None,
            )
        };
        let result = GenerateCompactionTool::new(fixture.session.id)
            .call(GenerateCompactionArgs { session_id: None })
            .await;
        match result {
            Ok(summary) => {
                let summary: serde_json::Value = serde_json::from_str(&summary)?;
                assert_eq!(summary["status"], "queued");
                assert_eq!(summary["content"], metrics::DAG);
                let mut inner = fixture.session.inner.lock().await;
                queue.apply(&mut inner, &fixture.session).map(|applied| {
                    assert!(applied);
                })
            }
            Err(error) => Err(error.to_string()),
        }
    } else {
        let output = codelet_agent_loop::BackgroundOutput::with_provider(
            fixture.session.clone(),
            "openai".into(),
            None,
        );
        fixture
            .run(&output)
            .await
            .map_err(|error| error.to_string())
    };
    let requests = fixture.server.received_requests().await.ok_or("requests")?;
    let retained = codelet_core::persistence::load_session(fixture.session.id)?;
    assert_eq!(
        serde_json::to_value(codelet_core::persistence::get_session_messages_full(
            &retained
        )?)?,
        serde_json::to_value(original_messages)?
    );
    if valid {
        assert!(
            !requests.is_empty(),
            "valid exploration must reach the local provider"
        );
        assert!(result.is_ok(), "generated={generated}: {result:?}");
        assert!(retained.compaction_commit.is_some());
        let synthesis = requests
            .iter()
            .filter(|request| !budget::has_tools(request))
            .collect::<Vec<_>>();
        assert_eq!(synthesis.len(), 1);
        let body: serde_json::Value = synthesis[0].body_json()?;
        assert_eq!(body["model"], "o3");
        let allowance = body["max_completion_tokens"]
            .as_u64()
            .or_else(|| body["max_tokens"].as_u64());
        assert_eq!(
            allowance,
            expected_limits
                .output_reservation
                .map(|tokens| tokens.min(8192))
        );
        codelet_providers::request_admission::admit_complete_request(
            &synthesis[0].body,
            expected_limits,
        )?;
        assert!(body["messages"]
            .to_string()
            .contains(&format!("original {}", requests.len() - 2)));
    } else {
        assert!(
            requests.is_empty(),
            "invalid limits must fail before dispatch"
        );
        let error = result.expect_err("invalid synthesis must be rejected");
        let expected_error = if capacity == Some(0) {
            "positive context capacity"
        } else if output == Some(0) {
            "positive output reservation"
        } else {
            "leaves no input budget"
        };
        assert!(error.contains(expected_error), "{error}");
        assert!(retained.compaction_commit.is_none());
        assert!(requests.iter().all(budget::has_tools));
    }
    assert!(!fixture
        .session
        .compaction_in_progress
        .load(std::sync::atomic::Ordering::Acquire));
    codelet_tools::set_generate_compaction_handler(fixture.session.id, None);
    codelet_tools::set_inject_summary_handler(fixture.session.id, None);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_missing_output_metadata_uses_the_effective_provider_reservation(
) -> metrics::TestResult {
    // @step Given a profile has explicit context capacity but no output metadata
    let capacity = Some(200_000);
    // @step When both compaction routes exhaust exploration and synthesize a summary
    // @step Then synthesis uses the provider output fallback and commits a valid summary
    for generated in [false, true] {
        exercise(generated, "openai", capacity, None, true).await?;
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_missing_context_and_output_metadata_use_effective_provider_defaults(
) -> metrics::TestResult {
    // @step Given a profile has neither context nor output metadata
    let capacity = None;
    // @step When both compaction routes exhaust exploration and synthesize a summary
    // @step Then synthesis uses both provider defaults and commits a valid summary
    for generated in [false, true] {
        exercise(generated, "openai", capacity, None, true).await?;
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_explicit_limits_and_provider_clamps_remain_authoritative() -> metrics::TestResult
{
    // @step Given profiles have explicit limits including overrides above a provider hard cap
    let cases = [("openai", 200_000, 2048), ("anthropic", 1_000_000, 128_000)];
    // @step When both compaction routes exhaust exploration and synthesize a summary
    // @step Then synthesis respects the resolved output allowance and commits a valid summary
    for (provider, capacity, output) in cases {
        for generated in [false, true] {
            exercise(generated, provider, Some(capacity), Some(output), true).await?;
        }
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_invalid_explicit_limits_fail_closed_without_durable_promotion(
) -> metrics::TestResult {
    // @step Given profiles have zero capacity or zero output or no remaining input budget
    let cases = [(0, 4096), (200_000, 0), (4096, 4096), (4096, 8192)];
    // @step When both compaction routes attempt compaction
    // @step Then invalid limits are rejected locally and durable originals remain unchanged
    for (capacity, output) in cases {
        for generated in [false, true] {
            exercise(generated, "openai", Some(capacity), Some(output), false).await?;
        }
    }
    Ok(())
}
