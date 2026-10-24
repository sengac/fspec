use super::{
    fixture::{self, Fixture, TestResult, SENTINEL, SUFFIX},
    runtime,
};
use codelet_core::{persistence::*, session_manager_handle::SessionManagerHandle};
use codelet_rpc_types::SessionId;
use codelet_sessions::{background_session::BackgroundSession, SessionManager};
use serde_json::{json, Value};
use std::{path::Path, sync::Arc};

pub async fn inspect(
    row: &str,
    root: &Path,
    background: &Arc<BackgroundSession>,
    after_live: bool,
) -> TestResult<Value> {
    let manifest = load_session(background.id)?;
    let original = super::originals::retrieve(root, &manifest)?;
    let boundary = manifest
        .compaction_commit
        .as_ref()
        .ok_or("boundary missing")?;
    let input = background.inner.lock().await.token_tracker.input_tokens;
    assert_eq!(
        input,
        if after_live { manifest.token_usage.current_context_tokens } else { boundary.input_tokens },
        "runtime must restore authoritative occupancy (admitted before sending, measured after sending)"
    );
    if row == "codex" {
        let inner = background.inner.lock().await;
        let extension = inner
            .provider_session
            .extension::<codelet_providers::codex::session::CodexSession>("codex")
            .ok_or("selected Codex session extension missing")?;
        assert_eq!(
            extension.history.generation()?,
            boundary.identity.generation,
            "retained stale checkpoint must not overwrite the authoritative boundary"
        );
    }
    let request_started = timestamp()?;
    let wire = runtime::send(row, root, background).await?;
    assert!(!wire.to_string().contains("codexContinuation"));
    Ok(
        json!({"wire":wire,"manifest":manifest,"original":original,"provider":row,
        "inputTokens":input,"admittedInputTokens":boundary.input_tokens,
        "afterLive":after_live,"pid":std::process::id(),
        "requestStarted":request_started,
        "requestFinished":timestamp()?,
        "contextSnapshot":background.committed_context_snapshot(),
        "events":background.get_buffered_output(100)}),
    )
}

pub async fn cold(
    root: &Path,
    id: uuid::Uuid,
    row: &str,
    test: &str,
    reconcile: bool,
) -> TestResult<Value> {
    cold_with_phase(root, id, row, test, reconcile, false).await
}

pub async fn cold_with_phase(
    root: &Path,
    id: uuid::Uuid,
    row: &str,
    test: &str,
    reconcile: bool,
    after_live: bool,
) -> TestResult<Value> {
    let report = root.join(format!("cold-{row}-{after_live}.json"));
    let mut command = tokio::process::Command::new(std::env::current_exe()?);
    command
        .args(["--exact", test, "--nocapture"])
        .env("CMPCT083_COLD_ROOT", root)
        .env("CMPCT083_COLD_SESSION", id.to_string())
        .env("CMPCT083_ROW", row)
        .env("CMPCT083_REPORT", &report)
        .env("CMPCT083_AFTER_LIVE", after_live.to_string())
        .kill_on_drop(true);
    if reconcile {
        command.env("CMPCT083_RECONCILE", "1");
    }
    reset_session_store_for_tests();
    reset_message_store_for_tests();
    let mut child = command.spawn()?;
    let status = tokio::time::timeout(std::time::Duration::from_secs(40), child.wait()).await??;
    reset_session_store_for_tests();
    assert!(status.success(), "fresh process failed for {row}: {status}");
    Ok(serde_json::from_slice(&std::fs::read(report)?)?)
}

pub async fn child() -> TestResult<bool> {
    let process_started = timestamp()?;
    let Some(root) = std::env::var_os("CMPCT083_COLD_ROOT") else {
        return Ok(false);
    };
    let root = std::path::PathBuf::from(root);
    fixture::environment(&root)?;
    let id = SessionId::new(std::env::var("CMPCT083_COLD_SESSION")?);
    let row = std::env::var("CMPCT083_ROW")?;
    let manager = SessionManager::new();
    manager.resume_session(&id)?;
    let background = manager.get_session(&id.value)?;
    if std::env::var_os("CMPCT083_RECONCILE").is_some() {
        super::recovery::reconcile(&root, &background).await?;
    }
    let after_live = std::env::var("CMPCT083_AFTER_LIVE").is_ok_and(|value| value == "true");
    let mut report = inspect(&row, &root, &background, after_live).await?;
    report["processStarted"] = json!(process_started);
    super::restoration::repeated(&manager, &id, &background, &root, &row).await?;
    report["afterRepeatedRestore"] = inspect(&row, &root, &background, true).await?;
    std::fs::write(
        std::env::var_os("CMPCT083_REPORT").ok_or("report path")?,
        serde_json::to_vec(&report)?,
    )?;
    Ok(true)
}

fn timestamp() -> TestResult<u64> {
    Ok(u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos(),
    )?)
}

pub fn assert_request(report: &Value, generation: usize) -> TestResult {
    let wire = report["wire"].to_string();
    for expected in [
        format!("Committed cycle {generation}"),
        "src/deep.rs".into(),
        SUFFIX.into(),
    ] {
        assert!(
            wire.contains(&expected),
            "physical request lost {expected}: {wire}"
        );
    }
    if report["wire"]["adapter"] == "stub" {
        fixture::assert_stub_suffix(&report["wire"])?;
    } else if report["wire"]["contents"].is_array() {
        fixture::assert_gemini_suffix(&report["wire"])?;
    } else if report["wire"]["input"].is_array() {
        super::codex::assert_suffix(&report["wire"])?;
    } else {
        let call = wire
            .find("call_after_snapshot")
            .ok_or("missing suffix call")?;
        let result = wire
            .rfind("call_after_snapshot")
            .ok_or("missing suffix result identity")?;
        assert!(call < result, "tool call must precede its result: {wire}");
        assert_eq!(wire.matches("call_after_snapshot").count(), 2, "{wire}");
        match report["provider"].as_str() {
            Some("anthropic") => super::suffix::assert_anthropic(&report["wire"])?,
            Some("custom") => super::suffix::assert_custom(&report["wire"])?,
            _ => super::suffix::assert_chat(&report["wire"])?,
        }
    }
    assert!(!wire.contains(SENTINEL), "covered original replayed");
    assert!(report["original"]
        .as_str()
        .is_some_and(|text| text.contains(SENTINEL)));
    assert_eq!(
        report["manifest"]["compaction_commit"]["identity"]["generation"],
        generation
    );
    assert_eq!(report["contextSnapshot"]["generation"], generation);
    assert!(report["admittedInputTokens"]
        .as_u64()
        .is_some_and(|tokens| tokens > 0));
    assert_eq!(
        report["inputTokens"],
        if report["afterLive"] == true {
            report["manifest"]["token_usage"]["current_context_tokens"].clone()
        } else {
            report["admittedInputTokens"].clone()
        }
    );
    if report["processStarted"].is_u64() {
        let repeated = report
            .get("afterRepeatedRestore")
            .ok_or("cold process omitted its repeated-restore request evidence")?;
        assert!(
            report["requestFinished"]
                .as_u64()
                .ok_or("first cold request finish")?
                < repeated["requestStarted"]
                    .as_u64()
                    .ok_or("repeated-restore request start")?,
            "repeated-restore request must follow the first cold request"
        );
        assert_eq!(report["pid"], repeated["pid"]);
        assert_eq!(
            report["manifest"]["compaction_commit"]["identity"],
            repeated["manifest"]["compaction_commit"]["identity"],
            "repeated restore changed authoritative compaction identity"
        );
        assert_request(repeated, generation)?;
    }
    Ok(())
}

pub async fn repeated(row: &str, test: &str) -> TestResult {
    if child().await? {
        return Ok(());
    }
    let fixture = Fixture::new(row).await?;
    runtime::send(row, fixture.directory.path(), &fixture.background).await?;
    super::application::cycle(&fixture, row, 1, false).await?;
    let first = load_session(fixture.background.id)?
        .compaction
        .ok_or("first DAG")?
        .summary;
    super::application::cycle(&fixture, row, 2, true).await?;
    let before = load_session(fixture.background.id)?
        .compaction_commit
        .ok_or("live boundary")?;
    let probe = cold(
        fixture.directory.path(),
        fixture.background.id,
        row,
        test,
        false,
    )
    .await?;
    assert_request(&probe, 2)?;
    let live = inspect(row, fixture.directory.path(), &fixture.background, false).await?;
    let cold = cold_with_phase(
        fixture.directory.path(),
        fixture.background.id,
        row,
        test,
        false,
        true,
    )
    .await?;
    assert!(
        live["requestStarted"].as_u64().ok_or("live request time")?
            < cold["requestStarted"].as_u64().ok_or("cold request time")?,
        "required live physical request must precede the fresh-process physical request"
    );
    assert_ne!(probe["pid"], live["pid"]);
    assert_ne!(probe["pid"], cold["pid"]);
    assert!(
        live["requestFinished"]
            .as_u64()
            .ok_or("live request completion time")?
            < cold["processStarted"]
                .as_u64()
                .ok_or("cold process start time")?,
        "required fresh process must start after the live physical request completes"
    );
    assert_ne!(first, live["manifest"]["compaction"]["summary"]);
    assert_ne!(
        live["pid"], cold["pid"],
        "must cross an OS process boundary"
    );
    assert_request(&live, 2)?;
    assert_request(&cold, 2)?;
    assert_eq!(
        live["manifest"]["compaction"]["summary"], cold["manifest"]["compaction"]["summary"],
        "fresh process changes intended committed DAG lineage"
    );
    let after = load_session(fixture.background.id)?
        .compaction_commit
        .ok_or("cold boundary")?;
    assert_eq!(
        before.identity, after.identity,
        "resume must not promote another generation"
    );
    assert_eq!(before.checkpoint, after.checkpoint);
    Ok(())
}
