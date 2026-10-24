#![doc = "Feature: spec/features/codex-subagent-recovery-ownership.feature"]

#[path = "../src/auxiliary_agent_builder.rs"]
mod auxiliary_agent_builder;
#[path = "../src/deep_search_handler.rs"]
pub mod deep_search_handler;
#[path = "../src/deep_search_provider_config.rs"]
mod deep_search_provider_config;

pub use codelet_agent_loop::{graph_search_handler, session_search_handler};
use codelet_providers::codex::reconnect::{CodexNetworkFailure, NetworkFailure};
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::completion::{CompletionError, GetTokenUsage, Usage};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::time::Instant;

#[derive(Clone)]
struct Response;

impl GetTokenUsage for Response {
    fn token_usage(&self) -> Option<Usage> {
        None
    }
}

fn wrapped_exhaustion() -> anyhow::Error {
    anyhow::Error::from(CompletionError::HttpError(
        rig::http_client::Error::Instance(Box::new(CodexNetworkFailure {
            source: NetworkFailure::Interrupted,
        })),
    ))
    .context("connection reset by peer while collecting Codex exhaustion")
}

#[tokio::test(start_paused = true)]
async fn scenario_subagent_collectors_preserve_codex_terminal_failures_and_ordinary_recovery() {
    // @step Given Codex subagents with wrapped typed exhaustion or permanent errors containing network-like text
    for (provider, error) in [
        ("codex", wrapped_exhaustion()),
        (
            "codex",
            anyhow::anyhow!("permanent provider rejection: connection reset is not permitted"),
        ),
        ("zai", wrapped_exhaustion()),
    ] {
        let expected = format!("DeepSearch sub-agent failed: {error}");
        let polls = Arc::new(AtomicUsize::new(0));
        let recorded = polls.clone();
        let stream = futures::stream::iter(vec![
            Err::<MultiTurnStreamItem<Response>, _>(error),
            Ok(MultiTurnStreamItem::final_response(
                "must not hide the terminal failure",
                Usage::new(),
            )),
        ])
        .inspect(move |_| {
            recorded.fetch_add(1, Ordering::SeqCst);
        });
        let started = Instant::now();

        // @step When DeepSearch or compaction collects the streaming result
        let result =
            deep_search_handler::collect_final_response_from_stream(stream, provider).await;

        // @step Then the original terminal failure is returned without polling further or generic backoff
        assert_eq!(result, Err(expected), "provider: {provider}");
        assert_eq!(polls.load(Ordering::SeqCst), 1, "provider: {provider}");
        assert_eq!(started.elapsed(), Duration::ZERO, "provider: {provider}");
    }

    // @step And ordinary provider recovery retains its retry budget and subagent hard deadlines remain enforced
    let stream = futures::stream::iter(vec![
        Err::<MultiTurnStreamItem<Response>, _>(anyhow::anyhow!("connection reset by peer")),
        Ok(MultiTurnStreamItem::final_response(
            "ordinary recovery succeeds",
            Usage::new(),
        )),
    ]);
    let started = Instant::now();
    let result = deep_search_handler::collect_final_response_from_stream(stream, "zai").await;
    assert_eq!(result, Ok("ordinary recovery succeeds".into()));
    assert_eq!(started.elapsed(), Duration::from_secs(1));

    let polls = Arc::new(AtomicUsize::new(0));
    let recorded = polls.clone();
    let stream = futures::stream::iter((1..=5).map(|attempt| {
        Err::<MultiTurnStreamItem<Response>, _>(anyhow::anyhow!(
            "connection reset by peer {attempt}"
        ))
    }))
    .inspect(move |_| {
        recorded.fetch_add(1, Ordering::SeqCst);
    });
    let started = Instant::now();
    let result = deep_search_handler::collect_final_response_from_stream(stream, "zai").await;
    assert_eq!(
        result,
        Err("DeepSearch sub-agent failed: connection reset by peer 4".into())
    );
    assert_eq!(polls.load(Ordering::SeqCst), 4);
    assert_eq!(started.elapsed(), Duration::from_secs(7));

    let stream = futures::stream::iter(vec![Err::<MultiTurnStreamItem<Response>, _>(
        anyhow::anyhow!("connection reset by peer"),
    )]);
    let result = tokio::time::timeout(
        Duration::from_millis(50),
        deep_search_handler::collect_final_response_from_stream(stream, "zai"),
    )
    .await;
    assert!(
        result.is_err(),
        "hard deadline must interrupt generic backoff"
    );

    let stream = futures::stream::pending::<Result<MultiTurnStreamItem<Response>, anyhow::Error>>();
    let result = tokio::time::timeout(
        Duration::from_millis(50),
        deep_search_handler::collect_final_response_from_stream(stream, "codex"),
    )
    .await;
    assert!(
        result.is_err(),
        "hard deadline must interrupt Codex polling"
    );
}
