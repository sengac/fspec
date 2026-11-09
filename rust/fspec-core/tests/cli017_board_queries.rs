//! CLI-017 — list-work-units answers board queries by time, history, text,
//! links and every other board field, so nobody reads work-units.json with jq.
//!
//! Feature: spec/features/board-queries-with-list-work-units.feature
//!
//! Every query runs the core command with a fixed clock and a named zone
//! (`run_at`), never the wall clock (TEST-042).

#[path = "cli017_board_queries/support.rs"]
mod support;

use serde_json::{json, Value};
use support::*;

#[test]
fn cards_done_in_the_last_two_days_are_counted_per_local_day() -> TestResult {
    // @step Given a board in time zone Australia/Brisbane where now is 2026-11-07T06:00:00Z
    let clock = brisbane()?;
    // @step And CLI-001 was done at 2026-11-05T15:30:00Z, CLI-002 at 2026-11-06T23:00:00Z and CLI-003 at 2026-11-04T10:00:00Z
    let done = |id: &str, at: &str| {
        card(
            id,
            "story",
            "done",
            json!({"stateHistory": history(&[("specifying", "2026-10-02T00:00:00.000Z"), ("done", at)])}),
        )
    };
    let dir = board(&[
        done("CLI-001", "2026-11-05T15:30:00.000Z"),
        done("CLI-002", "2026-11-06T23:00:00.000Z"),
        done("CLI-003", "2026-11-04T10:00:00.000Z"),
    ])?;
    // @step When I list work units with "--status done --done-since 2d --group-by day"
    let out = ok(
        dir.path(),
        json!({"status": "done", "doneSince": "2d", "groupBy": "day"}),
        &clock,
    )?;
    // @step Then the output counts 1 card on 2026-11-06 and 1 card on 2026-11-07
    assert_eq!(
        group_rows(&out),
        vec![
            ("2026-11-06".to_string(), "1".to_string()),
            ("2026-11-07".to_string(), "1".to_string())
        ],
        "{out}"
    );
    Ok(())
}

#[test]
fn cards_in_testing_or_specifying_with_open_questions() -> TestResult {
    // @step Given CLI-001 in testing has an open question, CLI-002 in specifying has an open question, CLI-003 in testing has only an answered question and CLI-004 in implementing has an open question
    let open = json!([{"id": 0, "text": "@human: open?", "selected": false, "deleted": false}]);
    let answered = json!([{"id": 0, "text": "@human: done?", "selected": true, "answer": "yes", "deleted": false},
                          {"id": 1, "text": "@human: gone?", "selected": false, "deleted": true}]);
    let dir = board(&[
        card("CLI-001", "story", "testing", json!({"questions": open})),
        card("CLI-002", "story", "specifying", json!({"questions": open})),
        card(
            "CLI-003",
            "story",
            "testing",
            json!({"questions": answered}),
        ),
        card(
            "CLI-004",
            "story",
            "implementing",
            json!({"questions": open}),
        ),
    ])?;
    // @step When I list work units with "--status testing,specifying --has-open-questions --format ids"
    let out = ok(
        dir.path(),
        json!({"status": "testing,specifying", "hasOpenQuestions": true, "format": "ids"}),
        &brisbane()?,
    )?;
    // @step Then the output is the ids CLI-001 and CLI-002
    assert_eq!(ids(&out), strings(&["CLI-001", "CLI-002"]), "{out}");
    Ok(())
}

#[test]
fn bugs_created_since_a_date_that_are_not_done() -> TestResult {
    // @step Given a board in time zone Australia/Brisbane with bug BUG-001 created 2026-10-29T14:00:00Z, bug BUG-002 created 2026-10-30T02:00:00Z, done bug BUG-003 created 2026-11-01T00:00:00Z and story CLI-001 created 2026-11-01T00:00:00Z
    let clock = brisbane()?;
    let created = |at: &str| json!({"createdAt": at});
    let dir = board(&[
        card(
            "BUG-001",
            "bug",
            "backlog",
            created("2026-10-29T14:00:00.000Z"),
        ),
        card(
            "BUG-002",
            "bug",
            "testing",
            created("2026-10-30T02:00:00.000Z"),
        ),
        card(
            "BUG-003",
            "bug",
            "done",
            created("2026-11-01T00:00:00.000Z"),
        ),
        card(
            "CLI-001",
            "story",
            "backlog",
            created("2026-11-01T00:00:00.000Z"),
        ),
    ])?;
    // @step When I list work units with "--type bug --created-since 2026-10-30 --status backlog,specifying,testing,implementing,validating,blocked --format ids"
    let out = ok(
        dir.path(),
        json!({"type": "bug", "createdSince": "2026-10-30",
               "status": ["backlog", "specifying", "testing", "implementing", "validating", "blocked"],
               "format": "ids"}),
        &clock,
    )?;
    // @step Then the output is the ids BUG-001 and BUG-002
    assert_eq!(ids(&out), strings(&["BUG-001", "BUG-002"]), "{out}");
    Ok(())
}

#[test]
fn ids_of_the_cards_blocked_by_a_card() -> TestResult {
    // @step Given BUG-002 and BUG-003 are blocked by BUG-001 and BUG-004 depends on BUG-001
    let dir = board(&[
        card(
            "BUG-001",
            "bug",
            "backlog",
            json!({"blocks": ["BUG-002", "BUG-003"]}),
        ),
        card(
            "BUG-002",
            "bug",
            "backlog",
            json!({"blockedBy": ["BUG-001"]}),
        ),
        card(
            "BUG-003",
            "bug",
            "blocked",
            json!({"blockedBy": ["BUG-001"]}),
        ),
        card(
            "BUG-004",
            "bug",
            "backlog",
            json!({"dependsOn": ["BUG-001"]}),
        ),
    ])?;
    // @step When I list work units with "--blocked-by BUG-001 --format ids"
    let out = ok(
        dir.path(),
        json!({"blockedBy": "BUG-001", "format": "ids"}),
        &brisbane()?,
    )?;
    // @step Then the output is the ids BUG-002 and BUG-003
    assert_eq!(ids(&out), strings(&["BUG-002", "BUG-003"]), "{out}");
    Ok(())
}

#[test]
fn cards_with_assumptions_show_their_provisional_decisions() -> TestResult {
    // @step Given CLI-001 has the assumptions "uses the local zone" and "2d is 48 h" and CLI-002 has none
    let dir = board(&[
        card(
            "CLI-001",
            "story",
            "specifying",
            json!({"assumptions": ["uses the local zone", "2d is 48 h"]}),
        ),
        card("CLI-002", "story", "specifying", json!({"assumptions": []})),
    ])?;
    // @step When I list work units with "--has-assumptions --fields id,assumptions --format json"
    let out = ok(
        dir.path(),
        json!({"hasAssumptions": true, "fields": "id,assumptions", "format": "json"}),
        &brisbane()?,
    )?;
    // @step Then the JSON lists only CLI-001 with assumptions "uses the local zone" and "2d is 48 h"
    assert_eq!(
        units(&out)?,
        vec![json!({"id": "CLI-001", "assumptions": ["uses the local zone", "2d is 48 h"]})],
        "{out}"
    );
    Ok(())
}

#[test]
fn cards_that_entered_validating_today_whatever_their_status_now() -> TestResult {
    // @step Given a board in time zone Australia/Brisbane where now is 2026-11-07T06:00:00Z
    let clock = brisbane()?;
    // @step And CLI-001 entered validating at 2026-11-06T15:00:00Z and is now done, CLI-002 entered validating at 2026-11-06T13:00:00Z and CLI-003 never entered validating
    let dir = board(&[
        card(
            "CLI-001",
            "story",
            "done",
            json!({"stateHistory": history(&[("validating", "2026-11-06T15:00:00.000Z"), ("done", "2026-11-06T20:00:00.000Z")])}),
        ),
        card(
            "CLI-002",
            "story",
            "validating",
            json!({"stateHistory": history(&[("validating", "2026-11-06T13:00:00.000Z")])}),
        ),
        card(
            "CLI-003",
            "story",
            "testing",
            json!({"stateHistory": history(&[("testing", "2026-11-07T01:00:00.000Z")])}),
        ),
    ])?;
    // @step When I list work units with "--entered validating --entered-since 2026-11-07 --format ids"
    let out = ok(
        dir.path(),
        json!({"entered": "validating", "enteredSince": "2026-11-07", "format": "ids"}),
        &clock,
    )?;
    // @step Then the output is the ids CLI-001
    assert_eq!(ids(&out), strings(&["CLI-001"]), "{out}");
    Ok(())
}

#[test]
fn a_local_day_follows_the_zones_daylight_saving_change() -> TestResult {
    // @step Given a board in time zone America/New_York where now is 2026-11-07T06:00:00Z
    let clock = clock_in("America/New_York")?;
    // @step And CLI-001 was created at 2026-11-02T04:30:00Z and CLI-002 at 2026-11-02T05:30:00Z
    let dir = board(&[
        card(
            "CLI-001",
            "story",
            "backlog",
            json!({"createdAt": "2026-11-02T04:30:00.000Z"}),
        ),
        card(
            "CLI-002",
            "story",
            "backlog",
            json!({"createdAt": "2026-11-02T05:30:00.000Z"}),
        ),
    ])?;
    // @step When I list work units with "--created-since 2026-11-02 --format ids"
    let out = ok(
        dir.path(),
        json!({"createdSince": "2026-11-02", "format": "ids"}),
        &clock,
    )?;
    // @step Then the output is the ids CLI-002
    assert_eq!(ids(&out), strings(&["CLI-002"]), "{out}");
    // @step When I list work units with "--created-since 2026-11-01 --group-by day"
    let out = ok(
        dir.path(),
        json!({"createdSince": "2026-11-01", "groupBy": "day"}),
        &clock,
    )?;
    // @step Then the output counts 1 card on 2026-11-01 and 1 card on 2026-11-02
    assert_eq!(
        group_rows(&out),
        vec![
            ("2026-11-01".to_string(), "1".to_string()),
            ("2026-11-02".to_string(), "1".to_string())
        ],
        "{out}"
    );
    Ok(())
}

#[test]
fn a_date_only_until_value_covers_that_whole_local_day() -> TestResult {
    // @step Given a board in time zone Australia/Brisbane with CLI-001 created at 2026-10-30T13:59:00Z and CLI-002 created at 2026-10-30T14:00:00Z
    let dir = board(&[
        card(
            "CLI-001",
            "story",
            "backlog",
            json!({"createdAt": "2026-10-30T13:59:00.000Z"}),
        ),
        card(
            "CLI-002",
            "story",
            "backlog",
            json!({"createdAt": "2026-10-30T14:00:00.000Z"}),
        ),
    ])?;
    // @step When I list work units with "--created-until 2026-10-30 --format ids"
    let out = ok(
        dir.path(),
        json!({"createdUntil": "2026-10-30", "format": "ids"}),
        &brisbane()?,
    )?;
    // @step Then the output is the ids CLI-001
    assert_eq!(ids(&out), strings(&["CLI-001"]), "{out}");
    Ok(())
}

#[test]
fn relative_durations_timestamps_and_now() -> TestResult {
    // @step Given a board in time zone Australia/Brisbane where now is 2026-11-07T06:00:00Z
    let clock = brisbane()?;
    // @step And CLI-001 was updated at 2026-11-07T05:45:00Z, CLI-002 at 2026-11-07T01:00:00Z, CLI-003 at 2026-11-01T07:00:00Z and CLI-004 at 2026-10-20T00:00:00Z
    let updated = |id: &str, at: &str| card(id, "story", "backlog", json!({"updatedAt": at}));
    let dir = board(&[
        updated("CLI-001", "2026-11-07T05:45:00.000Z"),
        updated("CLI-002", "2026-11-07T01:00:00.000Z"),
        updated("CLI-003", "2026-11-01T07:00:00.000Z"),
        updated("CLI-004", "2026-10-20T00:00:00.000Z"),
    ])?;
    // @step When I list work units with "--updated-since 30m --format ids"
    let out = ok(
        dir.path(),
        json!({"updatedSince": "30m", "format": "ids"}),
        &clock,
    )?;
    // @step Then the output is the ids CLI-001
    assert_eq!(ids(&out), strings(&["CLI-001"]), "{out}");
    // @step When I list work units with "--updated-since 6h --updated-until now --format ids"
    let out = ok(
        dir.path(),
        json!({"updatedSince": "6h", "updatedUntil": "now", "format": "ids"}),
        &clock,
    )?;
    // @step Then the output is the ids CLI-001 and CLI-002
    assert_eq!(ids(&out), strings(&["CLI-001", "CLI-002"]), "{out}");
    // @step When I list work units with "--updated-since 1w --updated-until 2026-11-07T11:30:00+10:00 --format ids"
    let out = ok(
        dir.path(),
        json!({"updatedSince": "1w", "updatedUntil": "2026-11-07T11:30:00+10:00", "format": "ids"}),
        &clock,
    )?;
    // @step Then the output is the ids CLI-002 and CLI-003
    assert_eq!(ids(&out), strings(&["CLI-002", "CLI-003"]), "{out}");
    Ok(())
}

#[test]
fn an_unknown_status_is_an_exact_error_listing_the_valid_statuses() -> TestResult {
    // @step Given a board with one card
    let dir = board(&[card("CLI-001", "story", "backlog", json!({}))])?;
    // @step When I list work units with "--status donee"
    let e = err(dir.path(), json!({"status": "donee"}), &brisbane()?)?;
    // @step Then it fails with "Unknown status 'donee'. Valid statuses: backlog, specifying, testing, implementing, validating, done, blocked"
    assert!(
        e.contains("Unknown status 'donee'. Valid statuses: backlog, specifying, testing, implementing, validating, done, blocked"),
        "{e}"
    );
    Ok(())
}

#[test]
fn unknown_values_are_exact_errors_listing_the_valid_values() -> TestResult {
    // @step Given a board with prefixes CLI and BUG and epic board-queries registered
    let dir = board(&[
        card("CLI-001", "story", "backlog", json!({})),
        card("BUG-001", "bug", "backlog", json!({})),
    ])?;
    let clock = brisbane()?;
    let fails = |args: Value| err(dir.path(), args, &clock);
    // @step When I list work units with "--type feature"
    let e = fails(json!({"type": "feature"}))?;
    // @step Then it fails with "Unknown type 'feature'. Valid types: story, task, bug"
    assert!(
        e.contains("Unknown type 'feature'. Valid types: story, task, bug"),
        "{e}"
    );
    // @step When I list work units with "--prefix NOPE"
    let e = fails(json!({"prefix": "NOPE"}))?;
    // @step Then it fails with "Unknown prefix 'NOPE'. Valid prefixes: BUG, CLI"
    assert!(
        e.contains("Unknown prefix 'NOPE'. Valid prefixes: BUG, CLI"),
        "{e}"
    );
    // @step When I list work units with "--epic nope"
    let e = fails(json!({"epic": "nope"}))?;
    // @step Then it fails with "Unknown epic 'nope'. Valid epics: board-queries"
    assert!(
        e.contains("Unknown epic 'nope'. Valid epics: board-queries"),
        "{e}"
    );
    // @step When I list work units with "--fields id,colour"
    let e = fails(json!({"fields": "id,colour"}))?;
    // @step Then it fails with an error naming the field colour and listing the valid fields
    assert!(
        e.contains("Unknown field 'colour'. Valid fields: id, title, type, status, epic, prefix,"),
        "{e}"
    );
    // @step When I list work units with "--format text"
    let e = fails(json!({"format": "text"}))?;
    // @step Then it fails with "Unknown format 'text'. Valid formats: table, ids, json, jsonl, count, csv"
    assert!(
        e.contains("Unknown format 'text'. Valid formats: table, ids, json, jsonl, count, csv"),
        "{e}"
    );
    // @step When I list work units with "--group-by colour"
    let e = fails(json!({"groupBy": "colour"}))?;
    // @step Then it fails with "Unknown group-by 'colour'. Valid group-by values: status, type, prefix, epic, day, week"
    assert!(
        e.contains("Unknown group-by 'colour'. Valid group-by values: status, type, prefix, epic, day, week"),
        "{e}"
    );
    // @step When I list work units with "--sort colour"
    let e = fails(json!({"sort": "colour"}))?;
    // @step Then it fails with an error naming the sort key colour and listing the valid fields
    assert!(
        e.contains("Unknown sort key 'colour'. Valid fields: id, title, type, status,"),
        "{e}"
    );
    Ok(())
}

#[test]
fn invalid_time_values_and_ids_are_exact_errors() -> TestResult {
    // @step Given a board with one card CLI-001
    let dir = board(&[card("CLI-001", "story", "backlog", json!({}))])?;
    let clock = brisbane()?;
    // @step When I list work units with "--created-since yesterday"
    let e = err(dir.path(), json!({"createdSince": "yesterday"}), &clock)?;
    // @step Then it fails with an error naming 'yesterday' and the accepted forms YYYY-MM-DD, an ISO-8601 timestamp, Nm, Nh, Nd, Nw and now
    assert!(
        e.contains("Invalid time 'yesterday' for --created-since. Use YYYY-MM-DD, an ISO-8601 timestamp, a duration back from now (Nm, Nh, Nd or Nw, e.g. 30m, 6h, 2d, 1w) or now"),
        "{e}"
    );
    // @step When I list work units with "--entered-since 2d"
    let e = err(dir.path(), json!({"enteredSince": "2d"}), &clock)?;
    // @step Then it fails with "--entered-since and --entered-until need --entered <status>"
    assert!(
        e.contains("--entered-since and --entered-until need --entered <status>"),
        "{e}"
    );
    // @step When I list work units with "--blocked-by CLI-999"
    let e = err(dir.path(), json!({"blockedBy": "CLI-999"}), &clock)?;
    // @step Then it fails with "Work unit 'CLI-999' is not on the board"
    assert!(e.contains("Work unit 'CLI-999' is not on the board"), "{e}");
    // @step When I list work units with "--work-unit BUG-001..CLI-010"
    let e = err(
        dir.path(),
        json!({"workUnitId": "BUG-001..CLI-010"}),
        &clock,
    )?;
    // @step Then it fails with "A --work-unit range must use one prefix: BUG-001..CLI-010"
    assert!(
        e.contains("A --work-unit range must use one prefix: BUG-001..CLI-010"),
        "{e}"
    );
    Ok(())
}

#[test]
fn id_lists_id_ranges_and_field_filters() -> TestResult {
    // @step Given CLI-001 is the parent of CLI-002 and CLI-003, CLI-002 has estimate 3, CLI-003 has estimate 8 and CLI-004 has no estimate
    let dir = board(&[
        card(
            "CLI-001",
            "story",
            "backlog",
            json!({"children": ["CLI-002", "CLI-003"]}),
        ),
        card(
            "CLI-002",
            "story",
            "backlog",
            json!({"parent": "CLI-001", "estimate": 3}),
        ),
        card(
            "CLI-003",
            "story",
            "backlog",
            json!({"parent": "CLI-001", "estimate": 8}),
        ),
        card(
            "CLI-004",
            "story",
            "backlog",
            json!({"estimate": null, "children": []}),
        ),
    ])?;
    let clock = brisbane()?;
    let list =
        |args: Value| -> Result<Vec<String>, Failure> { Ok(ids(&ok(dir.path(), args, &clock)?)) };
    // @step When I list work units with "--work-unit CLI-002..CLI-004 --format ids"
    let out = list(json!({"workUnitId": "CLI-002..CLI-004", "format": "ids"}))?;
    // @step Then the output is the ids CLI-002, CLI-003 and CLI-004
    assert_eq!(out, strings(&["CLI-002", "CLI-003", "CLI-004"]));
    // @step When I list work units with "--work-unit CLI-001,CLI-004 --format ids"
    let out = list(json!({"workUnitId": "CLI-001,CLI-004", "format": "ids"}))?;
    // @step Then the output is the ids CLI-001 and CLI-004
    assert_eq!(out, strings(&["CLI-001", "CLI-004"]));
    // @step When I list work units with "--parent CLI-001 --estimate-min 5 --format ids"
    let out = list(json!({"parent": "CLI-001", "estimateMin": 5, "format": "ids"}))?;
    // @step Then the output is the ids CLI-003
    assert_eq!(out, strings(&["CLI-003"]));
    // @step When I list work units with "--has-children --format ids"
    let out = list(json!({"hasChildren": true, "format": "ids"}))?;
    // @step Then the output is the ids CLI-001
    assert_eq!(out, strings(&["CLI-001"]));
    // @step When I list work units with "--no-estimate --format ids"
    let out = list(json!({"noEstimate": true, "format": "ids"}))?;
    // @step Then the output is the ids CLI-001 and CLI-004
    assert_eq!(out, strings(&["CLI-001", "CLI-004"]));
    // @step When I list work units with "--has-estimate --estimate-max 3 --format ids"
    let out = list(json!({"hasEstimate": true, "estimateMax": 3, "format": "ids"}))?;
    // @step Then the output is the ids CLI-002
    assert_eq!(out, strings(&["CLI-002"]));
    Ok(())
}

#[test]
fn text_and_content_filters_ignore_deleted_items() -> TestResult {
    // @step Given CLI-001 is titled "Board Query" with a rule and an example, CLI-002 has "board query" in its description, an attachment and an architecture note, and CLI-003 has only deleted rules
    let item = |deleted: bool| json!([{"id": 0, "text": "x", "deleted": deleted, "createdAt": "2026-10-01T00:00:00.000Z"}]);
    let dir = board(&[
        card(
            "CLI-001",
            "story",
            "backlog",
            json!({"title": "Board Query", "rules": item(false), "examples": item(false)}),
        ),
        card(
            "CLI-002",
            "story",
            "backlog",
            json!({"title": "Other", "description": "uses a board query",
                   "attachments": ["spec/attachments/CLI-002/a.md"], "architectureNotes": item(false)}),
        ),
        card(
            "CLI-003",
            "story",
            "backlog",
            json!({"title": "Third", "rules": item(true), "architectureNotes": item(true)}),
        ),
    ])?;
    let clock = brisbane()?;
    let list =
        |args: Value| -> Result<Vec<String>, Failure> { Ok(ids(&ok(dir.path(), args, &clock)?)) };
    // @step When I list work units with "--text BOARD --format ids"
    let out = list(json!({"text": "BOARD", "format": "ids"}))?;
    // @step Then the output is the ids CLI-001 and CLI-002
    assert_eq!(out, strings(&["CLI-001", "CLI-002"]));
    // @step When I list work units with "--title board --format ids"
    let out = list(json!({"title": "board", "format": "ids"}))?;
    // @step Then the output is the ids CLI-001
    assert_eq!(out, strings(&["CLI-001"]));
    // @step When I list work units with "--has-rules --has-examples --format ids"
    let out = list(json!({"hasRules": true, "hasExamples": true, "format": "ids"}))?;
    // @step Then the output is the ids CLI-001
    assert_eq!(out, strings(&["CLI-001"]));
    // @step When I list work units with "--no-rules --format ids"
    let out = list(json!({"noRules": true, "format": "ids"}))?;
    // @step Then the output is the ids CLI-002 and CLI-003
    assert_eq!(out, strings(&["CLI-002", "CLI-003"]));
    // @step When I list work units with "--has-attachments --has-architecture-notes --format ids"
    let out = list(json!({"hasAttachments": true, "hasArchitectureNotes": true, "format": "ids"}))?;
    // @step Then the output is the ids CLI-002
    assert_eq!(out, strings(&["CLI-002"]));
    Ok(())
}

#[test]
fn a_tag_on_a_cards_feature_file_selects_the_card() -> TestResult {
    // @step Given spec/features/a.feature carries the feature tags @CLI-001 and @cli, and spec/features/b.feature carries @CLI-002 and @tui
    let dir = board(&[
        card("CLI-001", "story", "backlog", json!({})),
        card("CLI-002", "story", "backlog", json!({})),
    ])?;
    let features = dir.path().join("spec/features");
    std::fs::create_dir_all(&features)?;
    std::fs::write(
        features.join("a.feature"),
        "@CLI-001\n@cli\nFeature: A\n\n  Scenario: S\n    Given x\n",
    )?;
    std::fs::write(
        features.join("b.feature"),
        "@CLI-002 @tui\nFeature: B\n\n  Scenario: S\n    Given x\n",
    )?;
    // @step When I list work units with "--tag @cli --format ids"
    let out = ok(
        dir.path(),
        json!({"tag": "@cli", "format": "ids"}),
        &brisbane()?,
    )?;
    // @step Then the output is the ids CLI-001
    assert_eq!(ids(&out), strings(&["CLI-001"]), "{out}");
    Ok(())
}

#[test]
fn link_filters_take_an_id_or_any_and_blocked_cards_include_unfinished_blockers() -> TestResult {
    // @step Given BUG-002 depends on BUG-001, BUG-003 relates to BUG-001, BUG-004 is blocked by done BUG-005, BUG-006 is blocked by BUG-001 and BUG-007 has status blocked
    let dir = board(&[
        card("BUG-001", "bug", "backlog", json!({"blocks": ["BUG-006"]})),
        card(
            "BUG-002",
            "bug",
            "backlog",
            json!({"dependsOn": ["BUG-001"]}),
        ),
        card(
            "BUG-003",
            "bug",
            "backlog",
            json!({"relatesTo": ["BUG-001"]}),
        ),
        card(
            "BUG-004",
            "bug",
            "backlog",
            json!({"blockedBy": ["BUG-005"]}),
        ),
        card("BUG-005", "bug", "done", json!({"blocks": ["BUG-004"]})),
        card(
            "BUG-006",
            "bug",
            "backlog",
            json!({"blockedBy": ["BUG-001"]}),
        ),
        card("BUG-007", "bug", "blocked", json!({})),
    ])?;
    let clock = brisbane()?;
    let list =
        |args: Value| -> Result<Vec<String>, Failure> { Ok(ids(&ok(dir.path(), args, &clock)?)) };
    // @step When I list work units with "--depends-on any --format ids"
    let out = list(json!({"dependsOn": "any", "format": "ids"}))?;
    // @step Then the output is the ids BUG-002
    assert_eq!(out, strings(&["BUG-002"]));
    // @step When I list work units with "--relates-to BUG-001 --format ids"
    let out = list(json!({"relatesTo": "BUG-001", "format": "ids"}))?;
    // @step Then the output is the ids BUG-003
    assert_eq!(out, strings(&["BUG-003"]));
    // @step When I list work units with "--blocks any --format ids"
    let out = list(json!({"blocks": "any", "format": "ids"}))?;
    // @step Then the output is the ids BUG-001 and BUG-005
    assert_eq!(out, strings(&["BUG-001", "BUG-005"]));
    // @step When I list work units with "--blocked --format ids"
    let out = list(json!({"blocked": true, "format": "ids"}))?;
    // @step Then the output is the ids BUG-006 and BUG-007
    assert_eq!(out, strings(&["BUG-006", "BUG-007"]));
    Ok(())
}

fn two_cards() -> Result<tempfile::TempDir, Failure> {
    board(&[
        card(
            "CLI-001",
            "story",
            "backlog",
            json!({"title": "Board queries", "epic": "board-queries"}),
        ),
        card("CLI-002", "bug", "done", json!({"title": "Fix, then test"})),
    ])
}

#[test]
fn the_table_format_is_the_default_and_an_empty_result_prints_the_sentinel() -> TestResult {
    // @step Given CLI-001 is a backlog story in epic board-queries titled "Board queries" and CLI-002 is a done bug titled "Fix, then test" with no epic
    let dir = two_cards()?;
    let clock = brisbane()?;
    // @step When I list work units with no options
    let out = ok(dir.path(), json!({}), &clock)?;
    // @step Then the output is a table with the header ID, TYPE, STATUS, EPIC and TITLE, one aligned row per card and the line "2 work units"
    assert_eq!(
        out,
        "ID       TYPE   STATUS   EPIC           TITLE\n\
         CLI-001  story  backlog  board-queries  Board queries\n\
         CLI-002  bug    done                    Fix, then test\n\
         \n\
         2 work units"
    );
    // @step When I list work units with "--status testing"
    let out = ok(dir.path(), json!({"status": "testing"}), &clock)?;
    // @step Then the output is "No work units found"
    assert_eq!(out, "No work units found");
    Ok(())
}

#[test]
fn the_ids_json_jsonl_count_and_csv_formats() -> TestResult {
    // @step Given CLI-001 is a backlog story in epic board-queries titled "Board queries" and CLI-002 is a done bug titled "Fix, then test" with no epic
    let dir = two_cards()?;
    let clock = brisbane()?;
    // @step When I list work units with "--format json"
    let out = ok(dir.path(), json!({"format": "json"}), &clock)?;
    // @step Then the JSON workUnits array holds CLI-001 with epic board-queries and CLI-002 without an epic key
    let list = units(&out)?;
    assert_eq!(unit(&list, "CLI-001")?["epic"], "board-queries", "{out}");
    assert!(unit(&list, "CLI-002")?.get("epic").is_none(), "{out}");
    // @step When I list work units with "--format jsonl"
    let out = ok(dir.path(), json!({"format": "jsonl"}), &clock)?;
    // @step Then the output is two one-line JSON objects
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2, "{out}");
    for line in lines {
        assert!(serde_json::from_str::<Value>(line)?.is_object(), "{line}");
    }
    // @step When I list work units with "--format count"
    let out = ok(dir.path(), json!({"format": "count"}), &clock)?;
    // @step Then the output is "2"
    assert_eq!(out, "2");
    // @step When I list work units with "--format csv --fields id,title"
    let out = ok(
        dir.path(),
        json!({"format": "csv", "fields": "id,title"}),
        &clock,
    )?;
    // @step Then the output is the lines "id,title", "CLI-001,Board queries" and "CLI-002,\"Fix, then test\""
    assert_eq!(
        out,
        "id,title\nCLI-001,Board queries\nCLI-002,\"Fix, then test\""
    );
    Ok(())
}

#[test]
fn derived_time_fields_print_in_local_time_with_the_zone() -> TestResult {
    // @step Given a board in time zone Australia/Brisbane where now is 2026-11-07T06:00:00Z
    let clock = brisbane()?;
    // @step And CLI-001 was created at 2026-11-01T00:00:00Z, entered testing at 2026-11-03T02:00:00Z and was done at 2026-11-05T03:00:00Z, and CLI-002 was created at 2026-11-04T01:48:00Z and never changed status
    let dir = board(&[
        card(
            "CLI-001",
            "story",
            "done",
            json!({"createdAt": "2026-11-01T00:00:00.000Z",
                   "stateHistory": history(&[("specifying", "2026-11-02T00:00:00.000Z"),
                                             ("testing", "2026-11-03T02:00:00.000Z"),
                                             ("done", "2026-11-05T03:00:00.000Z")])}),
        ),
        card(
            "CLI-002",
            "story",
            "backlog",
            json!({"createdAt": "2026-11-04T01:48:00.000Z"}),
        ),
    ])?;
    // @step When I list work units with "--fields id,createdAt,enteredAt:testing,doneAt,ageInStatus --format json"
    let out = ok(
        dir.path(),
        json!({"fields": "id,createdAt,enteredAt:testing,doneAt,ageInStatus", "format": "json"}),
        &clock,
    )?;
    let list = units(&out)?;
    // @step Then CLI-001 has createdAt "2026-11-01T10:00:00+10:00", enteredAt:testing "2026-11-03T12:00:00+10:00", doneAt "2026-11-05T13:00:00+10:00" and ageInStatus "2d 3h 0m"
    assert_eq!(
        unit(&list, "CLI-001")?,
        &json!({"id": "CLI-001", "createdAt": "2026-11-01T10:00:00+10:00",
                "enteredAt:testing": "2026-11-03T12:00:00+10:00",
                "doneAt": "2026-11-05T13:00:00+10:00", "ageInStatus": "2d 3h 0m"})
    );
    // @step And CLI-002 has ageInStatus "3d 4h 12m" and no doneAt or enteredAt:testing key
    assert_eq!(
        unit(&list, "CLI-002")?,
        &json!({"id": "CLI-002", "createdAt": "2026-11-04T11:48:00+10:00", "ageInStatus": "3d 4h 12m"})
    );
    Ok(())
}

#[test]
fn open_questions_dependencies_and_list_fields() -> TestResult {
    // @step Given CLI-001 has open question 1 "@human: which zone?", answered question 0 and dependsOn CLI-002, and CLI-002 blocks CLI-003
    let dir = board(&[
        card(
            "CLI-001",
            "story",
            "specifying",
            json!({"dependsOn": ["CLI-002"], "questions": [
                {"id": 0, "text": "@human: which tz?", "selected": true, "answer": "local", "deleted": false},
                {"id": 1, "text": "@human: which zone?", "selected": false, "deleted": false}]}),
        ),
        card(
            "CLI-002",
            "story",
            "backlog",
            json!({"blocks": ["CLI-003"]}),
        ),
        card(
            "CLI-003",
            "story",
            "backlog",
            json!({"blockedBy": ["CLI-002"]}),
        ),
    ])?;
    let clock = brisbane()?;
    // @step When I list work units with "--work-unit CLI-001 --fields id,openQuestions,dependencies --format json"
    let out = ok(
        dir.path(),
        json!({"workUnitId": "CLI-001", "fields": "id,openQuestions,dependencies", "format": "json"}),
        &clock,
    )?;
    // @step Then CLI-001 has openQuestions [{"id": 1, "text": "@human: which zone?"}] and dependencies {"dependsOn": ["CLI-002"]}
    assert_eq!(
        units(&out)?,
        vec![json!({"id": "CLI-001",
                    "openQuestions": [{"id": 1, "text": "@human: which zone?"}],
                    "dependencies": {"dependsOn": ["CLI-002"]}})],
        "{out}"
    );
    // @step When I list work units with "--work-unit CLI-001,CLI-002 --fields id,openQuestions,blocks --format csv"
    let out = ok(
        dir.path(),
        json!({"workUnitId": ["CLI-001", "CLI-002"], "fields": "id,openQuestions,blocks", "format": "csv"}),
        &clock,
    )?;
    // @step Then the output is the lines "id,openQuestions,blocks", "CLI-001,[1] @human: which zone?," and "CLI-002,,CLI-003"
    assert_eq!(
        out,
        "id,openQuestions,blocks\nCLI-001,[1] @human: which zone?,\nCLI-002,,CLI-003"
    );
    Ok(())
}

#[test]
fn group_by_status_type_prefix_epic_and_week() -> TestResult {
    // @step Given BUG-001 and BUG-002 are backlog bugs in epic board-queries, CLI-001 is a done story and CLI-002 is a testing story in epic board-queries
    let epic = json!({"epic": "board-queries"});
    let dir = board(&[
        card("BUG-001", "bug", "backlog", epic.clone()),
        card("CLI-001", "story", "done", json!({})),
        card("BUG-002", "bug", "backlog", epic.clone()),
        card("CLI-002", "story", "testing", epic),
    ])?;
    let clock = brisbane()?;
    // @step When I list work units with "--group-by status"
    let out = ok(dir.path(), json!({"groupBy": "status"}), &clock)?;
    // @step Then the output counts backlog 2, testing 1 and done 1 in workflow order
    assert_eq!(
        out,
        "STATUS   COUNT\nbacklog  2\ntesting  1\ndone     1\n\n4 work units"
    );
    // @step When I list work units with "--group-by prefix --format csv"
    let out = ok(
        dir.path(),
        json!({"groupBy": "prefix", "format": "csv"}),
        &clock,
    )?;
    // @step Then the output is the lines "prefix,count", "BUG,2" and "CLI,2"
    assert_eq!(out, "prefix,count\nBUG,2\nCLI,2");
    // @step When I list work units with "--group-by epic --format json"
    let out = ok(
        dir.path(),
        json!({"groupBy": "epic", "format": "json"}),
        &clock,
    )?;
    // @step Then the JSON groups are board-queries with 3 cards and the key null with CLI-001
    let v = json_out(&out)?;
    assert_eq!(v["groupBy"], "epic", "{out}");
    let groups = v["groups"].as_array().ok_or("no groups")?;
    assert_eq!(groups.len(), 2, "{out}");
    assert_eq!(groups[0]["key"], "board-queries");
    assert_eq!(groups[0]["count"], 3);
    assert_eq!(groups[0]["workUnits"].as_array().map(Vec::len), Some(3));
    assert_eq!(groups[1]["key"], Value::Null);
    assert_eq!(groups[1]["count"], 1);
    assert_eq!(groups[1]["workUnits"][0]["id"], "CLI-001");
    // @step When I list work units with "--group-by type --format count"
    let out = ok(
        dir.path(),
        json!({"groupBy": "type", "format": "count"}),
        &clock,
    )?;
    // @step Then the output counts bug 2 and story 2
    assert_eq!(out, "bug 2\nstory 2");
    Ok(())
}

#[test]
fn group_by_day_or_week_needs_exactly_one_kind_of_time_filter() -> TestResult {
    // @step Given a board in time zone Australia/Brisbane where now is 2026-11-07T06:00:00Z
    let clock = brisbane()?;
    // @step And CLI-001 was created at 2026-11-01T00:00:00Z and CLI-002 at 2026-11-03T00:00:00Z
    let dir = board(&[
        card(
            "CLI-001",
            "story",
            "backlog",
            json!({"createdAt": "2026-11-01T00:00:00.000Z"}),
        ),
        card(
            "CLI-002",
            "story",
            "backlog",
            json!({"createdAt": "2026-11-03T00:00:00.000Z"}),
        ),
    ])?;
    // @step When I list work units with "--created-since 2026-10-01 --group-by week"
    let out = ok(
        dir.path(),
        json!({"createdSince": "2026-10-01", "groupBy": "week"}),
        &clock,
    )?;
    // @step Then the output counts 1 card in 2026-W44 and 1 card in 2026-W45
    assert_eq!(
        group_rows(&out),
        vec![
            ("2026-W44".to_string(), "1".to_string()),
            ("2026-W45".to_string(), "1".to_string())
        ],
        "{out}"
    );
    let needs_one =
        "--group-by day needs exactly one kind of time filter (created, updated, done or entered)";
    // @step When I list work units with "--group-by day"
    let e = err(dir.path(), json!({"groupBy": "day"}), &clock)?;
    // @step Then it fails with "--group-by day needs exactly one kind of time filter (created, updated, done or entered)"
    assert!(e.contains(needs_one), "{e}");
    // @step When I list work units with "--created-since 2026-10-01 --done-since 2d --group-by day"
    let e = err(
        dir.path(),
        json!({"createdSince": "2026-10-01", "doneSince": "2d", "groupBy": "day"}),
        &clock,
    )?;
    // @step Then it fails with "--group-by day needs exactly one kind of time filter (created, updated, done or entered)"
    assert!(e.contains(needs_one), "{e}");
    // @step When I list work units with "--created-since 2026-10-01 --group-by week --format ids"
    let e = err(
        dir.path(),
        json!({"createdSince": "2026-10-01", "groupBy": "week", "format": "ids"}),
        &clock,
    )?;
    // @step Then it fails with "--group-by works with --format table, count, csv or json"
    assert!(
        e.contains("--group-by works with --format table, count, csv or json"),
        "{e}"
    );
    Ok(())
}

#[test]
fn sort_and_limit() -> TestResult {
    // @step Given CLI-001 was created at 2026-11-01T00:00:00Z, CLI-002 at 2026-11-03T00:00:00Z and CLI-003 at 2026-11-02T00:00:00Z
    let dir = board(&[
        card(
            "CLI-001",
            "story",
            "backlog",
            json!({"createdAt": "2026-11-01T00:00:00.000Z"}),
        ),
        card(
            "CLI-002",
            "story",
            "backlog",
            json!({"createdAt": "2026-11-03T00:00:00.000Z"}),
        ),
        card(
            "CLI-003",
            "story",
            "backlog",
            json!({"createdAt": "2026-11-02T00:00:00.000Z"}),
        ),
    ])?;
    let clock = brisbane()?;
    // @step When I list work units with "--sort createdAt --desc --limit 2 --format ids"
    let out = ok(
        dir.path(),
        json!({"sort": "createdAt", "desc": true, "limit": 2, "format": "ids"}),
        &clock,
    )?;
    // @step Then the output is the ids CLI-002 and CLI-003
    assert_eq!(ids(&out), strings(&["CLI-002", "CLI-003"]), "{out}");
    // @step When I list work units with "--sort createdAt --format ids"
    let out = ok(
        dir.path(),
        json!({"sort": "createdAt", "format": "ids"}),
        &clock,
    )?;
    // @step Then the output is the ids CLI-001, CLI-003 and CLI-002
    assert_eq!(
        ids(&out),
        strings(&["CLI-001", "CLI-003", "CLI-002"]),
        "{out}"
    );
    Ok(())
}

#[test]
fn a_large_board_answers_an_exact_query() -> TestResult {
    use chrono::{Duration, TimeZone};
    // @step Given a generated board of 2500 cards of about 13 MB with known done times
    let clock = brisbane()?;
    let now = utc(NOW)?;
    let filler = "x".repeat(5_000);
    let mut cards = Vec::with_capacity(2_500);
    let mut expected: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    let zone: chrono_tz::Tz = "Australia/Brisbane".parse().map_err(|e| format!("{e}"))?;
    for i in 0..2_500_i64 {
        let kind = if i % 3 == 0 { "bug" } else { "story" };
        let done_at = now - Duration::minutes(i * 37);
        let stamp = done_at.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        if kind == "bug" && done_at >= now - Duration::hours(48) {
            let day = zone
                .from_utc_datetime(&done_at.naive_utc())
                .format("%Y-%m-%d")
                .to_string();
            *expected.entry(day).or_insert(0) += 1;
        }
        cards.push(card(
            &format!("BIG-{:04}", i + 1),
            kind,
            "done",
            json!({"description": filler, "stateHistory": history(&[("specifying", "2026-01-01T00:00:00.000Z"), ("done", &stamp)])}),
        ));
    }
    let dir = board(&cards)?;
    let size = std::fs::metadata(dir.path().join("spec/work-units.json"))?.len();
    assert!(size > 12_000_000, "fixture board is {size} bytes");
    // @step When I list work units with "--type bug --done-since 2d --group-by day --format json"
    let out = ok(
        dir.path(),
        json!({"type": "bug", "doneSince": "2d", "groupBy": "day", "format": "json"}),
        &clock,
    )?;
    // @step Then the JSON group counts equal the counts computed from the generated cards
    let v = json_out(&out)?;
    let got: std::collections::BTreeMap<String, u64> = v["groups"]
        .as_array()
        .ok_or("no groups")?
        .iter()
        .filter_map(|g| Some((g["key"].as_str()?.to_string(), g["count"].as_u64()?)))
        .collect();
    assert!(!expected.is_empty());
    assert_eq!(got, expected);
    Ok(())
}

#[test]
fn the_workflow_docs_tell_agents_to_query_the_board_with_list_work_units() -> TestResult {
    // @step Given the init doc templates, their fixtures and the bootstrap doc
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let docs = [
        "src/commands/init_doc_plain_cli.md",
        "src/commands/init_doc_plain_ext.md",
        "src/commands/init_doc_plain_ide.md",
        "src/commands/init_doc_sysrem_cli.md",
        "src/commands/bootstrap_doc.txt",
        "tests/fixtures/init_docs/aider.md",
        "tests/fixtures/init_docs/claude.md",
        "tests/fixtures/init_docs/cline.md",
        "tests/fixtures/init_docs/cursor.md",
        "tests/fixtures/init_docs/gemini.md",
    ];
    for doc in docs {
        // @step When I read each of them
        let text = std::fs::read_to_string(root.join(doc))?;
        // @step Then each has a "Querying the board" section with list-work-units examples
        assert!(
            text.contains("### Querying the board\n"),
            "{doc}: no section"
        );
        assert!(
            text.contains("fspec list-work-units --status done --done-since 2d --group-by day"),
            "{doc}: no example"
        );
        // @step And each says never to read or parse spec/work-units.json directly with jq, grep or scripts
        assert!(
            text.contains(
                "Never read or parse `spec/work-units.json` directly (no `jq`, `grep` or scripts)"
            ),
            "{doc}: no rule"
        );
    }
    Ok(())
}
