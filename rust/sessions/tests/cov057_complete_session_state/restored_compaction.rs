//! COV-057: automatic compaction on a cold-restored session.
//!
//! The threshold is a saved `fspec-config.json` setting (so the restored
//! session re-resolves it), every turn is a real `run_agent_stream` turn
//! against an offline wiremock Anthropic endpoint, and the restore is a real
//! cold `resume_session`. Only the compactor builder's model call is
//! replaced: the registered compactor sub-agent handler (the seam the agent
//! loop fills in production) answers with a DAG over the captured source;
//! capture, validation, commit and request replay are the real engine.

use super::*;
use codelet_core::persistence::compaction_boundary::CompactionSource;
use std::sync::{Arc, Mutex};

const THRESHOLD: u64 = 200_000;
const SUMMARY: &str = "Parser, tokenizer and error-span work completed across all earlier turns.";

fn prompt(index: usize) -> String {
    format!("prompt-{index:02}: continue the parser work")
}

/// Clears the process-global compactor registration on every exit path.
struct Builder(uuid::Uuid);

impl Builder {
    fn register(session: uuid::Uuid, captured: Arc<Mutex<Vec<CompactionSource>>>) -> Self {
        codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(
            session,
            Some(Arc::new(move |_, source: CompactionSource| {
                let end = source.prefix.len().saturating_sub(1);
                if let Ok(mut sources) = captured.lock() {
                    sources.push(source);
                }
                Box::pin(async move {
                    Ok(format!(
                        "<dag-node depth=\"D1\" turns=\"0-{end}\" label=\"Earlier work\">\n{SUMMARY}\n</dag-node>"
                    ))
                })
            })),
        );
        Self(session)
    }
}

impl Drop for Builder {
    fn drop(&mut self) {
        codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(self.0, None);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_compaction_operates_correctly_on_restored_session() -> TestResult {
    let (directory, manager, _, _) = manager().await?;
    std::fs::write(
        directory.path().join("fspec-config.json"),
        serde_json::to_vec(&serde_json::json!({"providers":{"anthropic":{"settings":{
            "contextWindow": 272_000, "maxOutputTokens": 8192,
            "compactionThreshold": {"type": "tokens", "value": THRESHOLD}
        }}}}))?,
    )?;
    let session_id = SessionId::new(
        manager
            .create_session(
                "anthropic/claude-sonnet-4",
                &directory.path().to_string_lossy(),
            )
            .await?,
    );
    let uuid = uuid::Uuid::parse_str(&session_id.value)?;
    // @step Given a session with 50 turns and 180000 input tokens
    for index in 0..50 {
        let usage = 3_600 * (index as u64 + 1);
        turn(&manager, &session_id, &prompt(index), "done", usage).await?;
    }
    {
        let session = manager.get_session(&session_id.value)?;
        let inner = session.inner.lock().await;
        assert_eq!(inner.token_tracker.input_tokens, 180_000);
        assert_eq!(load_session(uuid)?.messages.len(), 100);
    }
    // @step And the compaction threshold is 200000 tokens
    let threshold = |session: &codelet_sessions::background_session::BackgroundSession| {
        session.model_limits().compaction_threshold
    };
    let original = manager.get_session(&session_id.value)?;
    assert_eq!(threshold(&original), Some(200_000));
    drop(original);
    // @step When the session is saved and restored
    drop(manager);
    let restored = cold_resume(&session_id)?;
    let background = restored.get_session(&session_id.value)?;
    assert_eq!(threshold(&background), Some(200_000));
    // BUG-326: the restored input is the last request's input, exactly as
    // the live tracker reported it (the reply's 7 tokens are not input).
    assert_eq!(
        background.inner.lock().await.token_tracker.input_tokens,
        180_000
    );
    let captured = Arc::new(Mutex::new(Vec::new()));
    let _builder = Builder::register(uuid, captured.clone());
    // @step And 5 more turns are added reaching 210000 tokens
    for (offset, usage) in [185_000, 190_000, 195_000, 199_000, 210_000]
        .into_iter()
        .enumerate()
    {
        turn(&restored, &session_id, &prompt(50 + offset), "done", usage).await?;
        assert!(load_session(uuid)?.compaction_commit.is_none());
    }
    assert_eq!(
        background.inner.lock().await.token_tracker.input_tokens,
        210_000
    );
    // @step Then compaction is triggered by the next prompt
    let trigger = prompt(55);
    let wire = turn(&restored, &session_id, &trigger, "Spans wired", 1_200).await?;
    let sources = captured.lock().map_err(|_| "poisoned")?.clone();
    assert_eq!(sources.len(), 1, "exactly one automatic compaction round");
    let manifest = load_session(uuid)?;
    let commit = manifest
        .compaction_commit
        .as_ref()
        .ok_or("compaction committed")?;
    assert_eq!(commit.identity.generation, 1);
    // @step And compaction summarizes the oldest turns
    let covered = commit.covered_prefix().len();
    assert_eq!(
        covered, 111,
        "50 restored + 5 new turns + the trigger prompt"
    );
    let oldest: Vec<_> = manifest.messages[..covered]
        .iter()
        .map(|message| message.message_id)
        .collect();
    assert_eq!(commit.covered_prefix(), oldest.as_slice());
    let state = manifest.compaction.as_ref().ok_or("summary state")?;
    assert_eq!(state.compacted_before_index, covered);
    assert!(state.summary.contains(SUMMARY));
    let sent = serde_json::to_string(&wire["messages"])?;
    assert!(
        sent.contains(SUMMARY),
        "compacted request carries the summary"
    );
    // The trigger prompt is inside the covered prefix too: the engine sends
    // the summary plus a "Continue" in its place.
    for index in 0..=55 {
        assert!(
            !sent.contains(&prompt(index)),
            "turn {index} was not summarized"
        );
    }
    // @step And the turn after the compaction boundary is preserved verbatim
    // Durable replay (what a restore loads): the summary, then the
    // post-boundary reply exactly as it was persisted.
    let replay: Vec<_> = get_session_messages(&manifest)?
        .into_iter()
        .map(|stored| (stored.role, stored.content))
        .collect();
    assert_eq!(replay.len(), 2, "summary + post-boundary turn: {replay:?}");
    assert!(replay[0].0 == "user" && replay[0].1.contains(SUMMARY));
    assert_eq!(
        replay[1],
        ("assistant".to_owned(), "Spans wired".to_owned())
    );
    let next = prompt(56);
    let wire = turn(&restored, &session_id, &next, "ok", 1_500).await?;
    assert_eq!(captured.lock().map_err(|_| "poisoned")?.len(), 1);
    let messages = wire["messages"].as_array().ok_or("messages")?;
    let text = |message: &serde_json::Value| serde_json::to_string(&message["content"]);
    let position = |needle: &str| -> Result<Vec<usize>, serde_json::Error> {
        let mut found = Vec::new();
        for (index, message) in messages.iter().enumerate() {
            if text(message)?.contains(needle) {
                found.push(index);
            }
        }
        Ok(found)
    };
    let (summary, reply, latest) = (
        position(SUMMARY)?,
        position("Spans wired")?,
        position(&next)?,
    );
    assert_eq!(
        reply.len(),
        1,
        "post-boundary reply sent once: {messages:?}"
    );
    assert_eq!(messages[reply[0]]["role"], "assistant");
    assert_eq!(latest, [messages.len() - 1]);
    assert!(summary.len() == 1 && summary[0] < reply[0] && reply[0] < latest[0]);
    let sent = serde_json::to_string(messages)?;
    assert!((0..=55).all(|index| !sent.contains(&prompt(index))));
    Ok(())
}
