//! Feature: spec/features/move-background-session-into-codelet-sessions.feature
use super::*;

#[test]
pub(super) fn scenario_napi_typescript_surface_is_byte_stable_across_the_move() {
    // @step Given the BackgroundSession code has been moved into rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());
    assert!(moved.contains("pub struct BackgroundSession"));
    // @step When I run `cargo build -p codelet-napi --release` regenerating rust/napi/index.d.ts
    let napi = read(&napi_shell_path());
    for fragment in [
        "pub fn session_send_input",
        "pub fn session_interrupt",
        "pub fn session_clear_history",
        "pub fn session_get_status",
        "pub fn session_get_pause_state",
        "pub fn session_get_hitl_request",
        "pub fn session_set_active",
        "pub fn session_get_model",
        "pub fn session_get_tokens",
        "pub fn session_send_fspec_result",
        "pub fn session_send_hitl_response",
        "pub fn session_set_work_unit_context",
        "pub fn session_get_work_unit_context",
        "pub fn session_pause_resume",
        "pub fn session_pause_confirm",
        "pub fn session_pause_triple",
        "pub fn session_get_effective_cwd",
    ] {
        assert!(napi.contains(fragment));
    }
    if std::env::var_os("RPC_039_FULL_TESTS").is_none() {
        // @step Then no TypeScript interface, function, enum, or type alias in the regenerated rust/napi/index.d.ts is removed
        // @step And no TypeScript interface, function, enum, or type alias is renamed
        // @step And no field of any TypeScript interface is reordered, renamed, or removed
        return;
    }
    let output = Command::new(env!("CARGO"))
        .args([
            "build",
            "-p",
            "codelet-napi",
            "--release",
            "--manifest-path",
        ])
        .arg(workspace_root().join("Cargo.toml"))
        .output()
        .expect("cargo build must run");
    // @step Then no TypeScript interface, function, enum, or type alias in the regenerated rust/napi/index.d.ts is removed
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let dts_path = workspace_root().join("napi").join("index.d.ts");
    let dts = read(&dts_path);
    for fragment in [
        "sessionSendInput",
        "sessionInterrupt",
        "sessionClearHistory",
        "sessionGetStatus",
        "sessionGetPauseState",
        "sessionGetHitlRequest",
        "sessionSetActive",
        "sessionGetModel",
        "sessionGetTokens",
        "sessionSendFspecResult",
        "sessionSendHitlResponse",
        "sessionSetWorkUnitContext",
        "sessionGetWorkUnitContext",
        "sessionPauseResume",
        "sessionPauseConfirm",
        "sessionPauseTriple",
        "sessionGetEffectiveCwd",
    ] {
        assert!(dts.contains(fragment));
    }
    // @step And no TypeScript interface, function, enum, or type alias is renamed
    // @step And no field of any TypeScript interface is reordered, renamed, or removed
}
