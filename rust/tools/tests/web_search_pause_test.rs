use codelet_common::web_search::WebSearchAction;

#[test]
fn test_open_page_with_pause_shows_visible_browser() -> anyhow::Result<()> {
    let action = WebSearchAction::OpenPage {
        url: Some("https://example.com".to_string()),
        headless: false,
        pause: true,
    };

    let WebSearchAction::OpenPage {
        url,
        headless,
        pause,
    } = &action
    else {
        anyhow::bail!("Expected OpenPage action");
    };

    assert_eq!(url.as_deref(), Some("https://example.com"));
    assert!(!headless, "headless should be false for visible browser");
    assert!(*pause, "pause should be true");

    let action_json = serde_json::to_string(&action)?;
    assert!(
        action_json.contains("\"pause\":true"),
        "WebSearchAction::OpenPage should have pause:true. Got: {action_json}"
    );

    Ok(())
}

#[test]
fn test_pause_with_headless_auto_overrides() -> anyhow::Result<()> {
    let action = WebSearchAction::OpenPage {
        url: Some("https://example.com".to_string()),
        headless: true,
        pause: true,
    };

    let action_json = serde_json::to_string(&action)?;
    assert!(
        action_json.contains("\"pause\":true"),
        "WebSearchAction::OpenPage should have pause:true. Got: {action_json}"
    );

    Ok(())
}

#[test]
fn test_capture_screenshot_with_pause() -> anyhow::Result<()> {
    let action = WebSearchAction::CaptureScreenshot {
        url: Some("https://example.com".to_string()),
        output_path: None,
        full_page: Some(false),
        headless: false,
        pause: true,
    };

    let action_json = serde_json::to_string(&action)?;
    assert!(
        action_json.contains("\"pause\":true"),
        "WebSearchAction::CaptureScreenshot should have pause:true. Got: {action_json}"
    );

    Ok(())
}
