//! Feature: spec/features/completion-contract-dispatch-arming.feature

const HELPER: &str = "sync_completion_contract_for_user_turn";
const RECOVERY_HELPER: &str = "sync_completion_contract_for_recovery";
const LOCK: &str = "let mut inner_session = session.inner.lock().await";
const OUTPUT: &str = "BackgroundOutput::with_provider";

fn code(source: &str) -> String {
    source
        .lines()
        .map(|line| line.split_once("//").map_or(line, |(code, _)| code))
        .collect::<Vec<_>>()
        .join("\n")
}

fn delegated_and_armed(frontend: &str, engine: &str) -> bool {
    let Some(exports) = frontend.split_once("pub use codelet_agent_loop::{") else {
        return false;
    };
    let Some((exports, _)) = exports.1.split_once("};") else {
        return false;
    };
    let exports_loop = exports.split(',').any(|item| item.trim() == "agent_loop");
    let Some(output) = engine.find(OUTPUT) else {
        return false;
    };
    let Some(lock) = engine[..output].rfind(LOCK) else {
        return false;
    };
    exports_loop
        && !frontend.contains("fn agent_loop(")
        && !frontend.contains(OUTPUT)
        && engine[lock..output].contains(&format!("session.{HELPER}("))
        && engine[lock..output].contains(&format!("session.{RECOVERY_HELPER}("))
        && [frontend, engine].iter().all(|source| {
            !source.contains("set_continue_armed(") && !source.contains("set_session_goal(")
        })
}

#[test]
fn frontends_delegate_to_one_armed_dispatch_implementation() {
    // @step Given the NAPI compatibility exports and shared agent loop source
    let frontend = code(include_str!("../../napi/src/agent_loop.rs"));
    let engine = code(include_str!("../../agent-loop/src/agent_loop.rs"));
    // @step When the shared dispatch site and frontend delegation are inspected
    let helper_home = code(include_str!(
        "../src/background_session/completion_contract.rs"
    ));
    // @step Then NAPI delegates to the shared engine that syncs completion before creating output
    assert!(delegated_and_armed(&frontend, &engine));
    assert_eq!(helper_home.matches(&format!("pub fn {HELPER}(")).count(), 1);
    assert_eq!(
        helper_home
            .matches(&format!("pub fn {RECOVERY_HELPER}("))
            .count(),
        1
    );
    // @step And neither frontend duplicates the arming block
    for source in [&frontend, &engine] {
        assert!(!source.contains("set_continue_armed("));
        assert!(!source.contains("set_session_goal("));
    }
    // @step And removing delegation or the shared sync call is detected
    assert!(!delegated_and_armed(
        &frontend.replace("pub use codelet_agent_loop::{", "pub use other::{"),
        &engine,
    ));
    assert!(!delegated_and_armed(
        &frontend,
        &engine.replace(&format!("session.{HELPER}("), "session.missing_sync("),
    ));
    assert!(!delegated_and_armed(
        &frontend,
        &engine.replace(
            &format!("session.{RECOVERY_HELPER}("),
            "session.missing_sync(",
        ),
    ));
    assert!(!delegated_and_armed(
        &format!("{frontend}\nfn agent_loop() {{}}"),
        &engine,
    ));
    assert!(!delegated_and_armed(
        &frontend,
        &format!("{engine}\nset_continue_armed(id, true);"),
    ));
}
