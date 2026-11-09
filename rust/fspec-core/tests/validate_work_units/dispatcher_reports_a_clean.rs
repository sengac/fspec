use super::*;

#[test]
fn dispatcher_reports_a_clean_store_as_valid() {
    // Scenario: Dispatcher reports a clean store as valid

    // @step Given spec/work-units.json contains consistent work units with matching parent/child links, valid statuses, and correct state arrays
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": { "id": "AUTH-001", "title": "Parent", "status": "done", "children": ["AUTH-002"] },
                "AUTH-002": { "id": "AUTH-002", "title": "Child", "status": "backlog", "parent": "AUTH-001" }
            },
            "states": {
                "backlog": ["AUTH-002"], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": ["AUTH-001"], "blocked": []
            }
        }),
    );

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result);

    // @step Then the result reports valid=true with no errors
    assert_eq!(data["valid"].as_bool(), Some(true), "got {data}");
    assert!(errors(&data).is_empty(), "expected no errors; got {data}");

    // @step Then the result checks list includes schema, uniqueIds, parentChild, exampleMapping, and dependencies
    let checks: Vec<String> = data["checks"]
        .as_array()
        .expect("checks array")
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    for expected in [
        "schema",
        "uniqueIds",
        "parentChild",
        "exampleMapping",
        "dependencies",
    ] {
        assert!(
            checks.contains(&expected.to_string()),
            "checks missing {expected}: {data}"
        );
    }
}

#[test]
fn dispatcher_flags_a_work_unit_whose_parent_is_missing() {
    // Scenario: Dispatcher reproduces the TS crash for a work unit with a
    // missing parent.
    //
    // The TypeScript reference pushes a "references non-existent parent" error
    // but does NOT `continue`; it then dereferences
    // `workUnitsData.workUnits[parent].children` on the `undefined` parent,
    // throwing a TypeError. The `.action` catch renders this as
    // `✗ Failed to validate work units: Cannot read properties of undefined
    // (reading 'children')` (exit 1). Parity requires the dispatcher to surface
    // that exact message as a failure rather than the structured error.

    // @step Given spec/work-units.json contains AUTH-002 with parent AUTH-001 but no AUTH-001 entry
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-002": { "id": "AUTH-002", "title": "Child", "status": "backlog", "parent": "AUTH-001" }
            },
            "states": {
                "backlog": ["AUTH-002"], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": []
            }
        }),
    );

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns failure with the message 'Cannot read properties of undefined (reading 'children')'
    assert!(!result.success, "{result:?}");
    assert_eq!(
        result.error.as_deref(),
        Some("Cannot read properties of undefined (reading 'children')"),
        "{result:?}"
    );
}

#[test]
fn dispatcher_flags_a_parent_that_does_not_list_its_child() {
    // Scenario: Dispatcher flags a parent that does not list its child

    // @step Given spec/work-units.json contains AUTH-001 with no children and AUTH-002 whose parent is AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": { "id": "AUTH-001", "title": "Parent", "status": "done" },
                "AUTH-002": { "id": "AUTH-002", "title": "Child", "status": "backlog", "parent": "AUTH-001" }
            },
            "states": {
                "backlog": ["AUTH-002"], "specifying": [], "testing": [],
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

    // @step Then the errors include "Work unit AUTH-002 has parent AUTH-001, but parent doesn't list it as a child"
    assert!(
        has_exact_error(
            &data,
            "Work unit AUTH-002 has parent AUTH-001, but parent doesn't list it as a child"
        ),
        "got {data}"
    );
}

#[test]
fn dispatcher_flags_a_missing_child_reference() {
    // Scenario: Dispatcher flags a missing child reference

    // @step Given spec/work-units.json contains AUTH-001 whose children list AUTH-099 which does not exist
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": { "id": "AUTH-001", "title": "Parent", "status": "done", "children": ["AUTH-099"] }
            },
            "states": {
                "backlog": [], "specifying": [], "testing": [],
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

    // @step Then the errors include 'Work unit AUTH-001 references non-existent child: AUTH-099'
    assert!(
        has_exact_error(
            &data,
            "Work unit AUTH-001 references non-existent child: AUTH-099"
        ),
        "got {data}"
    );
}

#[test]
fn dispatcher_flags_an_invalid_status_value() {
    // Scenario: Dispatcher flags an invalid status value

    // @step Given spec/work-units.json contains AUTH-001 with status 'review'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": { "id": "AUTH-001", "title": "X", "status": "review" }
            },
            "states": {
                "backlog": [], "specifying": [], "testing": [],
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

    // @step Then the errors include a message starting with 'Invalid status value for AUTH-001: review'
    assert!(
        errors(&data)
            .iter()
            .any(|e| e.starts_with("Invalid status value for AUTH-001: review")),
        "got {data}"
    );
}

#[test]
fn dispatcher_flags_a_work_unit_absent_from_its_state_array() {
    // Scenario: Dispatcher flags a work unit absent from its state array

    // @step Given spec/work-units.json contains AUTH-001 with status 'done' but states.done does not include AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &json!({
            "version": "0.7.1",
            "workUnits": {
                "AUTH-001": { "id": "AUTH-001", "title": "X", "status": "done" }
            },
            "states": {
                "backlog": [], "specifying": [], "testing": [],
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

    // @step Then the errors include a message containing 'has status' and 'is not in states.done array'
    assert!(
        errors(&data)
            .iter()
            .any(|e| e.contains("has status") && e.contains("is not in states.done array")),
        "got {data}"
    );
}
