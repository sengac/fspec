//! Feature: spec/features/codex-terminal-lifecycle.feature
#[path = "fixtures/prov148.rs"]
pub mod fixture;
use fixture::*;
use serde_json::json;
use std::time::Duration;

#[tokio::test]
async fn completion_finishes_before_the_socket_closes() -> TestResult {
    // @step Given a Codex stream whose socket stays open after completion
    let server = Server::start(
        vec![[text(), terminal("completed", Some(true), None)].concat()],
        true,
    )
    .await?;
    // @step When the caller consumes the stream
    let result =
        tokio::time::timeout(Duration::from_millis(700), collect(server.models()?.0)).await??;
    // @step Then one final response arrives without waiting for socket closure
    assert_eq!(result.finals, [20]);
    assert!(result.errors.is_empty());
    Ok(())
}

#[tokio::test]
async fn premature_eof_retracts_abandoned_text_before_replacement() -> TestResult {
    let server = Server::start(
        vec![
            text(),
            terminal("completed", Some(true), None).replace("Hello", "Replacement"),
        ],
        false,
    )
    .await?;
    let result =
        tokio::time::timeout(Duration::from_secs(3), collect(server.models()?.0)).await??;
    assert_eq!(result.text, "Replacement");
    assert!(result.errors.is_empty(), "{result:?}");
    assert_eq!(result.finals, [20]);
    assert_eq!(result.attempts, ["started", "aborted", "started"]);
    let requests = server.requests.lock().map_err(|_| "poisoned requests")?;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0], requests[1]);
    Ok(())
}

#[tokio::test]
async fn backend_failures_never_dispatch_buffered_tools() -> TestResult {
    // @step Given a Codex stream with tool arguments followed by failure
    let delta = event(
        json!({"type":"response.function_call_arguments.delta", "item_id":"fc_test", "delta":"{}", "output_index":1, "content_index":0,"sequence_number":1}),
    );
    // @step When failed and top-level error events arrive
    for failure in [
        terminal("failed", None, None),
        event(
            json!({"type":"error", "error":{"code":"bad_request", "message":"repair the schema"}}),
        ),
        event(json!({"type":"error", "code":"bad_request", "message":"repair the schema"})),
    ] {
        for message in ["repair the schema", "request aborted; repair the schema"] {
            let failure = failure.replace("repair the schema", message);
            let result = run([delta.clone(), tool(), failure].concat()).await?;
            // @step Then actionable codes and messages survive without tool dispatch or success
            assert_eq!(result.errors.len(), 1);
            assert!(result.errors[0].contains("bad_request"));
            assert!(result.errors[0].contains(message));
            assert_eq!(result.tools, 0);
            assert!(result.finals.is_empty());
        }
    }
    Ok(())
}

#[tokio::test]
async fn incomplete_outcomes_retain_usage_and_reject_unsafe_continuation() -> TestResult {
    // @step Given incomplete Codex responses with filtering budget and interrupted tool outcomes
    for reason in ["content_filter", "max_output_tokens", "interrupted"] {
        // @step When the caller consumes each stream
        let result =
            run([text(), tool(), terminal("incomplete", None, Some(reason))].concat()).await?;
        // @step Then each reason and usage survive without tool dispatch or success
        assert_eq!(result.usage, [20]);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains(reason));
        assert_eq!(result.tools, 0);
        assert!(result.finals.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn explicit_false_continues_text_with_a_bounded_policy() -> TestResult {
    // @step Given a completed text-only Codex response with end_turn false
    let continuing = [text(), terminal("completed", Some(false), None)].concat();
    // @step When subsequent responses complete or continue indefinitely
    for endless in [false, true] {
        let bodies = if endless {
            vec![continuing.clone(); 4]
        } else {
            vec![
                continuing.clone(),
                [text(), terminal("completed", Some(true), None)].concat(),
            ]
        };
        let server = Server::start(bodies, false).await?;
        let result = collect(server.models()?.0).await?;
        // @step Then text history is replayed and at most three follow-ups occur
        let requests = server.requests.lock().map_err(|_| "poisoned requests")?;
        assert_eq!(requests.len(), if endless { 4 } else { 2 });
        assert_eq!(requests[1]["input"][1]["role"], "assistant");
        assert_eq!(requests[1]["input"][1]["content"][0]["text"], "Hello");
        if endless {
            assert!(result.finals.is_empty());
            assert!(result.errors[0].contains("continuation limit"));
        } else {
            assert_eq!(result.finals, [40]);
            assert_eq!(result.usage, [20, 20]);
            assert_eq!(result.text, "HelloHello");
        }
    }
    Ok(())
}

#[tokio::test]
async fn interrupted_text_resumes_without_replaying_tool_effects() -> TestResult {
    // @step Given an interrupted incomplete text-only response
    let interrupted = [text(), terminal("incomplete", None, Some("interrupted"))].concat();
    // @step When the response includes replayable assistant output
    let server = Server::start(
        vec![
            interrupted,
            [text(), terminal("completed", Some(true), None)].concat(),
        ],
        false,
    )
    .await?;
    let result = collect(server.models()?.0).await?;
    // @step Then a follow-up completes without dispatching tools
    assert_eq!(result.text, "HelloHello");
    assert_eq!(result.finals, [40]);
    assert!(result.errors.is_empty());
    assert_eq!(result.tools, 0);
    assert_eq!(
        server
            .requests
            .lock()
            .map_err(|_| "poisoned requests")?
            .len(),
        2
    );
    Ok(())
}

#[tokio::test]
async fn true_absent_and_tool_responses_use_the_appropriate_fallback() -> TestResult {
    // @step Given completed Codex responses with true absent or tool-bearing false turn markers
    for (marker, tools) in [(Some(true), false), (None, false), (Some(false), true)] {
        let body = [
            text(),
            if tools { tool() } else { String::new() },
            terminal("completed", marker, None),
        ]
        .concat();
        // @step When the caller consumes each stream
        let result = run(body).await?;
        // @step Then text ends and completed tools return once to the generic loop
        assert_eq!(result.finals, [20]);
        assert_eq!(result.tools, usize::from(tools));
        assert!(result.errors.is_empty());
    }
    Ok(())
}

#[tokio::test]
async fn unknown_events_are_ignored_but_malformed_terminals_fail() -> TestResult {
    // @step Given optional unknown events and malformed known terminal events
    let unknown = event(json!({"type":"response.future_optional", "value":123}));
    // @step When the caller consumes each stream
    let result = run([unknown, terminal("completed", None, None)].concat()).await?;
    // @step Then unknown events permit completion but malformed terminals never succeed
    assert_eq!(result.finals, [20]);
    for kind in [
        "response.completed",
        "response.failed",
        "response.incomplete",
        "error",
    ] {
        let malformed = event(json!({"type":kind, "response":{}}));
        let result = run([malformed, terminal("completed", None, None)].concat()).await?;
        assert_eq!(result.errors.len(), 1);
        assert!(result.finals.is_empty());
    }
    let result = run(event(
        json!({"type":"response.completed", "response":{"status":"completed", "end_turn":"false"}}),
    ))
    .await?;
    assert_eq!(result.errors.len(), 1);
    assert!(result.finals.is_empty());
    Ok(())
}

#[tokio::test]
async fn duplicate_completion_does_not_duplicate_usage_or_tools() -> TestResult {
    // @step Given a Codex stream with repeated completed events
    let done = terminal("completed", None, None);
    // @step When the caller consumes the stream
    let result = run([tool(), done.clone(), done].concat()).await?;
    // @step Then usage tools and final response each appear once
    assert_eq!(result.finals, [20]);
    assert_eq!(result.usage, [20]);
    assert_eq!(result.tools, 1);
    Ok(())
}

#[tokio::test]
async fn ordinary_responses_retains_its_existing_eof_behavior() -> TestResult {
    // @step Given identical partial streams for Codex and ordinary Responses models
    let codex_server = Server::start(
        vec![
            text(),
            terminal("completed", Some(true), None).replace("Hello", "Replacement"),
        ],
        false,
    )
    .await?;
    let ordinary_server = Server::start(vec![text()], false).await?;
    // @step When both callers consume independently scripted streams within bounded deadlines
    let codex =
        tokio::time::timeout(Duration::from_secs(3), collect(codex_server.models()?.0)).await??;
    let ordinary =
        tokio::time::timeout(Duration::from_secs(2), collect(ordinary_server.models()?.1))
            .await??;
    // @step Then Codex recovers with replacement text while ordinary Responses rejects missing terminal evidence without retrying
    assert!(codex.errors.is_empty(), "{codex:?}");
    assert_eq!(codex.text, "Replacement");
    assert_eq!(codex.attempts, ["started", "aborted", "started"]);
    assert_eq!(codex.finals.len(), 1);
    assert_eq!(ordinary.errors.len(), 1, "{ordinary:?}");
    assert!(
        ordinary.errors[0].contains("missing Responses terminal evidence"),
        "{ordinary:?}"
    );
    assert_eq!(ordinary.text, "Hello");
    assert!(ordinary.attempts.is_empty());
    assert!(ordinary.finals.is_empty());
    assert_eq!(
        codex_server
            .requests
            .lock()
            .map_err(|_| "poisoned requests")?
            .len(),
        2
    );
    assert_eq!(
        ordinary_server
            .requests
            .lock()
            .map_err(|_| "poisoned requests")?
            .len(),
        1
    );
    Ok(())
}
