//! Feature: spec/features/board-card-enter-chooser-and-complete-details.feature

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use serde_json::json;

use super::*;

async fn activate(app: &mut App) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    assert!(app
        .handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE
        )))
        .is_consumed());
    let mut paths = Vec::new();
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        for _ in 0..128 {
            while let Some(action) = app.try_recv_action() {
                if let Action::OpenAttachment(path) = &action {
                    paths.push(path.clone());
                }
                app.dispatch(action);
            }
            let Some(task) = app.next_pending_task() else {
                return Ok::<(), Box<dyn std::error::Error>>(());
            };
            task.await?;
        }
        Err("attachment activation did not settle".into())
    })
    .await??;
    Ok(paths)
}

#[tokio::test]
#[serial_test::serial]
async fn attachments_retain_exact_paths_and_unavailable_viewer_failures_retain_details(
) -> TestResult {
    let _isolation = Isolation::new()?;
    // @step Given details with duplicate attachment basenames and Unicode spaces hash and percent paths
    let paths = [
        "spec/attachments/A/a 日本語 #%.md",
        "spec/attachments/B/a 日本語 #%.md",
        "spec/attachments/A/literal%20name.md",
        "spec/attachments/A/literal name.md",
    ];
    for (websocket, outcome) in [
        (false, "success"),
        (false, "unavailable"),
        (false, "browser failure"),
        (true, "success"),
        (true, "unavailable"),
        (true, "browser failure"),
    ] {
        let mut fixture = Canonical::new(websocket).await?;
        assert_ne!(std::env::current_dir()?, fixture.root.path());
        let browser = browser::Browser::new(
            &mut fixture.app,
            outcome != "browser failure",
            outcome != "unavailable",
        );
        let wrong_root = codelet_test_helpers::setup_temp_directory()?;
        for path in paths {
            let full_path = fixture.root.path().join(path);
            std::fs::create_dir_all(full_path.parent().ok_or("attachment parent")?)?;
            std::fs::write(full_path, browser::contents(path))?;
            let wrong_path = wrong_root.path().join(path);
            std::fs::create_dir_all(wrong_path.parent().ok_or("wrong-root parent")?)?;
            std::fs::write(wrong_path, format!("WRONG ROOT {path}"))?;
        }
        let wrong_viewer =
            codelet_attachment_viewer::start_viewer(wrong_root.path().to_path_buf()).await?;
        fixture
            .app
            .set_viewer_port_for_test(Some(wrong_viewer.port));
        fixture.record["attachments"] = json!(paths);
        fixture.save()?;
        open_details(&mut fixture.app).await?;
        // @step When the user selects each attachment with Tab Shift-Tab and Enter
        for path in paths {
            focus_label(&mut fixture.app, path);
            assert!(event(
                &mut fixture.app,
                Event::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT))
            ));
            press(&mut fixture.app, KeyCode::Tab);
            let layers = fixture.app.compositor().layer_ids();
            let previous = browser.attempts()?;
            // @step Then the existing attachment action receives its exact original path
            assert_eq!(activate(&mut fixture.app).await?, vec![path.to_string()]);
            let attempts = browser.attempts()?;
            if outcome == "unavailable" {
                assert_eq!(attempts, previous);
                // @step And an unavailable viewer shows a visible error without closing details
                assert_visible_error(
                    &mut fixture.app,
                    path,
                    &["viewer unavailable", "viewer is unavailable"],
                )?;
            } else {
                let launched = attempts.last().ok_or("missing browser launch")?;
                let parsed = reqwest::Url::parse(launched)?;
                assert_eq!(parsed.scheme(), "http");
                assert_eq!(parsed.host_str(), Some("127.0.0.1"));
                let port = parsed.port().ok_or("viewer port")?;
                assert_ne!(port, wrong_viewer.port);
                let target = App::attachment_url(port, path);
                let encoded_name = if path.ends_with("日本語 #%.md") {
                    "%20%23%25.md"
                } else if path.contains("%20") {
                    "literal%2520name.md"
                } else {
                    "literal%20name.md"
                };
                assert!(target.contains(encoded_name));
                assert_eq!(&attempts[..previous.len()], previous.as_slice());
                assert_eq!(&attempts[previous.len()..], &[target]);
                browser::assert_served(launched, path).await?;
                if outcome == "browser failure" {
                    assert_visible_error(&mut fixture.app, path, &["browser", "launch"])?;
                    assert_visible_error(
                        &mut fixture.app,
                        path,
                        &["board024 injected browser launch failure"],
                    )?;
                } else {
                    assert!(!overlay_text(&mut fixture.app, 180, 36)
                        .to_lowercase()
                        .contains("error:"));
                }
            }
            assert_eq!(fixture.app.compositor().layer_ids(), layers);
            assert!(selected_position(&buffer(&mut fixture.app, 120, 36), path).is_some());
            assert!(text(&mut fixture.app).contains("AUTH-001"));
            assert!(fixture.app.current_session().is_none());
            assert_eq!(activate(&mut fixture.app).await?, vec![path.to_string()]);
            let repeated = browser.attempts()?;
            if outcome == "unavailable" {
                assert_eq!(repeated, attempts);
            } else {
                assert_eq!(&repeated[..attempts.len()], attempts.as_slice());
                assert_eq!(
                    &repeated[attempts.len()..],
                    &[attempts
                        .last()
                        .ok_or("missing previous browser launch")?
                        .clone()]
                );
            }
        }
        wrong_viewer.stop().await;
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn missing_directory_and_rejected_attachment_paths_show_contextual_errors() -> TestResult {
    let _isolation = Isolation::new()?;
    for (path, reason) in [
        ("spec/attachments/A/absent-target.md", "not found"),
        ("spec/attachments/A/container", "directory"),
        ("../outside.md", "rejected"),
    ] {
        // @step Given details with missing directory and traversal attachment targets
        let mut fixture = Canonical::new(false).await?;
        if reason == "directory" {
            std::fs::create_dir_all(fixture.root.path().join(path))?;
        }
        fixture.record["attachments"] = json!([path]);
        fixture.save()?;
        let browser = browser::Browser::new(&mut fixture.app, true, true);
        open_details(&mut fixture.app).await?;
        // @step When the user activates each attachment
        focus_label(&mut fixture.app, path);
        let _paths = activate(&mut fixture.app).await?;
        // @step Then each failure is visible and the current card and attachment selection remain usable
        assert_visible_error(&mut fixture.app, path, &[reason])?;
        assert!(
            browser.attempts()?.is_empty(),
            "invalid target must not launch a browser"
        );
        assert!(text(&mut fixture.app).contains("AUTH-001"));
        assert!(selected_position(&buffer(&mut fixture.app, 120, 36), path).is_some());
        press(&mut fixture.app, KeyCode::Esc);
        assert_chooser(&mut fixture.app);
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn display_sanitization_preserves_original_identifiers_and_attachment_actions() -> TestResult
{
    let _isolation = Isolation::new()?;
    // @step Given canonical text with terminal control sequences and an attachment path requiring encoding
    let mut fixture = Canonical::new(false).await?;
    let path = "spec/attachments/A/a 日本語 #%.md";
    fixture.record["title"] = json!("Canonical\u{1b}[31m A\u{1b}[0m");
    fixture.record["description"] =
        json!("CLEAN-TEXT\u{1b}]0;bad-title\u{7}\u{1b}[31m日本語\u{1b}[0m\u{8}");
    fixture.record["attachments"] = json!([path]);
    fixture.save()?;
    std::fs::write(fixture.root.path().join(path), "# Safe contents")?;
    let browser = browser::Browser::new(&mut fixture.app, true, false);
    // @step When the user renders details and selects the attachment
    open_details(&mut fixture.app).await?;
    let rendered = harvest(&mut fixture.app, 120, 36);
    focus_label(&mut fixture.app, path);
    // @step Then rendered text contains no unsafe terminal controls
    assert!(rendered.contains("CLEAN-TEXT"));
    assert!(!rendered
        .chars()
        .any(|character| character.is_control() && character != '\n'));
    assert!(!rendered.contains("bad-title"));
    // @step And attachment activation retains the original path for existing URL encoding
    assert_eq!(activate(&mut fixture.app).await?, vec![path.to_string()]);
    assert!(browser.attempts()?.is_empty());
    assert_eq!(
        App::attachment_url(8123, path),
        "http://127.0.0.1:8123/view/spec/attachments/A/a%20%E6%97%A5%E6%9C%AC%E8%AA%9E%20%23%25.md"
    );
    Ok(())
}
