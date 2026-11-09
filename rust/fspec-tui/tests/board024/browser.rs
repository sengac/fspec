use std::sync::{Arc, Mutex};

use codelet_fspec_tui::App;

use super::TestResult;

pub struct Browser {
    attempts: Arc<Mutex<Vec<String>>>,
}

impl Browser {
    pub fn new(app: &mut App, succeeds: bool, viewer_available: bool) -> Self {
        let attempts = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&attempts);
        app.set_board_attachment_launcher_for_test(Arc::new(move |url| {
            recorded
                .lock()
                .map_err(|_| "browser recorder poisoned".to_string())?
                .push(url.to_owned());
            if succeeds {
                Ok(())
            } else {
                Err("BOARD024 injected browser launch failure".to_string())
            }
        }));
        app.set_board_attachment_viewer_enabled_for_test(viewer_available);
        Self { attempts }
    }

    pub fn attempts(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(self
            .attempts
            .lock()
            .map_err(|_| "browser recorder poisoned")?
            .clone())
    }
}

pub fn contents(path: &str) -> String {
    format!("# Canonical attachment\n\nExact workspace content for {path}\n")
}

pub async fn assert_served(url: &str, path: &str) -> TestResult {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
    let response = client.get(url).send().await?;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let basename = path.rsplit('/').next().ok_or("attachment basename")?;
    let expected = codelet_attachment_viewer::markdown::viewer_template(
        basename,
        &codelet_attachment_viewer::markdown::render_markdown(&contents(path)),
    );
    assert_eq!(response.text().await?, expected);
    Ok(())
}
