use super::*;

#[test]
fn test_pause_kind_enum() {
    assert_eq!(PauseKind::Continue, PauseKind::Continue);
    assert_eq!(PauseKind::Confirm, PauseKind::Confirm);
    assert_ne!(PauseKind::Continue, PauseKind::Confirm);
}

#[test]
fn test_pause_kind_triple() {
    assert_eq!(PauseKind::Triple, PauseKind::Triple);
    assert_ne!(PauseKind::Triple, PauseKind::Continue);
    assert_ne!(PauseKind::Triple, PauseKind::Confirm);
}

#[test]
fn test_pause_response_enum() {
    assert_eq!(PauseResponse::Resumed, PauseResponse::Resumed);
    assert_eq!(PauseResponse::Approved, PauseResponse::Approved);
    assert_eq!(PauseResponse::Denied, PauseResponse::Denied);
    assert_eq!(PauseResponse::Interrupted, PauseResponse::Interrupted);
}

#[test]
fn test_pause_response_triple_variants() {
    assert_eq!(PauseResponse::AllowOnce, PauseResponse::AllowOnce);
    assert_eq!(PauseResponse::AllowSession, PauseResponse::AllowSession);
    assert_ne!(PauseResponse::AllowOnce, PauseResponse::AllowSession);
    assert_ne!(PauseResponse::AllowOnce, PauseResponse::Denied);
    assert_ne!(PauseResponse::AllowSession, PauseResponse::Approved);
}

#[test]
fn test_pause_state_from_request() {
    let request = PauseRequest {
        kind: PauseKind::Continue,
        tool_name: "Test".to_string(),
        message: "Test message".to_string(),
        details: Some("details".to_string()),
    };
    let state: PauseState = request.into();
    assert_eq!(state.kind, PauseKind::Continue);
    assert_eq!(state.tool_name, "Test");
    assert_eq!(state.message, "Test message");
    assert_eq!(state.details, Some("details".to_string()));
}

#[test]
#[serial]
fn test_no_handler_returns_resumed() {
    with_clean_handler(|sid| {
        let response = pause_for_user(
            sid,
            PauseRequest {
                kind: PauseKind::Continue,
                tool_name: "Test".to_string(),
                message: "Test".to_string(),
                details: None,
            },
        );
        assert_eq!(response, PauseResponse::Resumed);
    });
}

#[test]
#[serial]
fn test_has_pause_handler_when_not_set() {
    with_clean_handler(|sid| {
        assert!(!has_pause_handler(sid));
    });
}

#[test]
#[serial]
fn test_set_pause_handler_sets_handler() {
    with_clean_handler(|sid| {
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        let handler: PauseHandler = Arc::new(move |_| {
            called_clone.store(true, Ordering::SeqCst);
            PauseResponse::Resumed
        });

        set_pause_handler(sid, Some(handler));
        assert!(has_pause_handler(sid));

        let response = pause_for_user(
            sid,
            PauseRequest {
                kind: PauseKind::Continue,
                tool_name: "WebSearch".to_string(),
                message: "Page loaded".to_string(),
                details: None,
            },
        );

        assert_eq!(response, PauseResponse::Resumed);
        assert!(called.load(Ordering::SeqCst));

        set_pause_handler(sid, None);
        assert!(!has_pause_handler(sid));
    });
}

#[test]
#[serial]
fn test_handler_receives_correct_request() {
    with_clean_handler(|sid| {
        let handler: PauseHandler = Arc::new(|request| {
            assert_eq!(request.kind, PauseKind::Continue);
            assert_eq!(request.tool_name, "WebSearch");
            assert_eq!(request.message, "Page loaded");
            PauseResponse::Resumed
        });

        set_pause_handler(sid, Some(handler));
        pause_for_user(
            sid,
            PauseRequest {
                kind: PauseKind::Continue,
                tool_name: "WebSearch".to_string(),
                message: "Page loaded".to_string(),
                details: None,
            },
        );
    });
}

#[test]
#[serial]
fn test_handler_can_return_different_responses() {
    with_clean_handler(|sid| {
        let handler: PauseHandler = Arc::new(|_| PauseResponse::Approved);
        set_pause_handler(sid, Some(handler));
        let response = pause_for_user(
            sid,
            PauseRequest {
                kind: PauseKind::Confirm,
                tool_name: "Test".to_string(),
                message: "Test".to_string(),
                details: None,
            },
        );
        assert_eq!(response, PauseResponse::Approved);

        let handler: PauseHandler = Arc::new(|_| PauseResponse::Denied);
        set_pause_handler(sid, Some(handler));
        let response = pause_for_user(
            sid,
            PauseRequest {
                kind: PauseKind::Confirm,
                tool_name: "Test".to_string(),
                message: "Test".to_string(),
                details: None,
            },
        );
        assert_eq!(response, PauseResponse::Denied);

        let handler: PauseHandler = Arc::new(|_| PauseResponse::Interrupted);
        set_pause_handler(sid, Some(handler));
        let response = pause_for_user(
            sid,
            PauseRequest {
                kind: PauseKind::Continue,
                tool_name: "Test".to_string(),
                message: "Test".to_string(),
                details: None,
            },
        );
        assert_eq!(response, PauseResponse::Interrupted);
    });
}
