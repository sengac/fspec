//! Feature: spec/features/board-card-enter-chooser-and-complete-details.feature

use crossterm::event::KeyCode;
use serde_json::json;

use super::*;

#[tokio::test]
#[serial_test::serial]
async fn full_canonical_details_agree_across_embedded_and_websocket_backends() -> TestResult {
    let _isolation = Isolation::new()?;
    let mut transport_outputs = Vec::new();
    for websocket in [false, true] {
        // @step Given a daemon workspace with a complete rich card and a truncated board summary
        let mut fixture = Canonical::new(websocket).await?;
        assert_eq!(
            fixture
                .app
                .board_store()
                .selected_work_unit()
                .and_then(|card| card.description.as_deref()),
            Some("TRUNCATED BOARD SUMMARY")
        );
        assert_ne!(std::env::current_dir()?, fixture.root.path());
        // @step When the user chooses Show full details through each transport
        request_details(&mut fixture.app);
        let response = held_responses(&mut fixture.app, "AUTH-001", false).await?;
        let envelope = response
            .envelope
            .ok_or("missing canonical details envelope")?;
        assert_eq!(envelope["id"], "AUTH-001");
        assert_eq!(
            envelope["workspace"].as_str(),
            fixture.root.path().canonicalize()?.to_str()
        );
        assert_eq!(envelope["workUnit"], fixture.record);
        for action in response.actions {
            fixture.app.dispatch(action);
        }
        settle(&mut fixture.app).await?;
        let rendered = harvest(&mut fixture.app, 120, 36);
        // @step Then all stored sections including historical and unknown fields are reachable
        for value in [
            "ROLE-MARKER",
            "ACTION-MARKER",
            "BENEFIT-MARKER",
            "COMPLETE-EPIC",
            "ACTIVE-RULE",
            "RETIRED-RULE",
            "EXAMPLE-MARKER",
            "ANSWERED-QUESTION",
            "ANSWER-MARKER",
            "ARCHITECTURE-MARKER",
            "ASSUMPTION-MARKER",
            "STATE-HISTORY-MARKER",
            "LEGACY-HISTORY-MARKER",
            "LEGACY-UPDATED",
            "BLOCKED-REASON-MARKER",
            "EVENT-STORM-MARKER",
            "RETIRED-EVENT-MARKER",
            "process_modeling",
            "VIRTUAL-HOOK-MARKER",
            "UNKNOWN-FIELD-MARKER",
            "ODD-NOTES-MARKER",
            "ODD-HISTORY-MARKER",
            "ODD-EVENT-MARKER",
            "2026-01-01",
            "2026-10-28",
            "12345",
            "AUTH-002",
            "AUTH-404",
            "PARENT-111",
            "CHILD-222",
            "BLOCK-333",
            "BLOCKER-444",
            "RELATED-555",
            "a 日本語 #%.md",
        ] {
            assert!(
                rendered.contains(value),
                "missing canonical value {value} via websocket={websocket}"
            );
        }
        assert!(
            !rendered.contains("TRUNCATED BOARD SUMMARY"),
            "canonical details must not substitute the board summary"
        );
        assert!(
            rendered.to_lowercase().contains("historical")
                || rendered.to_lowercase().contains("deleted")
        );
        let normalized = rendered.to_lowercase().replace(' ', "");
        for field in [
            "estimate",
            "iterations",
            "actualtokens",
            "parent",
            "children",
            "blocks",
            "dependson",
            "blockedby",
            "relatesto",
            "assumptions",
            "nextitemid",
        ] {
            assert!(
                normalized.contains(field),
                "missing readable canonical field {field}"
            );
        }
        // @step And canonical Unicode text is not replaced by the board summary
        for value in [
            "FIRST-CANONICAL-LINE",
            "日本語 café 🦀",
            "FINAL-CANONICAL-LINE",
        ] {
            assert!(
                rendered.contains(value),
                "canonical description missing {value}"
            );
        }
        transport_outputs.push(rendered);
    }
    assert_eq!(
        transport_outputs[0], transport_outputs[1],
        "transport-independent rendering must agree"
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn missing_and_malformed_canonical_cards_show_errors_without_losing_the_chooser() -> TestResult
{
    let _isolation = Isolation::new()?;
    for (invalid, categories) in [
        ("missing file", &["not found", "missing"][..]),
        ("invalid JSON", &["json", "parse"][..]),
        ("missing target", &["not found", "missing"][..]),
        ("invalid target", &["invalid", "object"][..]),
    ] {
        // @step Given a board summary whose canonical store is missing malformed or invalid
        let mut fixture = Canonical::new(false).await?;
        let path = fixture.root.path().join("spec/work-units.json");
        match invalid {
            "missing file" => std::fs::remove_file(path)?,
            "invalid JSON" => std::fs::write(path, "not JSON")?,
            "missing target" => std::fs::write(path, "{\"workUnits\":{}}")?,
            _ => std::fs::write(path, "{\"workUnits\":{\"AUTH-001\":42}}")?,
        }
        // @step When the user chooses Show full details
        open_details(&mut fixture.app).await?;
        // @step Then a visible useful error retains the captured card and origin
        let rendered = text(&mut fixture.app);
        assert!(rendered.contains("AUTH-001"));
        assert_visible_error(&mut fixture.app, "AUTH-001", categories)?;
        assert_eq!(
            fixture
                .app
                .board_store()
                .selected_work_unit()
                .map(|card| card.id.as_str()),
            Some("AUTH-001")
        );
        assert_eq!(fixture.app.active_view(), ViewMode::Board);
        // @step And Escape returns through the chooser without creating an agent
        press(&mut fixture.app, KeyCode::Esc);
        assert_chooser(&mut fixture.app);
        press(&mut fixture.app, KeyCode::Esc);
        assert!(fixture.app.compositor().is_empty());
        assert!(fixture.app.current_session().is_none());
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn reopening_the_same_card_rejects_stale_success_and_error_completions() -> TestResult {
    let _isolation = Isolation::new()?;
    for old_error in [false, true] {
        // @step Given an earlier details request has completed but its response is delayed
        let mut fixture = Canonical::new(false).await?;
        if old_error {
            std::fs::write(
                fixture.root.path().join("spec/work-units.json"),
                "OLD-INVALID-JSON",
            )?;
        }
        request_details(&mut fixture.app);
        let delayed = held_responses(&mut fixture.app, "AUTH-001", old_error).await?;
        // @step When the user closes and reopens the same card after its canonical data changes
        press(&mut fixture.app, KeyCode::Esc);
        assert_chooser(&mut fixture.app);
        press(&mut fixture.app, KeyCode::Esc);
        fixture.record["description"] = json!("NEW-GENERATION-DETAILS");
        fixture.save()?;
        request_details(&mut fixture.app);
        let current_responses = held_responses(&mut fixture.app, "AUTH-001", false).await?;
        assert_ne!(delayed.generation, current_responses.generation);
        let loading = text(&mut fixture.app);
        for response in delayed.actions.clone() {
            fixture.app.dispatch(response);
        }
        assert_eq!(
            text(&mut fixture.app),
            loading,
            "old responses must not replace a newer loading generation"
        );
        for response in current_responses.actions {
            fixture.app.dispatch(response);
        }
        // @step Then the current generation shows only the new canonical details
        let current = text(&mut fixture.app);
        assert!(current.contains("NEW-GENERATION-DETAILS"));
        assert!(!current.contains("FIRST-CANONICAL-LINE"));
        // @step And delivering an older success or error cannot replace the current details
        for response in delayed.actions {
            fixture.app.dispatch(response);
        }
        assert_eq!(
            text(&mut fixture.app),
            current,
            "same-ID generations must reject late responses"
        );
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn dismissed_requests_cannot_resurrect_dialogs_or_replace_another_card() -> TestResult {
    let _isolation = Isolation::new()?;
    for open_other in [false, true] {
        // @step Given a completed details response has not yet reached the open dialog
        let mut fixture = Canonical::new(false).await?;
        request_details(&mut fixture.app);
        let delayed = held_responses(&mut fixture.app, "AUTH-001", false).await?;
        // @step When the user dismisses it or opens a different card before response delivery
        press(&mut fixture.app, KeyCode::Esc);
        press(&mut fixture.app, KeyCode::Esc);
        assert!(fixture.app.compositor().is_empty());
        assert_eq!(fixture.app.active_view(), ViewMode::Board);
        if open_other {
            fixture
                .app
                .board_store_mut()
                .select_work_unit("AUTH-002", 20);
            request_details(&mut fixture.app);
            let current = held_responses(&mut fixture.app, "AUTH-002", false).await?;
            assert_ne!(delayed.generation, current.generation);
            for response in current.actions {
                fixture.app.dispatch(response);
            }
            assert!(text(&mut fixture.app).contains("B-CANONICAL-DETAIL"));
        }
        let previous = text(&mut fixture.app);
        let layers = fixture.app.compositor().layer_ids();
        // @step Then the delayed response cannot reopen the dismissed dialog or replace the other card
        for response in delayed.actions {
            fixture.app.dispatch(response);
        }
        assert_eq!(text(&mut fixture.app), previous);
        assert_eq!(fixture.app.compositor().layer_ids(), layers);
    }
    Ok(())
}
