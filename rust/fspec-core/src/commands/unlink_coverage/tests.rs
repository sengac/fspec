#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn args_parse_defaults() {
    let a: UnlinkCoverageArgs = serde_json::from_str("{}").unwrap();
    assert!(a.feature_name.is_none());
    assert!(a.scenario.is_none());
    assert!(a.all.is_none());
}

#[test]
fn args_parse_full() {
    let a: UnlinkCoverageArgs = serde_json::from_str(
        r#"{"featureName":"user-login","scenario":"Login","testFile":"t.ts","implFile":"i.ts","all":false}"#,
    )
    .unwrap();
    assert_eq!(a.feature_name.as_deref(), Some("user-login"));
    assert_eq!(a.scenario.as_deref(), Some("Login"));
    assert_eq!(a.test_file.as_deref(), Some("t.ts"));
    assert_eq!(a.impl_file.as_deref(), Some("i.ts"));
    assert_eq!(a.all, Some(false));
}

#[test]
fn update_stats_creates_missing_stats() {
    // BUG-608: unlink-coverage writes stats with link-coverage's definition,
    // which creates a missing stats block.
    let mut cov = json!({
        "scenarios": [ { "name": "S", "testMappings": [] } ]
    });
    update_stats(&mut cov);
    assert_eq!(cov["stats"]["totalScenarios"], 1);
    assert_eq!(cov["stats"]["coveredScenarios"], 0);
}

#[test]
fn update_stats_counts_lines_and_files() {
    let mut cov = json!({
        "stats": { "totalScenarios": 1 },
        "scenarios": [ { "name": "Login", "testMappings": [
            { "file": "t.ts", "lines": "1-10", "implMappings": [ { "file": "i.ts", "lines": [1,2,3] } ] }
        ] } ]
    });
    update_stats(&mut cov);
    let stats = cov.get("stats").unwrap();
    assert_eq!(stats.get("coveredScenarios").unwrap(), 1);
    assert_eq!(stats.get("coveragePercent").unwrap(), 100);
    assert_eq!(stats.get("testFiles").unwrap(), &json!(["t.ts"]));
    assert_eq!(stats.get("implFiles").unwrap(), &json!(["i.ts"]));
    // 10 test lines (1-10 inclusive) + 3 impl lines (array length).
    assert_eq!(stats.get("totalLinesCovered").unwrap(), 13);
}

#[test]
fn update_stats_preserves_total_scenarios_verbatim() {
    let mut cov = json!({
        "stats": { "totalScenarios": 2 },
        "scenarios": [
            { "name": "A", "testMappings": [ { "file": "t.ts", "lines": "1-3", "implMappings": [] } ] },
            { "name": "B", "testMappings": [] }
        ]
    });
    update_stats(&mut cov);
    let stats = cov.get("stats").unwrap();
    assert_eq!(stats.get("totalScenarios").unwrap(), 2);
    assert_eq!(stats.get("coveragePercent").unwrap(), 50);
}

#[test]
fn update_stats_impl_lines_string_counts_chars() {
    let mut cov = json!({
        "stats": { "totalScenarios": 1 },
        "scenarios": [ { "name": "S", "testMappings": [
            { "file": "t.ts", "lines": "1-3", "implMappings": [ { "file": "i.ts", "lines": "10-20" } ] }
        ] } ]
    });
    update_stats(&mut cov);
    let stats = cov.get("stats").unwrap();
    // 3 test lines + 5 chars ("10-20").
    assert_eq!(stats.get("totalLinesCovered").unwrap(), 8);
}
