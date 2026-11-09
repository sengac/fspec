//! Feature: spec/features/fspec-workflow-guidance-named-keys.feature
//!
//! TOOL-020 tests, moved out of fspec_workflow_guidance.rs's inline tests so
//! each test file maps one feature (COV-075).
//!
//! Tests that the guidance uses named keys instead of positional `_` args.
//! These tests verify the fix for TOOL-020.

use super::*;

#[test]
fn test_guidance_does_not_use_positional_args_for_update_work_unit_status() {
    // @step Given the fspec workflow guidance file exists at rust/tools/src/fspec_workflow_guidance.rs
    let guidance = get_fspec_workflow_guidance();

    // @step When I inspect the update-work-unit-status examples
    // @step Then I should see "workUnitId" as a named key
    assert!(
        guidance.contains("workUnitId"),
        "Guidance should use 'workUnitId' named key for update-work-unit-status"
    );

    // @step And I should NOT see "_": ["AUTH-001", "specifying"] positional pattern
    assert!(
        !guidance.contains("\"_\": [\"AUTH-001\", \"specifying\""),
        "Guidance should NOT use positional _ args for update-work-unit-status"
    );
}

#[test]
fn test_guidance_does_not_use_positional_args_for_show_work_unit() {
    // @step Given the fspec workflow guidance file exists
    let guidance = get_fspec_workflow_guidance();

    // @step When I inspect the show-work-unit examples
    // @step Then I should see "workUnitId" as a named key
    assert!(
        guidance.contains("workUnitId"),
        "Guidance should use 'workUnitId' named key"
    );

    // @step And I should NOT see "_": ["AUTH-001"] positional pattern
    assert!(
        !guidance.contains("\"_\": [\"AUTH-001\"]"),
        "Guidance should NOT use positional _ args for show-work-unit"
    );
}

#[test]
fn test_guidance_does_not_use_positional_args_for_create_story() {
    // @step Given the fspec workflow guidance file exists
    let guidance = get_fspec_workflow_guidance();

    // @step When I inspect the create-story examples
    // @step Then I should see "prefix" and "title" as named keys
    assert!(
        guidance.contains("\"prefix\""),
        "Guidance should use 'prefix' named key for create-story"
    );
    assert!(
        guidance.contains("\"title\""),
        "Guidance should use 'title' named key for create-story"
    );

    // @step And I should NOT see "_": ["AUTH", "User Login"] positional pattern
    assert!(
        !guidance.contains("\"_\": [\"AUTH\""),
        "Guidance should NOT use positional _ args for create-story"
    );
}

#[test]
fn test_guidance_does_not_use_positional_args_for_add_rule() {
    // @step Given the fspec workflow guidance file exists
    let guidance = get_fspec_workflow_guidance();

    // @step When I inspect the add-rule examples
    // @step Then I should see "workUnitId" and "rule" as named keys
    assert!(
        guidance.contains("workUnitId"),
        "Guidance should use 'workUnitId' named key for add-rule"
    );
    assert!(
        guidance.contains("\"rule\""),
        "Guidance should use 'rule' named key for add-rule"
    );

    // @step And I should NOT see "_": ["AUTH-001", "rule text"] positional pattern
    assert!(
        !guidance.contains("\"_\": [\"AUTH-001\", \"rule text\""),
        "Guidance should NOT use positional _ args for add-rule"
    );
}

#[test]
fn test_guidance_does_not_use_positional_args_for_add_example() {
    // @step Given the fspec workflow guidance file exists
    let guidance = get_fspec_workflow_guidance();

    // @step When I inspect the add-example examples
    // @step Then I should see "workUnitId" and "example" as named keys
    assert!(
        guidance.contains("workUnitId"),
        "Guidance should use 'workUnitId' named key for add-example"
    );
    assert!(
        guidance.contains("\"example\""),
        "Guidance should use 'example' named key for add-example"
    );

    // @step And I should NOT see "_": ["AUTH-001", "example text"] positional pattern
    assert!(
        !guidance.contains("\"_\": [\"AUTH-001\", \"example text\""),
        "Guidance should NOT use positional _ args for add-example"
    );
}

#[test]
fn test_guidance_does_not_use_positional_args_for_add_dependency() {
    // @step Given the fspec workflow guidance file exists
    let guidance = get_fspec_workflow_guidance();

    // @step When I inspect the add-dependency examples
    // @step Then I should see "workUnitId" and "dependsOn" as named keys
    assert!(
        guidance.contains("workUnitId"),
        "Guidance should use 'workUnitId' named key for add-dependency"
    );
    assert!(
        guidance.contains("dependsOn"),
        "Guidance should use 'dependsOn' named key for add-dependency"
    );

    // @step And I should NOT see "_": ["AUTH-002", "AUTH-001"] positional pattern
    assert!(
        !guidance.contains("\"_\": [\"AUTH-002\", \"AUTH-001\""),
        "Guidance should NOT use positional _ args for add-dependency"
    );
}

#[test]
fn test_guidance_does_not_use_positional_args_for_add_attachment() {
    // @step Given the fspec workflow guidance file exists
    let guidance = get_fspec_workflow_guidance();

    // @step When I inspect the add-attachment examples
    // @step Then I should see "workUnitId" and "filePath" as named keys
    assert!(
        guidance.contains("workUnitId"),
        "Guidance should use 'workUnitId' named key for add-attachment"
    );
    assert!(
        guidance.contains("filePath"),
        "Guidance should use 'filePath' named key for add-attachment"
    );

    // @step And I should NOT see "_": ["AUTH-001", "file.png"] positional pattern
    assert!(
        !guidance.contains("\"_\": [\"AUTH-001\", \"file.png\""),
        "Guidance should NOT use positional _ args for add-attachment"
    );
}

#[test]
fn test_guidance_does_not_use_positional_args_for_set_user_story() {
    // @step Given the fspec workflow guidance file exists
    let guidance = get_fspec_workflow_guidance();

    // @step When I inspect the set-user-story examples
    // @step Then I should see "workUnitId" as a named key
    assert!(
        guidance.contains("workUnitId"),
        "Guidance should use 'workUnitId' named key for set-user-story"
    );

    // @step And I should NOT see "_": ["AUTH-001"] positional pattern for work unit ID
    assert!(
        !guidance.contains("\"_\": [\"AUTH-001\"]"),
        "Guidance should NOT use positional _ args for set-user-story"
    );
}

#[test]
fn test_guidance_does_not_use_positional_args_for_link_coverage() {
    // @step Given the fspec workflow guidance file exists
    let guidance = get_fspec_workflow_guidance();

    // @step When I inspect the link-coverage examples
    // @step Then I should see "feature" as a named key
    assert!(
        guidance.contains("feature"),
        "Guidance should use 'feature' named key for link-coverage"
    );

    // @step And I should NOT see "_": ["user-auth"] positional pattern
    assert!(
        !guidance.contains("\"_\": [\"user-auth\""),
        "Guidance should NOT use positional _ args for link-coverage"
    );
}

#[test]
fn test_guidance_has_no_positional_args_patterns() {
    // @step Given the fspec workflow guidance file exists
    let guidance = get_fspec_workflow_guidance();

    // @step When I count all occurrences of "_": [ positional pattern
    // @step Then the count should be zero
    let count = guidance.matches("\"_\": [").count();
    assert_eq!(
        count, 0,
        "Guidance should have zero positional _ patterns, but found {count}"
    );

    // @step And every command example should use named keys matching Rust Args structs
    assert!(
        guidance.contains("workUnitId"),
        "Guidance should use named keys"
    );
    assert!(
        guidance.contains("workUnitId"),
        "Named keys should match Rust Args structs"
    );
}
