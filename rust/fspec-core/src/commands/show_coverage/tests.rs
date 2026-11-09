#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn args_parse_with_defaults() {
    let a: ShowCoverageArgs = serde_json::from_str("{}").unwrap();
    assert!(a.feature_name.is_none());
    assert!(a.format.is_none());
    assert!(a.output.is_none());
}

#[test]
fn args_parse_camel_case() {
    let a: ShowCoverageArgs =
        serde_json::from_str(r#"{"featureName":"auth","format":"json"}"#).unwrap();
    assert_eq!(a.feature_name.as_deref(), Some("auth"));
    assert_eq!(a.format.as_deref(), Some("json"));
}

#[test]
fn coverage_status_uncovered_when_no_test_mappings() {
    let s = CoverageScenario {
        name: "X".into(),
        test_mappings: vec![],
        extra: Default::default(),
    };
    assert_eq!(coverage_status(&s), CoverageStatusKind::Uncovered);
}
