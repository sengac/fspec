use crate::chrome_browser::ChromeError;
use std::sync::{Arc, Mutex};

pub(crate) struct BrowserCache<Browser> {
    state: Mutex<(bool, Option<Arc<Browser>>)>,
}

impl<Browser> BrowserCache<Browser> {
    pub(crate) const fn new() -> Self {
        Self {
            state: Mutex::new((true, None)),
        }
    }

    pub(crate) fn get(
        &self,
        mode: Option<bool>,
        failed: Option<&Arc<Browser>>,
        healthy: impl Fn(&Browser) -> bool,
        create: impl Fn(bool, Option<&Browser>) -> Result<Arc<Browser>, ChromeError>,
    ) -> Result<Arc<Browser>, ChromeError> {
        let mut state = self.state.lock().map_err(|error| {
            ChromeError::LaunchError(format!("Failed to acquire browser lock: {error}"))
        })?;
        let headless = mode.unwrap_or(state.0);
        if let Some(browser) = &state.1 {
            let failed_current = failed.is_some_and(|failed| Arc::ptr_eq(failed, browser));
            if state.0 == headless && !failed_current && healthy(browser) {
                return Ok(Arc::clone(browser));
            }
        }
        let replacing = state.1.is_some();
        let browser = create(headless, state.1.as_deref())?;
        *state = (headless, Some(Arc::clone(&browser)));
        if replacing {
            tracing::debug!(
                headless,
                "Cached browser replaced after mode change or transport expiry"
            );
        }
        Ok(browser)
    }

    pub(crate) fn run<ResultValue>(
        &self,
        mode: Option<bool>,
        healthy: impl Fn(&Browser) -> bool,
        create: impl Fn(bool, Option<&Browser>) -> Result<Arc<Browser>, ChromeError>,
        operation: impl Fn(Arc<Browser>) -> Result<ResultValue, ChromeError>,
    ) -> Result<ResultValue, ChromeError> {
        let browser = self.get(mode, None, &healthy, &create)?;
        match operation(Arc::clone(&browser)) {
            Err(error) if is_connection_closed_error(&error) => {
                tracing::debug!("Browser transport closed during operation; retrying once");
                let replacement = self.get(mode, Some(&browser), healthy, create)?;
                let result = operation(replacement);
                if result.is_err() {
                    tracing::warn!("Browser operation failed after one transport recovery attempt");
                }
                result
            }
            result => result,
        }
    }

    pub(crate) fn clear(&self) -> Option<Arc<Browser>> {
        self.state.lock().ok()?.1.take()
    }
}

fn is_connection_closed_error(error: &ChromeError) -> bool {
    let message = match error {
        ChromeError::TabError(message)
        | ChromeError::NavigationError(message)
        | ChromeError::EvaluationError(message)
        | ChromeError::ConnectionError(message)
        | ChromeError::ScreenshotError(message) => message,
        _ => return false,
    };
    message.contains("connection is closed") || message.contains("connection closed")
}
