// Feature: spec/features/websearch-tool-pause-integration.feature
//! PAUSE-001 / BUG-556 — the WebSearch pause: a paused page action runs a
//! visible browser and asks the session's pause handler after the page has
//! loaded. The decision (`WebSearchAction::browser_headless`) and the pause
//! contract (`page_pause_request`, `wait_for_user`) are tested without a
//! browser; a real window and its page need a display.

use codelet_common::web_search::WebSearchAction;
use codelet_tools::web_search::{page_pause_request, wait_for_user};
use codelet_tools::{set_pause_handler, PauseKind, PauseRequest, PauseResponse};
use serial_test::serial;
use std::sync::{Arc, Mutex};

/// Register a handler for a fresh session that records every request and
/// answers `answer`.
fn session_answering(answer: PauseResponse) -> (uuid::Uuid, Arc<Mutex<Vec<PauseRequest>>>) {
    let session = uuid::Uuid::new_v4();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let record = Arc::clone(&seen);
    set_pause_handler(
        session,
        Some(Arc::new(move |request: PauseRequest| {
            if let Ok(mut requests) = record.lock() {
                requests.push(request);
            }
            answer
        })),
    );
    (session, seen)
}

fn requests(seen: &Mutex<Vec<PauseRequest>>) -> anyhow::Result<Vec<PauseRequest>> {
    Ok(seen
        .lock()
        .map_err(|_| anyhow::anyhow!("pause log poisoned"))?
        .clone())
}

// Scenario: OpenPage with pause shows visible browser
#[test]
#[serial]
fn open_page_with_pause_shows_visible_browser() -> anyhow::Result<()> {
    // @step Given the agent calls WebSearchTool OpenPage on "https://example.com" with pause set to true
    let action = WebSearchAction::OpenPage {
        url: Some("https://example.com".to_string()),
        headless: false,
        pause: true,
    };
    let (session, seen) = session_answering(PauseResponse::Resumed);
    // @step When the tool prepares the page action
    let headless = action.browser_headless();
    let pause = page_pause_request(&action);
    // @step Then the browser runs visible
    assert_eq!(headless, Some(false));
    // @step And the pause handler is asked to continue a "WebSearch" pause saying "Page loaded: https://example.com"
    // @step When the user presses Enter
    let interrupted = wait_for_user(session, pause);
    let asked = requests(&seen)?;
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].kind, PauseKind::Continue);
    assert_eq!(asked[0].tool_name, "WebSearch");
    assert_eq!(asked[0].message, "Page loaded: https://example.com");
    // @step Then the tool goes on to return the page content
    assert!(!interrupted);
    set_pause_handler(session, None);
    Ok(())
}

// Scenario: Pause with headless true auto-overrides to visible
#[test]
fn pause_with_headless_true_auto_overrides_to_visible() -> anyhow::Result<()> {
    // @step Given the agent calls OpenPage with pause true and headless true
    let action = WebSearchAction::OpenPage {
        url: Some("https://example.com".to_string()),
        headless: true,
        pause: true,
    };
    // @step When the tool prepares the page action
    let headless = action.browser_headless();
    // @step Then headless should be automatically overridden to false
    // @step And the browser runs visible
    assert_eq!(headless, Some(false));
    let unpaused = WebSearchAction::OpenPage {
        url: Some("https://example.com".to_string()),
        headless: true,
        pause: false,
    };
    assert_eq!(unpaused.browser_headless(), Some(true));
    assert!(page_pause_request(&unpaused).is_none());
    Ok(())
}

// Scenario: CaptureScreenshot with pause allows interaction
#[test]
#[serial]
fn capture_screenshot_with_pause_allows_interaction() -> anyhow::Result<()> {
    // @step Given the agent calls CaptureScreenshot on "https://example.com" with pause set to true
    let action = WebSearchAction::CaptureScreenshot {
        url: Some("https://example.com".to_string()),
        output_path: None,
        full_page: None,
        headless: true,
        pause: true,
    };
    let (session, seen) = session_answering(PauseResponse::Resumed);
    // @step When the tool prepares the page action
    let headless = action.browser_headless();
    let pause = page_pause_request(&action);
    // @step Then the browser runs visible
    assert_eq!(headless, Some(false));
    // @step And the pause invites the user to interact with the page and press Enter to capture the screenshot
    let message = pause
        .as_ref()
        .map(|request| request.message.clone())
        .unwrap_or_default();
    assert_eq!(
        message,
        "Page loaded at https://example.com. Interact with the page, then press Enter to capture screenshot."
    );
    // @step When the user presses Enter
    let interrupted = wait_for_user(session, pause);
    assert_eq!(requests(&seen)?.len(), 1);
    // @step Then the tool goes on to capture the screenshot
    assert!(!interrupted);
    let (interrupting, _) = session_answering(PauseResponse::Interrupted);
    assert!(wait_for_user(interrupting, page_pause_request(&action)));
    set_pause_handler(session, None);
    set_pause_handler(interrupting, None);
    Ok(())
}
