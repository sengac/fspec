use codelet_tools::{
    has_pause_handler, pause_for_user, set_pause_handler, PauseHandler, PauseKind, PauseRequest,
    PauseResponse, PauseState,
};
use serial_test::serial;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

fn with_clean_handler<T>(f: impl FnOnce(uuid::Uuid) -> T) -> T {
    let sid = uuid::Uuid::new_v4();
    set_pause_handler(sid, None);
    let result = f(sid);
    set_pause_handler(sid, None);
    result
}

fn mutex_lock<T>(mutex: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, anyhow::Error> {
    mutex
        .lock()
        .map_err(|e: PoisonError<_>| anyhow::anyhow!("Mutex poisoned: {e}"))
}

fn wait_for_response<'a>(
    cvar: &'a Condvar,
    guard: std::sync::MutexGuard<'a, Option<PauseResponse>>,
) -> Result<std::sync::MutexGuard<'a, Option<PauseResponse>>, anyhow::Error> {
    cvar.wait(guard)
        .map_err(|e: PoisonError<_>| anyhow::anyhow!("Condvar wait failed: {e}"))
}

#[test]
#[serial]
fn test_no_handler_returns_resumed_immediately() {
    with_clean_handler(|sid| {
        let response = pause_for_user(sid, PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".to_string(),
            message: "Page loaded".to_string(),
            details: None,
        });

        assert_eq!(
            response,
            PauseResponse::Resumed,
            "Should return Resumed when no handler is set"
        );
    });
}

#[test]
#[serial]
fn test_handler_is_invoked_with_request() {
    with_clean_handler(|sid| {
        let handler_called = Arc::new(AtomicBool::new(false));
        let handler_called_clone = handler_called.clone();

        let handler: PauseHandler = Arc::new(move |request: PauseRequest| {
            handler_called_clone.store(true, Ordering::SeqCst);

            assert_eq!(request.kind, PauseKind::Continue);
            assert_eq!(request.tool_name, "WebSearch");
            assert_eq!(request.message, "Page loaded");
            assert!(request.details.is_none());

            PauseResponse::Resumed
        });

        set_pause_handler(sid, Some(handler));
        let response = pause_for_user(sid, PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".to_string(),
            message: "Page loaded".to_string(),
            details: None,
        });

        assert!(
            handler_called.load(Ordering::SeqCst),
            "Handler should have been called"
        );
        assert_eq!(response, PauseResponse::Resumed);
    });
}

#[test]
#[serial]
fn test_handler_returns_approved() {
    with_clean_handler(|sid| {
        let handler: PauseHandler = Arc::new(|_| PauseResponse::Approved);
        set_pause_handler(sid, Some(handler));

        let response = pause_for_user(sid, PauseRequest {
            kind: PauseKind::Confirm,
            tool_name: "Bash".to_string(),
            message: "Dangerous command".to_string(),
            details: Some("rm -rf /".to_string()),
        });

        assert_eq!(response, PauseResponse::Approved);
    });
}

#[test]
#[serial]
fn test_handler_returns_denied() {
    with_clean_handler(|sid| {
        let handler: PauseHandler = Arc::new(|_| PauseResponse::Denied);
        set_pause_handler(sid, Some(handler));

        let response = pause_for_user(sid, PauseRequest {
            kind: PauseKind::Confirm,
            tool_name: "Bash".to_string(),
            message: "Dangerous command".to_string(),
            details: Some("rm -rf /".to_string()),
        });

        assert_eq!(response, PauseResponse::Denied);
    });
}

#[test]
#[serial]
fn test_handler_returns_interrupted() {
    with_clean_handler(|sid| {
        let handler: PauseHandler = Arc::new(|_| PauseResponse::Interrupted);
        set_pause_handler(sid, Some(handler));

        let response = pause_for_user(sid, PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".to_string(),
            message: "Page loaded".to_string(),
            details: None,
        });

        assert_eq!(response, PauseResponse::Interrupted);
    });
}

#[test]
#[serial]
fn test_handler_can_block_and_resume() -> anyhow::Result<()> {
    with_clean_handler(|sid| {
        let response_signal: Arc<(Mutex<Option<PauseResponse>>, Condvar)> =
            Arc::new((Mutex::new(None), Condvar::new()));
        let signal_clone = Arc::clone(&response_signal);

        let handler: PauseHandler = Arc::new(move |_request: PauseRequest| {
            let (lock, cvar) = &*signal_clone;
            let Ok(mut response) = mutex_lock(lock) else {
                return PauseResponse::Interrupted;
            };

            while response.is_none() {
                let Ok(new_response) = wait_for_response(cvar, response) else {
                    return PauseResponse::Interrupted;
                };
                response = new_response;
            }

            response.take().unwrap_or(PauseResponse::Interrupted)
        });

        let signal_for_thread = Arc::clone(&response_signal);
        let signaler_thread = thread::spawn(move || -> anyhow::Result<()> {
            thread::sleep(Duration::from_millis(50));

            let (lock, cvar) = &*signal_for_thread;
            *mutex_lock(lock)? = Some(PauseResponse::Resumed);
            cvar.notify_one();
            Ok(())
        });

        set_pause_handler(sid, Some(handler));
        let response = pause_for_user(sid, PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".to_string(),
            message: "Page loaded".to_string(),
            details: None,
        });

        signaler_thread
            .join()
            .map_err(|_| anyhow::anyhow!("Signaler thread panicked"))??;

        assert_eq!(
            response,
            PauseResponse::Resumed,
            "Tool should have received Resumed response"
        );
        Ok(())
    })
}

#[test]
#[serial]
fn test_handler_can_be_interrupted() -> anyhow::Result<()> {
    with_clean_handler(|sid| {
        let response_signal: Arc<(Mutex<Option<PauseResponse>>, Condvar)> =
            Arc::new((Mutex::new(None), Condvar::new()));
        let signal_clone = Arc::clone(&response_signal);

        let handler: PauseHandler = Arc::new(move |_request: PauseRequest| {
            let (lock, cvar) = &*signal_clone;
            let Ok(mut response) = mutex_lock(lock) else {
                return PauseResponse::Interrupted;
            };

            while response.is_none() {
                let Ok(new_response) = wait_for_response(cvar, response) else {
                    return PauseResponse::Interrupted;
                };
                response = new_response;
            }

            response.take().unwrap_or(PauseResponse::Interrupted)
        });

        let signal_for_thread = Arc::clone(&response_signal);
        let signaler_thread = thread::spawn(move || -> anyhow::Result<()> {
            thread::sleep(Duration::from_millis(50));

            let (lock, cvar) = &*signal_for_thread;
            *mutex_lock(lock)? = Some(PauseResponse::Interrupted);
            cvar.notify_one();
            Ok(())
        });

        set_pause_handler(sid, Some(handler));
        let response = pause_for_user(sid, PauseRequest {
            kind: PauseKind::Continue,
            tool_name: "WebSearch".to_string(),
            message: "Page loaded".to_string(),
            details: None,
        });

        signaler_thread
            .join()
            .map_err(|_| anyhow::anyhow!("Signaler thread panicked"))??;

        assert_eq!(response, PauseResponse::Interrupted);
        Ok(())
    })
}

#[test]
#[serial]
fn test_has_pause_handler() {
    with_clean_handler(|sid| {
        assert!(
            !has_pause_handler(sid),
            "Should return false when no handler set"
        );

        let handler: PauseHandler = Arc::new(|_| PauseResponse::Resumed);
        set_pause_handler(sid, Some(handler));
        assert!(has_pause_handler(sid), "Should return true when handler is set");

        set_pause_handler(sid, None);
        assert!(
            !has_pause_handler(sid),
            "Should return false after clearing handler"
        );
    });
}

#[test]
fn test_pause_state_from_request() {
    let request = PauseRequest {
        kind: PauseKind::Confirm,
        tool_name: "Bash".to_string(),
        message: "Dangerous command".to_string(),
        details: Some("rm -rf /important".to_string()),
    };

    let state: PauseState = request.into();

    assert_eq!(state.kind, PauseKind::Confirm);
    assert_eq!(state.tool_name, "Bash");
    assert_eq!(state.message, "Dangerous command");
    assert_eq!(state.details, Some("rm -rf /important".to_string()));
}

#[test]
#[serial]
fn test_confirm_pause_with_details() -> anyhow::Result<()> {
    with_clean_handler(|sid| {
        let captured_details: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let captured_clone = Arc::clone(&captured_details);

        let handler: PauseHandler = Arc::new(move |request: PauseRequest| {
            if let Ok(mut guard) = mutex_lock(&captured_clone) {
                *guard = request.details;
            }
            PauseResponse::Approved
        });

        set_pause_handler(sid, Some(handler));
        pause_for_user(sid, PauseRequest {
            kind: PauseKind::Confirm,
            tool_name: "Bash".to_string(),
            message: "Dangerous command".to_string(),
            details: Some("sudo rm -rf /*".to_string()),
        });

        let captured = mutex_lock(&captured_details)?;
        assert_eq!(*captured, Some("sudo rm -rf /*".to_string()));
        Ok(())
    })
}

#[test]
#[serial]
fn test_multiple_pause_calls() {
    with_clean_handler(|sid| {
        let call_count = Arc::new(AtomicU32::new(0));
        let count_clone = Arc::clone(&call_count);

        let handler: PauseHandler = Arc::new(move |_| {
            count_clone.fetch_add(1, Ordering::SeqCst);
            PauseResponse::Resumed
        });

        set_pause_handler(sid, Some(handler));

        for _ in 0..3 {
            let response = pause_for_user(sid, PauseRequest {
                kind: PauseKind::Continue,
                tool_name: "Test".to_string(),
                message: "Test".to_string(),
                details: None,
            });
            assert_eq!(response, PauseResponse::Resumed);
        }

        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    });
}

#[test]
#[serial]
fn test_handler_replacement() {
    with_clean_handler(|sid| {
        let first_handler: PauseHandler = Arc::new(|_| PauseResponse::Approved);
        let second_handler: PauseHandler = Arc::new(|_| PauseResponse::Denied);

        set_pause_handler(sid, Some(first_handler));
        let response = pause_for_user(sid, PauseRequest {
            kind: PauseKind::Confirm,
            tool_name: "Test".to_string(),
            message: "First".to_string(),
            details: None,
        });
        assert_eq!(response, PauseResponse::Approved);

        set_pause_handler(sid, Some(second_handler));
        let response = pause_for_user(sid, PauseRequest {
            kind: PauseKind::Confirm,
            tool_name: "Test".to_string(),
            message: "Second".to_string(),
            details: None,
        });
        assert_eq!(response, PauseResponse::Denied);
    });
}
