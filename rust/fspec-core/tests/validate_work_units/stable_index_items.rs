use super::*;

fn store_with_rules(rules: Value, examples: Value) -> Value {
    json!({
        "version": "0.7.1",
        "workUnits": {
            "AUTH-001": {
                "id": "AUTH-001", "title": "X", "status": "backlog",
                "rules": rules, "examples": examples
            }
        },
        "states": {
            "backlog": ["AUTH-001"], "specifying": [], "testing": [],
            "implementing": [], "validating": [], "done": [], "blocked": []
        }
    })
}

#[test]
fn dispatcher_accepts_stable_index_rule_and_example_objects() {
    // @step Given spec/work-units.json contains AUTH-001 whose rules and examples are stable-index objects with id, text and deleted fields
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &store_with_rules(
            json!([
                {"id": 0, "text": "Tokens expire after an hour", "deleted": false, "createdAt": "2026-10-30T00:00:00Z"},
                {"id": 1, "text": "Refresh is single use", "deleted": true, "createdAt": "2026-10-30T00:00:00Z"}
            ]),
            json!([{"id": 0, "text": "An expired token is refused", "deleted": false}]),
        ),
    );

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result reports valid=true
    assert_eq!(data["valid"].as_bool(), Some(true), "got {data}");
}

#[test]
fn dispatcher_flags_a_stable_index_rule_with_empty_text() {
    // @step Given spec/work-units.json contains AUTH-001 whose rules array holds a live stable-index object with empty text at index 0 and a soft-deleted one with empty text at index 1
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &store_with_rules(
            json!([
                {"id": 0, "text": "  ", "deleted": false},
                {"id": 1, "text": "", "deleted": true}
            ]),
            json!([]),
        ),
    );

    // @step When I dispatch the validate-work-units command against that project root
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result);

    // @step Then the result reports valid=false
    assert_eq!(data["valid"].as_bool(), Some(false), "got {data}");

    // @step Then the errors include 'AUTH-001: rules[0] text must be a non-empty string'
    assert!(
        has_exact_error(
            &data,
            "Work unit AUTH-001: rules[0] text must be a non-empty string"
        ),
        "got {:?}",
        errors(&data)
    );

    // @step And no error is reported for the soft-deleted rule at index 1
    assert!(
        !errors(&data).iter().any(|error| error.contains("[1]")),
        "got {:?}",
        errors(&data)
    );
    assert_eq!(errors(&data).len(), 1, "got {:?}", errors(&data));
}
