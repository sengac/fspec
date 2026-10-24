//! Feature: spec/features/cached-browser-recovery.feature

mod chrome_browser {
    pub use codelet_tools::ChromeError;
}

fn synchronization_error(error: impl std::fmt::Display) -> ChromeError {
    ChromeError::LaunchError(error.to_string())
}
#[path = "../src/browser_cache.rs"]
mod browser_cache;

use browser_cache::BrowserCache;
use codelet_tools::ChromeError;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc, Arc, Barrier, Mutex,
};

struct Browser {
    headless: bool,
}

fn create(headless: bool, _: Option<&Browser>) -> Result<Arc<Browser>, ChromeError> {
    Ok(Arc::new(Browser { headless }))
}

#[test]
fn scenario_reuse_a_healthy_browser() -> Result<(), ChromeError> {
    // @step Given a healthy cached browser
    let cache = BrowserCache::new();
    let original = cache.get(Some(true), None, |_| true, create)?;
    // @step When another operation requests the same browser mode
    let reused = cache.get(
        Some(true),
        None,
        |_| true,
        |_, _| Err(ChromeError::LaunchError("must not create".into())),
    )?;
    // @step Then the same browser is reused
    assert!(Arc::ptr_eq(&original, &reused));
    assert!(cache.clear().is_some());
    assert!(cache.clear().is_none());
    Ok(())
}

#[test]
fn scenario_bound_recovery_when_the_replacement_also_fails() -> Result<(), ChromeError> {
    // @step Given a browser operation fails because its transport is closed
    let cache = BrowserCache::new();
    let calls = AtomicUsize::new(0);
    let creations = AtomicUsize::new(0);
    // @step When the replacement operation also fails
    let result: Result<(), _> = cache.run(
        None,
        |_| true,
        |mode, previous| {
            creations.fetch_add(1, Ordering::SeqCst);
            create(mode, previous)
        },
        |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            Err(ChromeError::ScreenshotError("connection is closed".into()))
        },
    );
    // @step Then the failure is returned after exactly one retry
    assert!(matches!(result, Err(ChromeError::ScreenshotError(_))));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(creations.load(Ordering::SeqCst), 2);
    let result: Result<(), _> = cache.run(
        None,
        |_| true,
        |_, _| Err(ChromeError::LaunchError("replacement unavailable".into())),
        |_| Err(ChromeError::ConnectionError("connection closed".into())),
    );
    assert!(matches!(result, Err(ChromeError::LaunchError(_))));
    Ok(())
}

#[test]
fn scenario_preserve_healthy_browsers_on_page_failures() -> Result<(), ChromeError> {
    // @step Given a healthy cached browser
    let cache = BrowserCache::new();
    let original = cache.get(None, None, |_| true, create)?;
    // @step When an operation fails with a page or script error
    for error in [
        ChromeError::EvaluationError("script rejected".into()),
        ChromeError::NavigationError("invalid URL".into()),
        ChromeError::Timeout,
        ChromeError::IoError("connection closed in file name".into()),
    ] {
        let error = Mutex::new(Some(error));
        let result: Result<(), _> = cache.run(
            None,
            |_| true,
            |_, _| Err(ChromeError::LaunchError("must not create".into())),
            |_| {
                Err(error
                    .lock()
                    .map_err(synchronization_error)?
                    .take()
                    .ok_or_else(|| synchronization_error("operation must be called once"))?)
            },
        );
        // @step Then the error is returned without replacing the browser
        assert!(result.is_err());
        assert!(Arc::ptr_eq(
            &original,
            &cache.get(None, None, |_| true, create)?
        ));
    }
    Ok(())
}

#[test]
fn scenario_concurrent_stale_callers_share_one_replacement() -> Result<(), ChromeError> {
    // @step Given two operations hold the same failed cached browser
    let cache = BrowserCache::new();
    let original = cache.get(None, None, |_| true, create)?;
    let barrier = Barrier::new(2);
    let (sent, received) = mpsc::channel();
    let receiver = Mutex::new(received);
    let creations = AtomicUsize::new(0);
    let factory = |mode, previous: Option<&Browser>| {
        creations.fetch_add(1, Ordering::SeqCst);
        create(mode, previous)
    };
    // @step When both operations request recovery
    let results = std::thread::scope(|scope| {
        let first = scope.spawn(|| {
            cache.run(
                None,
                |_| true,
                factory,
                |browser| {
                    if Arc::ptr_eq(&browser, &original) {
                        barrier.wait();
                        return Err(ChromeError::TabError("connection is closed".into()));
                    }
                    sent.send(()).map_err(synchronization_error)?;
                    Ok(browser)
                },
            )
        });
        let second = scope.spawn(|| {
            cache.run(
                None,
                |_| true,
                factory,
                |browser| {
                    if Arc::ptr_eq(&browser, &original) {
                        barrier.wait();
                        receiver
                            .lock()
                            .map_err(synchronization_error)?
                            .recv()
                            .map_err(synchronization_error)?;
                        return Err(ChromeError::TabError("connection is closed".into()));
                    }
                    Ok(browser)
                },
            )
        });
        Ok::<_, ChromeError>((
            first
                .join()
                .map_err(|_| synchronization_error("first worker failed"))??,
            second
                .join()
                .map_err(|_| synchronization_error("second worker failed"))??,
        ))
    })?;
    // @step Then both use one healthy replacement
    let (first, second) = results;
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(creations.load(Ordering::SeqCst), 1);
    // @step And a late stale failure does not evict that replacement
    assert!(Arc::ptr_eq(
        &first,
        &cache.get(None, None, |_| true, create)?
    ));
    Ok(())
}

#[test]
fn scenario_preserve_the_requested_browser_mode_during_recovery() -> Result<(), ChromeError> {
    // @step Given a failed cached browser in visible mode
    let cache = BrowserCache::new();
    let original = cache.get(Some(false), None, |_| true, create)?;
    // @step When the operation recovers
    let recovered = cache.run(
        Some(false),
        |_| true,
        create,
        |browser| {
            if Arc::ptr_eq(&browser, &original) {
                return Err(ChromeError::TabError("connection is closed".into()));
            }
            Ok(browser)
        },
    )?;
    // @step Then the replacement remains in visible mode
    assert!(!recovered.headless);
    Ok(())
}
