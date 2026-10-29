//! Feature: spec/features/board-card-details-canonical-backend.feature

use serde_json::json;

#[path = "board024_canonical_backend/errors.rs"]
mod errors;
#[path = "board024_canonical_backend/fixture.rs"]
mod fixture;
#[path = "board024_canonical_backend/record.rs"]
mod record;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn canonical_details_preserve_the_complete_record_and_daemon_workspace_across_both_transports(
) -> TestResult {
    // @step Given a daemon workspace with a complete canonical card including historical unknown and legacy fields
    let canonical = record::rich_record();
    let mut responses = Vec::new();
    let mut failures = Vec::new();
    for websocket in [false, true] {
        let fixture = fixture::Fixture::new(websocket, true, &canonical).await?;
        let workspace = fixture.root.path().canonicalize()?;
        assert_ne!(
            workspace,
            std::env::current_dir()?.canonicalize()?,
            "fixture must not be the client cwd"
        );
        // @step When the canonical details RPC is invoked through embedded and WebSocket clients
        match fixture.details("AUTH-001").await {
            Ok(value) => {
                // @step Then both clients receive the same complete id workspace workUnit JSON envelope without projection or truncation
                let envelope = fixture::envelope(value)?;
                assert_eq!(
                    envelope,
                    json!({
                        "id": "AUTH-001", "workspace": workspace.to_str().ok_or("UTF-8 fixture root")?,
                        "workUnit": canonical
                    })
                );
                responses.push(envelope["workUnit"].clone());
            }
            Err(error) => failures.push(format!("{}: {error}", fixture.transport_name())),
        }
    }
    assert!(
        failures.is_empty(),
        "canonical RPC unavailable:\n{}",
        failures.join("\n")
    );
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], responses[1]);
    Ok(())
}

#[tokio::test]
async fn canonical_details_read_updated_storage_instead_of_cached_board_snapshots() -> TestResult {
    // @step Given a canonical card and an earlier captured board summary snapshot
    let original = record::rich_record();
    let mut failures = Vec::new();
    for websocket in [false, true] {
        let fixture = fixture::Fixture::new(websocket, true, &original).await?;
        let before = fixture.details("AUTH-001").await;
        let mut updated = original.clone();
        updated["description"] = json!("CURRENT-CANONICAL-AFTER-BOARD-SNAPSHOT");
        updated["futureField"]["nested"] = json!("CURRENT-UNKNOWN-FIELD");
        // @step When canonical storage changes and each transport requests the same card again
        fixture.save(&updated)?;
        let after = fixture.details("AUTH-001").await;
        // @step Then both responses contain the new canonical record while the original snapshot remains unchanged
        assert_eq!(
            fixture.snapshot[0].description.as_deref(),
            original["description"].as_str()
        );
        match (before, after) {
            (Ok(before), Ok(after)) => {
                assert_eq!(fixture::envelope(before)?["workUnit"], original);
                assert_eq!(fixture::envelope(after)?["workUnit"], updated);
                assert_ne!(
                    fixture.snapshot[0].description.as_deref(),
                    updated["description"].as_str()
                );
            }
            (before, after) => failures.push(format!(
                "{}: before={before:?}, after={after:?}",
                fixture.transport_name()
            )),
        }
    }
    assert!(
        failures.is_empty(),
        "fresh canonical reads unavailable:\n{}",
        failures.join("\n")
    );
    Ok(())
}

#[tokio::test]
async fn canonical_loader_failures_remain_explicit_daemon_errors_on_both_transports() -> TestResult
{
    // @step Given isolated fixtures with missing workspace card or store unreadable store malformed JSON invalid store or empty identifier
    let cases = errors::cases();
    let mut failures = Vec::new();
    let mut exercised = 0;
    for websocket in [false, true] {
        for case in &cases {
            let fixture =
                fixture::Fixture::new(websocket, case.attach_workspace, &record::rich_record())
                    .await?;
            case.prepare(&fixture)?;
            // @step When each failure is requested through embedded and WebSocket canonical details dispatch
            let result = fixture.details(case.id).await;
            exercised += 1;
            // @step Then every request returns an explicit contextual daemon error rather than unknown method transport failure empty data or stale success
            if let Err(error) = case.check(result) {
                failures.push(format!(
                    "{} / {}: {error}",
                    fixture.transport_name(),
                    case.name
                ));
            }
        }
    }
    assert_eq!(
        exercised,
        cases.len() * 2,
        "every error case must reach both transports"
    );
    assert!(
        failures.is_empty(),
        "canonical loader errors not implemented:\n{}",
        failures.join("\n")
    );
    Ok(())
}
