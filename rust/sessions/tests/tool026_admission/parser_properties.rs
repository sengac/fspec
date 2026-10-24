use super::support::Result;
use codelet_tools::{execution_context::WorkUnitAttribution, tool_admission::split_arguments};
use proptest::{prelude::*, test_runner::TestRunner};
use serde_json::{json, Value};

pub fn verify() -> Result {
    TestRunner::default()
        .run(&(any::<String>(), any::<Vec<i64>>()), |(text, numbers)| {
            let payload =
                json!({"payload":{"work_unit_id":text,"values":numbers},"target_work_unit_id":"unchanged"});
            let mut input = payload.clone();
            input["work_unit_id"] = json!("AUTH-001");
            let (attribution, business) = split_arguments(&input.to_string())?;
            prop_assert_eq!(attribution, WorkUnitAttribution::Card("AUTH-001".into()));
            prop_assert_eq!(serde_json::from_str::<Value>(&business)?, payload);
            Ok(())
        })
        .map_err(|error| error.to_string())?;
    for input in [
        "null",
        "[]",
        "4",
        r#"{"work_unit_id":["NONE"]}"#,
        r#"{"work_unit_id":{"extra":true}}"#,
        r#"{"work_unit_id":"NONE","work_unit_id":"AUTH-001"}"#,
        r#"{"work_unit_id":"NONE","work_unit_id":"NONE"}"#,
        r#"{"work_unit_id":" NONE "}"#,
    ] {
        assert!(
            split_arguments(input).is_err(),
            "malformed metadata accepted: {input}"
        );
    }
    Ok(())
}
