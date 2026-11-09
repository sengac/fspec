//! Shared helpers for rpc410_hitl_wire_protocol_parity.rs (split out by REFAC-052).

use std::sync::{Arc, OnceLock};

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use codelet_tools::request_user_input::{
    HitlOption, HitlQuestion, HitlRequest as InternalHitlRequest,
    HitlResponse as InternalHitlResponse,
};

/// BUG-397: the one data directory every test in this binary uses. The
/// data directory is process-global, so a per-test tempdir would be
/// deleted while later tests still point at it; this one lives until the
/// test process exits.
static SHARED_DATA_DIR: OnceLock<tempfile::TempDir> = OnceLock::new();

/// Point the process-global data directory at the binary's shared
/// tempdir, created and registered on first use.
fn use_shared_data_dir() {
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    SHARED_DATA_DIR.get_or_init(|| {
        let dir = tempfile::tempdir().expect("tempdir for the rpc410 test binary");
        codelet_common::set_data_directory(dir.path().to_path_buf())
            .expect("set_data_directory must accept the shared tempdir");
        dir
    });
}

/// Create a fresh BackgroundSession via the SessionManagerHandle bridge,
/// in the binary's shared data directory. The Noop hooks ensure no agent
/// loop is spawned for the session. `create_session_checked` makes a
/// failed creation name its cause instead of returning an empty id.
pub async fn fresh_session(manager: &SessionManager) -> SessionId {
    use_shared_data_dir();
    super::offline_catalog::seed_offline_catalog().expect("seed the offline model catalog");
    assert_eq!(
        manager.set_default_model("anthropic/claude-opus-4-5"),
        Ok(())
    );
    let handle: &dyn SessionManagerHandle = manager;
    handle
        .create_session_checked(None)
        .expect("create_session_checked must create the session")
}

/// One internal question with two options.
pub fn option_question(id: &str, header: &str, question: &str) -> HitlQuestion {
    HitlQuestion {
        id: id.to_string(),
        header: header.to_string(),
        question: question.to_string(),
        options: Some(vec![
            HitlOption {
                label: "Option A".to_string(),
                description: "First choice".to_string(),
            },
            HitlOption {
                label: "Option B".to_string(),
                description: "Second choice".to_string(),
            },
        ]),
    }
}

/// One internal freeform question (options: None).
pub fn freeform_question(id: &str, header: &str, question: &str) -> HitlQuestion {
    HitlQuestion {
        id: id.to_string(),
        header: header.to_string(),
        question: question.to_string(),
        options: None,
    }
}

/// A pending internal 3-question request (approach / priority / notes;
/// notes is freeform).
pub fn three_question_request() -> InternalHitlRequest {
    InternalHitlRequest {
        questions: vec![
            option_question("approach", "Approach", "Which approach do you prefer?"),
            option_question("priority", "Priority", "What is the priority?"),
            freeform_question("notes", "Notes", "Any additional notes?"),
        ],
    }
}

/// A pending internal request with a single Yes/No question.
pub fn yes_no_request() -> InternalHitlRequest {
    InternalHitlRequest {
        questions: vec![HitlQuestion {
            id: "confirm_choice".to_string(),
            header: "Confirm".to_string(),
            question: "Proceed with the plan?".to_string(),
            options: Some(vec![
                HitlOption {
                    label: "Yes".to_string(),
                    description: "Go ahead".to_string(),
                },
                HitlOption {
                    label: "No".to_string(),
                    description: "Stop here".to_string(),
                },
            ]),
        }],
    }
}

/// Spawn a std thread blocked on `wait_for_hitl_response()` for the
/// given session and return its join handle. The thread returns the
/// internal response it receives.
pub fn spawn_blocked_waiter(
    manager: &Arc<SessionManager>,
    sid: &SessionId,
) -> std::thread::JoinHandle<InternalHitlResponse> {
    let session = manager
        .get_session(&sid.value)
        .expect("session must exist for waiter");
    std::thread::spawn(move || session.wait_for_hitl_response())
}

/// Join the waiter under the deadlock guard so a broken send path fails
/// the test instead of hanging it forever. Only call this AFTER the
/// unblocking `send_hitl_response` was issued.
pub fn join_waiter(waiter: std::thread::JoinHandle<InternalHitlResponse>) -> InternalHitlResponse {
    codelet_test_helpers::deadlock_guard_blocking("wait_for_hitl_response unblocking", move || {
        waiter.join()
    })
    .expect("wait_for_hitl_response never unblocked — send_hitl_response did not deliver")
    .expect("waiter thread must not panic")
}
