//! BUG-633 — assumptions can be removed and restored by stable id.
//!
//! Feature: spec/features/remove-and-restore-assumptions.feature

mod common;

use std::path::Path;
use std::process::{Command, Output};

use common::fspec_bin;
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const CARD: &str = "CARD-1";

/// A workspace whose board holds `CARD` in `status` with `assumptions`,
/// plus extra cards.
fn board(
    status: &str,
    assumptions: Value,
    extra: &[(&str, Value)],
) -> serde_json::Map<String, Value> {
    let mut units = serde_json::Map::new();
    let mut card = json!({
        "id": CARD, "title": "card", "type": "story", "status": status,
        "createdAt": "2026-06-01T00:00:00.000Z",
        "updatedAt": "2026-06-01T00:00:00.000Z",
        "assumptions": assumptions,
    });
    if let Some(next) = extra.iter().find(|(k, _)| *k == "nextAssumptionId") {
        card["nextAssumptionId"] = next.1.clone();
    }
    units.insert(CARD.to_owned(), card);
    units
}

fn write_board(root: &Path, units: serde_json::Map<String, Value>) -> TestResult {
    let mut states = serde_json::Map::new();
    for state in [
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        let ids: Vec<Value> = units
            .values()
            .filter(|u| u["status"] == state)
            .map(|u| u["id"].clone())
            .collect();
        states.insert(state.to_owned(), Value::Array(ids));
    }
    let spec = root.join("spec");
    std::fs::create_dir_all(&spec)?;
    std::fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(&json!({
            "version": "0.7.1",
            "workUnits": units,
            "states": states,
        }))?,
    )?;
    Ok(())
}

fn card(root: &Path, id: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(root.join("spec/work-units.json"))?;
    let data: Value = serde_json::from_str(&raw)?;
    Ok(data["workUnits"][id].clone())
}

fn fspec(root: &Path, args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(fspec_bin())
        .args(args)
        .current_dir(root)
        .output()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The `Assumptions:` block of `show-work-unit` text output.
fn listed_assumptions(root: &Path, id: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let output = fspec(root, &["show-work-unit", id])?;
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    Ok(text
        .split("\nAssumptions:\n")
        .nth(1)
        .map(|block| {
            block
                .lines()
                .take_while(|line| line.starts_with("  "))
                .map(|line| line.trim().to_owned())
                .collect()
        })
        .unwrap_or_default())
}

/// Move `CARD` to done (the CLI compacts only done cards).
fn mark_done(root: &Path) -> TestResult {
    let path = root.join("spec/work-units.json");
    let mut data: Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    data["workUnits"][CARD]["status"] = json!("done");
    for (state, ids) in data["states"].as_object_mut().ok_or("states")? {
        let ids = ids.as_array_mut().ok_or("state list")?;
        ids.retain(|id| id != CARD);
        if state == "done" {
            ids.push(json!(CARD));
        }
    }
    std::fs::write(&path, serde_json::to_string_pretty(&data)?)?;
    Ok(())
}

/// A card whose string assumptions "A" and "B" became items and whose
/// assumption 1 was removed.
fn removed_b(root: &Path) -> TestResult {
    write_board(root, board("specifying", json!(["A", "B"]), &[]))?;
    let output = fspec(root, &["remove-assumption", CARD, "1"])?;
    assert!(output.status.success(), "{}", stderr(&output));
    Ok(())
}

#[test]
fn removing_an_assumption_converts_the_card_s_string_assumptions_and_soft_deletes_it() -> TestResult
{
    // @step Given a card in specifying with string assumptions "A" and "B"
    let dir = tempfile::tempdir()?;
    write_board(dir.path(), board("specifying", json!(["A", "B"]), &[]))?;

    // @step When "remove-assumption CARD 1" runs
    let output = fspec(dir.path(), &["remove-assumption", CARD, "1"])?;
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("✓ Removed assumption: \"B\""),
        "{}",
        stdout(&output)
    );

    // @step Then the card stores assumption 0 "A" active and assumption 1 "B" deleted with a deletedAt, and nextAssumptionId 2
    let stored = card(dir.path(), CARD)?;
    let items = stored["assumptions"].as_array().ok_or("assumptions")?;
    assert_eq!(items.len(), 2, "{stored}");
    assert_eq!(items[0]["id"], 0);
    assert_eq!(items[0]["text"], "A");
    assert_eq!(items[0]["deleted"], false);
    assert!(items[0]["createdAt"].is_string(), "{stored}");
    assert_eq!(items[1]["id"], 1);
    assert_eq!(items[1]["text"], "B");
    assert_eq!(items[1]["deleted"], true);
    assert!(items[1]["deletedAt"].is_string(), "{stored}");
    assert_eq!(stored["nextAssumptionId"], 2);
    // @step And show-work-unit lists only "[0] A" under Assumptions
    assert_eq!(listed_assumptions(dir.path(), CARD)?, vec!["[0] A"]);
    Ok(())
}

#[test]
fn restoring_an_assumption_makes_it_active_again() -> TestResult {
    // @step Given a card whose assumption 1 "B" was removed
    let dir = tempfile::tempdir()?;
    removed_b(dir.path())?;

    // @step When "restore-assumption CARD 1" runs
    let output = fspec(dir.path(), &["restore-assumption", CARD, "1"])?;
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("✓ Restored assumption: \"B\""),
        "{}",
        stdout(&output)
    );

    // @step Then assumption 1 "B" is active with no deletedAt
    let stored = card(dir.path(), CARD)?;
    assert_eq!(stored["assumptions"][1]["deleted"], false, "{stored}");
    assert!(
        stored["assumptions"][1].get("deletedAt").is_none(),
        "{stored}"
    );
    // @step And show-work-unit lists "[0] A" and "[1] B"
    assert_eq!(
        listed_assumptions(dir.path(), CARD)?,
        vec!["[0] A", "[1] B"]
    );
    Ok(())
}

#[test]
fn adding_an_assumption_to_a_card_with_string_assumptions_continues_the_ids() -> TestResult {
    // @step Given a card in specifying with the string assumption "A"
    let dir = tempfile::tempdir()?;
    write_board(dir.path(), board("specifying", json!(["A"]), &[]))?;

    // @step When "fspec add-assumption CARD C" runs
    let output = fspec(dir.path(), &["add-assumption", CARD, "C"])?;
    assert!(output.status.success(), "{}", stderr(&output));

    // @step Then the card stores assumption 0 "A" and assumption 1 "C" as objects, and nextAssumptionId 2
    let stored = card(dir.path(), CARD)?;
    let items = stored["assumptions"].as_array().ok_or("assumptions")?;
    let ids_texts: Vec<(Value, Value, Value)> = items
        .iter()
        .map(|i| (i["id"].clone(), i["text"].clone(), i["deleted"].clone()))
        .collect();
    assert_eq!(
        ids_texts,
        vec![
            (json!(0), json!("A"), json!(false)),
            (json!(1), json!("C"), json!(false))
        ]
    );
    assert_eq!(stored["nextAssumptionId"], 2);
    Ok(())
}

#[test]
fn an_unknown_assumption_id_is_an_error() -> TestResult {
    // @step Given a card in specifying with two assumptions
    let dir = tempfile::tempdir()?;
    write_board(dir.path(), board("specifying", json!(["A", "B"]), &[]))?;
    let before = std::fs::read_to_string(dir.path().join("spec/work-units.json"))?;

    // @step When "remove-assumption CARD 5" runs, and then "restore-assumption CARD 5"
    let removed = fspec(dir.path(), &["remove-assumption", CARD, "5"])?;
    let restored = fspec(dir.path(), &["restore-assumption", CARD, "5"])?;

    // @step Then each fails with "Assumption with ID 5 not found" and the card is unchanged
    for output in [&removed, &restored] {
        assert_eq!(output.status.code(), Some(1), "{}", stderr(output));
        assert!(
            stderr(output).contains("Assumption with ID 5 not found"),
            "{}",
            stderr(output)
        );
    }
    let after = std::fs::read_to_string(dir.path().join("spec/work-units.json"))?;
    assert_eq!(before, after);
    Ok(())
}

#[test]
fn removing_twice_and_restoring_an_active_assumption_change_nothing() -> TestResult {
    // @step Given a card whose assumption 1 was removed
    let dir = tempfile::tempdir()?;
    removed_b(dir.path())?;
    let before = std::fs::read_to_string(dir.path().join("spec/work-units.json"))?;

    // @step When "remove-assumption CARD 1" runs again, and "restore-assumption CARD 0" runs
    let again = fspec(dir.path(), &["remove-assumption", CARD, "1"])?;
    let active = fspec(dir.path(), &["restore-assumption", CARD, "0"])?;

    // @step Then both succeed saying the assumption is already deleted or already active
    assert!(again.status.success(), "{}", stderr(&again));
    assert!(
        stdout(&again).contains("Item ID 1 already deleted"),
        "{}",
        stdout(&again)
    );
    assert!(active.status.success(), "{}", stderr(&active));
    assert!(
        stdout(&active).contains("Item ID 0 already active"),
        "{}",
        stdout(&active)
    );
    // @step And the card's file is not rewritten
    let after = std::fs::read_to_string(dir.path().join("spec/work-units.json"))?;
    assert_eq!(before, after);
    Ok(())
}

#[test]
fn assumptions_are_removed_and_restored_only_while_specifying() -> TestResult {
    // @step Given a card in testing with assumptions
    let dir = tempfile::tempdir()?;
    write_board(dir.path(), board("testing", json!(["A"]), &[]))?;

    // @step When "remove-assumption CARD 0" runs
    let output = fspec(dir.path(), &["remove-assumption", CARD, "0"])?;

    // @step Then it fails saying assumptions can only be removed during discovery/specification
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(
        stderr(&output)
            .contains("Can only remove assumptions during discovery/specification phase"),
        "{}",
        stderr(&output)
    );
    Ok(())
}

#[test]
fn show_deleted_lists_removed_assumptions_and_compaction_drops_them() -> TestResult {
    // @step Given a card whose assumption 1 "B" was removed
    let dir = tempfile::tempdir()?;
    removed_b(dir.path())?;
    let deleted_at = card(dir.path(), CARD)?["assumptions"][1]["deletedAt"]
        .as_str()
        .ok_or("deletedAt")?
        .to_owned();

    // @step When "fspec show-deleted CARD" runs and then, with the card done, "fspec compact-work-unit CARD"
    let shown = fspec(dir.path(), &["show-deleted", CARD])?;
    mark_done(dir.path())?;
    let compacted = fspec(dir.path(), &["compact-work-unit", CARD])?;

    // @step Then show-deleted lists assumption 1 "B" with its deletedAt
    assert!(shown.status.success(), "{}", stderr(&shown));
    assert!(
        stdout(&shown).contains(&format!("[1] B (deleted: {deleted_at})")),
        "{}",
        stdout(&shown)
    );
    // @step And after compaction the card stores only assumption 0 "A" and nextAssumptionId 1
    assert!(compacted.status.success(), "{}", stderr(&compacted));
    assert!(
        stdout(&compacted).contains("Assumptions: 1"),
        "{}",
        stdout(&compacted)
    );
    let stored = card(dir.path(), CARD)?;
    let items = stored["assumptions"].as_array().ok_or("assumptions")?;
    assert_eq!(items.len(), 1, "{stored}");
    assert_eq!(items[0]["id"], 0);
    assert_eq!(items[0]["text"], "A");
    assert_eq!(stored["nextAssumptionId"], 1);
    Ok(())
}

#[test]
fn readers_treat_string_and_object_assumptions_alike_and_skip_removed_ones() -> TestResult {
    // @step Given a card with string assumptions "A" and "B" and another card whose assumption 1 was removed
    let dir = tempfile::tempdir()?;
    let mut units = board("specifying", json!(["A", "B"]), &[]);
    units.insert(
        "CARD-2".to_owned(),
        json!({
            "id": "CARD-2", "title": "other", "type": "story", "status": "specifying",
            "createdAt": "2026-06-01T00:00:00.000Z",
            "updatedAt": "2026-06-01T00:00:00.000Z",
            "assumptions": [
                {"id": 0, "text": "A", "deleted": false, "createdAt": "2026-06-01T00:00:00.000Z"},
                {"id": 1, "text": "B", "deleted": true, "createdAt": "2026-06-01T00:00:00.000Z",
                 "deletedAt": "2026-06-02T00:00:00.000Z"}
            ],
            "nextAssumptionId": 2,
        }),
    );
    write_board(dir.path(), units)?;

    // @step When show-work-unit, list-work-units --has-assumptions --fields assumptions and validate-work-units read them
    let untouched = listed_assumptions(dir.path(), CARD)?;
    let other = listed_assumptions(dir.path(), "CARD-2")?;
    let listed = fspec(
        dir.path(),
        &[
            "list-work-units",
            "--has-assumptions",
            "--fields",
            "id,assumptions",
            "--format",
            "json",
        ],
    )?;
    let validated = fspec(dir.path(), &["validate-work-units"])?;

    // @step Then the untouched card shows "[0] A" and "[1] B", the other shows only "[0] A", and validate-work-units reports no error
    assert_eq!(untouched, vec!["[0] A", "[1] B"]);
    assert_eq!(other, vec!["[0] A"]);
    assert!(listed.status.success(), "{}", stderr(&listed));
    let listed_json: Value = serde_json::from_str(&stdout(&listed))?;
    let text = listed_json.to_string();
    assert!(text.contains("CARD-1") && text.contains("CARD-2"), "{text}");
    assert_eq!(
        text.matches("\"B\"").count() + text.matches("] B").count(),
        1,
        "{text}"
    );
    assert!(
        validated.status.success(),
        "{}{}",
        stdout(&validated),
        stderr(&validated)
    );
    Ok(())
}

#[test]
fn the_new_commands_have_help_pages() -> TestResult {
    let dir = tempfile::tempdir()?;
    // @step When "remove-assumption --help" and "restore-assumption --help" run
    let remove = fspec(dir.path(), &["remove-assumption", "--help"])?;
    let restore = fspec(dir.path(), &["restore-assumption", "--help"])?;

    // @step Then each prints its usage, arguments and an example
    for (output, name) in [
        (&remove, "remove-assumption"),
        (&restore, "restore-assumption"),
    ] {
        let text = stdout(output);
        assert!(output.status.success(), "{}", stderr(output));
        assert!(
            text.contains(&format!("fspec {name} <workUnitId> <index>")),
            "{text}"
        );
        assert!(text.contains("Assumption ID (stable index"), "{text}");
        assert!(
            text.contains(&format!("$ fspec {name} AUTH-001 1")),
            "{text}"
        );
    }
    // @step And "fspec add-assumption --help" and "fspec show-deleted --help" name remove-assumption and restore-assumption among related commands
    let add = stdout(&fspec(dir.path(), &["add-assumption", "--help"])?);
    assert!(
        add.contains("fspec remove-assumption") && add.contains("fspec restore-assumption"),
        "{add}"
    );
    let shown = stdout(&fspec(dir.path(), &["show-deleted", "--help"])?);
    assert!(shown.contains("fspec restore-assumption"), "{shown}");
    Ok(())
}
