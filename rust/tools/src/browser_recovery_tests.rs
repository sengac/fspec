//! Feature: spec/features/idle-browser-transport-recovery.feature

use super::*;
use crate::chrome_browser::ChromeConfig;
use serial_test::serial;
use std::time::Duration;

struct BrowserCleanup;

impl Drop for BrowserCleanup {
    fn drop(&mut self) {
        shutdown_browser();
    }
}

fn get_browser() -> Result<Arc<ChromeBrowser>, ChromeError> {
    BROWSER.get(None, None, browser_is_healthy, create_browser_with_headless)
}

fn install(config: ChromeConfig) -> Result<Arc<ChromeBrowser>, ChromeError> {
    shutdown_browser();
    let browser = Arc::new(ChromeBrowser::new(config)?);
    BROWSER.get(
        Some(browser.config().headless),
        None,
        browser_is_healthy,
        |_, _| Ok(Arc::clone(&browser)),
    )?;
    Ok(browser)
}

fn config() -> ChromeConfig {
    ChromeConfig {
        ws_url: None,
        timeout: Duration::from_secs(30),
        headless: true,
        user_agent: None,
        ..ChromeConfig::default()
    }
}

#[test]
#[serial]
#[ignore = "requires installed Chrome"]
fn scenario_recover_an_idle_expired_launched_browser() -> Result<(), ChromeError> {
    let _cleanup = BrowserCleanup;
    // @step Given a cached launched browser whose idle transport has expired
    let original = install(ChromeConfig {
        timeout: Duration::from_secs(1),
        ..config()
    })?;
    std::thread::sleep(Duration::from_secs(3));
    let error = original
        .new_tab()
        .err()
        .ok_or_else(|| ChromeError::LaunchError("expired transport must fail".into()))?;
    assert!(error
        .to_string()
        .contains("underlying connection is closed"));
    // @step When the next operation requests a browser
    let recovered = get_browser()?;
    // @step Then a healthy replacement is returned with the same configuration
    assert!(
        !Arc::ptr_eq(&original, &recovered),
        "expired cache was reused: {error}"
    );
    assert_eq!(recovered.config().timeout, original.config().timeout);
    let tab = recovered.new_tab()?;
    recovered.cleanup_tab(&tab);
    assert!(Arc::ptr_eq(&recovered, &with_browser_retry(Ok)?));
    shutdown_browser();
    Ok(())
}

#[test]
#[serial]
#[ignore = "requires installed Chrome"]
fn scenario_recover_an_idle_expired_connected_browser() -> Result<(), ChromeError> {
    let _cleanup = BrowserCleanup;
    // @step Given a cached connection to an externally owned browser whose idle transport has expired
    let owner = ChromeBrowser::new(config())?;
    let ws_url = owner.browser().get_ws_url();
    let original = install(ChromeConfig {
        ws_url: Some(ws_url.clone()),
        timeout: Duration::from_secs(1),
        ..config()
    })?;
    std::thread::sleep(Duration::from_secs(3));
    let error = original
        .new_tab()
        .err()
        .ok_or_else(|| ChromeError::ConnectionError("expired transport must fail".into()))?;
    assert!(error
        .to_string()
        .contains("underlying connection is closed"));
    // @step When the next operation requests a browser
    let recovered = get_browser()?;
    // @step Then a healthy connection is returned without stopping the external browser
    assert!(!Arc::ptr_eq(&original, &recovered));
    assert_eq!(recovered.config().ws_url.as_deref(), Some(ws_url.as_str()));
    assert!(recovered.browser().get_version().is_ok());
    assert!(Arc::ptr_eq(&recovered, &with_browser_mode(true, Ok)?));
    shutdown_browser();
    drop(recovered);
    drop(original);
    assert!(owner.browser().get_version().is_ok());
    Ok(())
}
