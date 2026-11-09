use super::*;

#[test]
fn dispatcher_flags_a_work_unit_listed_in_the_wrong_state_array() {
    // Scenario: Dispatcher flags a work unit listed in the wrong state array

    // @step Given spec/work-units.json contains AUTH-001 with status 'done' that also appears in states.backlog
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": { "id": "AUTH-001", "title": "X", "status": "done" }
            },
            "states": {
                "backlog": ["AUTH-001"], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": ["AUTH-001"], "blocked": []
            }
        }),
    );

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result reports valid=false
    assert_eq!(data["valid"].as_bool(), Some(false), "got {data}");

    // @step Then the errors include a message containing "is in 'backlog' array"
    assert!(has_error(&data, "is in 'backlog' array"), "got {data}");
}

#[test]
fn dispatcher_flags_an_empty_string_in_the_rules_array() {
    // Scenario: Dispatcher flags an empty string in the rules array

    // @step Given spec/work-units.json contains AUTH-001 whose rules array contains an empty string at index 0
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": { "id": "AUTH-001", "title": "X", "status": "backlog", "rules": [""] }
            },
            "states": {
                "backlog": ["AUTH-001"], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result reports valid=false
    assert_eq!(data["valid"].as_bool(), Some(false), "got {data}");

    // @step Then the errors include 'AUTH-001: rules array contains empty strings or non-strings at index 0'
    assert!(
        has_exact_error(
            &data,
            "Work unit AUTH-001: rules array contains empty strings or non-strings at index 0"
        ),
        "got {data}"
    );
}

#[test]
fn dispatcher_flags_a_malformed_questions_item() {
    // Scenario: Dispatcher flags a malformed questions item

    // @step Given spec/work-units.json contains AUTH-001 whose questions array contains a string instead of an object
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": { "id": "AUTH-001", "title": "X", "status": "backlog", "questions": ["not an object"] }
            },
            "states": {
                "backlog": ["AUTH-001"], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result reports valid=false
    assert_eq!(data["valid"].as_bool(), Some(false), "got {data}");

    // @step Then the errors include a message containing 'questions[0] must be a QuestionItem object'
    assert!(
        has_error(&data, "questions[0] must be a QuestionItem object"),
        "got {data}"
    );
}

#[test]
fn dispatcher_flags_a_non_array_dependency_field() {
    // Scenario: Dispatcher flags a non-array dependency field

    // @step Given spec/work-units.json contains AUTH-001 whose blockedBy field is a string instead of an array
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": { "id": "AUTH-001", "title": "X", "status": "backlog", "blockedBy": "AUTH-002" }
            },
            "states": {
                "backlog": ["AUTH-001"], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result reports valid=false
    assert_eq!(data["valid"].as_bool(), Some(false), "got {data}");

    // @step Then the errors include 'AUTH-001: blockedBy must be an array'
    assert!(
        has_exact_error(&data, "Work unit AUTH-001: blockedBy must be an array"),
        "got {data}"
    );
}

#[test]
fn dispatcher_reports_schema_errors_for_a_missing_work_units_field() {
    // Scenario: Dispatcher reproduces the TS crash for a missing workUnits field
    //
    // The TypeScript reference does NOT short-circuit on the schema check; its
    // Check 2 calls `Object.keys(workUnitsData.workUnits)`, which throws a
    // TypeError when `workUnits` is absent. The `.action` catch renders this as
    // `✗ Failed to validate work units: Cannot convert undefined or null to
    // object` (exit 1). Parity requires the dispatcher to surface that exact
    // message as a failure rather than a structured "missing workUnits" error.

    // @step Given spec/work-units.json contains a states object but no workUnits field
    let tmp = TempDir::new().expect("tempdir");
    write_raw_work_units(
        tmp.path(),
        r#"{ "version": "0.7.1", "states": { "backlog": [], "specifying": [], "testing": [], "implementing": [], "validating": [], "done": [], "blocked": [] } }"#,
    );

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns failure with the message 'Cannot convert undefined or null to object'
    assert!(!result.success, "{result:?}");
    assert_eq!(
        result.error.as_deref(),
        Some("Cannot convert undefined or null to object"),
        "{result:?}"
    );
}

#[test]
fn dispatcher_treats_an_empty_workunits_array_as_valid() {
    // Scenario: Dispatcher treats an empty workUnits array as a valid (empty) store
    //
    // In JS `typeof [] === 'object'` and `Object.keys([])` returns `[]` without
    // throwing, so the TS reference reports `✓ All work units are valid`
    // (exit 0) for `workUnits: []`. Parity requires the Rust port to coerce the
    // array via `Object.entries` semantics rather than collapsing every
    // non-object value into the missing-workUnits TypeError.

    // @step Given spec/work-units.json contains an empty workUnits array and a states object
    let tmp = TempDir::new().expect("tempdir");
    write_raw_work_units(
        tmp.path(),
        r#"{ "version": "0.7.1", "workUnits": [], "states": { "backlog": [], "specifying": [], "testing": [], "implementing": [], "validating": [], "done": [], "blocked": [] } }"#,
    );

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result);

    // @step Then the result reports valid=true with no errors
    assert_eq!(data["valid"].as_bool(), Some(true), "got {data}");
    assert!(errors(&data).is_empty(), "expected no errors; got {data}");
}

#[test]
fn dispatcher_auto_creates_and_validates_an_empty_store_when_missing() {
    // Scenario: Dispatcher auto-creates and validates an empty store when work-units.json is missing

    // @step Given an empty project root with no spec/work-units.json
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/work-units.json").exists());

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result);

    // @step Then the result reports valid=true with no errors
    assert_eq!(data["valid"].as_bool(), Some(true), "got {data}");
    assert!(errors(&data).is_empty(), "expected no errors; got {data}");
}
