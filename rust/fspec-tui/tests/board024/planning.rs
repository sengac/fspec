//! Feature: spec/features/board-card-details-planning-sections.feature

use codelet_test_helpers::TestResult;
use serde_json::json;

use super::*;

fn position(rendered: &str, needle: &str) -> Result<usize, Box<dyn std::error::Error>> {
    rendered
        .find(needle)
        .ok_or_else(|| format!("details missing {needle}:\n{rendered}").into())
}

#[tokio::test]
#[serial_test::serial]
async fn details_show_complete_example_mapping_and_event_storm_sections() -> TestResult {
    let _isolation = Isolation::new()?;
    // @step Given a canonical card with example mapping and every event storm item type
    let mut fixture = Canonical::new(false).await?;
    fixture.record["eventStorm"] = json!({"level": "big_picture", "nextItemId": 7, "items": [
        {"id": 0, "type": "event", "color": "orange", "text": "EVENT-ITEM-MARKER", "deleted": false},
        {"id": 1, "type": "command", "color": "blue", "text": "COMMAND-ITEM-MARKER", "deleted": false},
        {"id": 2, "type": "policy", "color": "purple", "text": "POLICY-ITEM-MARKER", "when": "POLICY-WHEN-MARKER", "then": "POLICY-THEN-MARKER", "deleted": false},
        {"id": 3, "type": "hotspot", "color": "red", "text": "HOTSPOT-ITEM-MARKER", "concern": "HOTSPOT-CONCERN-MARKER", "deleted": false},
        {"id": 4, "type": "aggregate", "color": "yellow", "text": "AGGREGATE-ITEM-MARKER", "responsibilities": ["RESPONSIBILITY-MARKER"], "boundedContext": "CONTEXT-ITEM-MARKER", "deleted": false},
        {"id": 5, "type": "bounded_context", "color": null, "text": "CONTEXT-ITEM-MARKER", "description": "CONTEXT-DESCRIPTION-MARKER", "deleted": false},
        {"id": 6, "type": "external_system", "color": "pink", "text": "EXTERNAL-ITEM-MARKER", "integrationType": "REST_API", "deleted": false}
    ]});
    fixture.save()?;
    // @step When the user opens full card details
    open_details(&mut fixture.app).await?;
    let rendered = harvest(&mut fixture.app, 120, 36);
    // @step Then an Example Mapping section shows the user story rules examples answered questions assumptions and architecture notes
    let mapping = position(&rendered, "Example Mapping")?;
    for value in [
        "ROLE-MARKER",
        "ACTION-MARKER",
        "BENEFIT-MARKER",
        "ACTIVE-RULE",
        "EXAMPLE-MARKER",
        "ASSUMPTION-MARKER",
        "ARCHITECTURE-MARKER",
    ] {
        assert!(
            position(&rendered, value)? > mapping,
            "{value} outside mapping"
        );
    }
    assert!(
        rendered
            .lines()
            .any(|line| line.contains("ANSWERED-QUESTION") && line.contains("ANSWER-MARKER")),
        "questions must show their answers"
    );
    // @step And an Event Storm section lists every item under its type heading with its type-specific fields
    let storm = position(&rendered, "Event Storm")?;
    for (heading, item, fields) in [
        ("Domain Events", "EVENT-ITEM-MARKER", &[][..]),
        ("Commands", "COMMAND-ITEM-MARKER", &[][..]),
        (
            "Policies",
            "POLICY-ITEM-MARKER",
            &["POLICY-WHEN-MARKER", "POLICY-THEN-MARKER"][..],
        ),
        (
            "Hotspots",
            "HOTSPOT-ITEM-MARKER",
            &["HOTSPOT-CONCERN-MARKER"][..],
        ),
        (
            "Aggregates",
            "AGGREGATE-ITEM-MARKER",
            &["RESPONSIBILITY-MARKER"][..],
        ),
        (
            "Bounded Contexts",
            "CONTEXT-ITEM-MARKER",
            &["CONTEXT-DESCRIPTION-MARKER"][..],
        ),
        (
            "External Systems",
            "EXTERNAL-ITEM-MARKER",
            &["REST_API"][..],
        ),
    ] {
        let heading_at = position(&rendered[storm..], heading)? + storm;
        let item_at = position(&rendered[heading_at..], item)? + heading_at;
        for field in fields {
            assert!(
                position(&rendered[item_at..], field).is_ok(),
                "{field} missing"
            );
        }
    }
    assert!(rendered.contains("big_picture"));
    Ok(())
}
