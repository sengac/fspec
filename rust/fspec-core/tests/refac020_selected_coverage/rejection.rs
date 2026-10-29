use crate::support::*;
use serde_json::{json, Value};

#[test]
fn invalid_or_stale_selector_leaves_exact_sidecar_bytes_unchanged() -> TestResult {
    // @step Given a valid duplicate-row coverage sidecar
    let temporary = fixture()?;
    let valid = indexed_link(temporary.path(), 1, "src/second.rs")?;
    let original = std::fs::read(temporary.path().join(SIDECAR))?;
    let cases = [
        ("stale", "expectedScenarioHash", json!("0".repeat(64))),
        ("malformed", "expectedScenarioHash", json!("not-a-hash")),
        ("wrong length", "expectedScenarioHash", json!("a".repeat(63))),
        ("wrong file", "testFile", json!("src/first.rs")),
        ("out of bounds", "testMappingIndex", json!(50)),
        ("negative", "testMappingIndex", json!(-1)),
        ("fractional", "testMappingIndex", json!(1.5)),
    ];
    let mut requests: Vec<(&str, Value)> = cases.into_iter().map(|(label, key, value)| {
        let mut args = valid.clone();
        args[key] = value;
        (label, args)
    }).collect();
    for key in ["expectedScenarioHash", "testMappingIndex"] {
        let mut args = valid.clone();
        args.as_object_mut().ok_or("argument object")?.remove(key);
        requests.push((key, args));
    }
    let mut append = valid.clone();
    append["testLines"] = json!("1-7");
    requests.push(("indexed append", append));
    let mut selected_without_impl = valid.clone();
    selected_without_impl.as_object_mut().ok_or("arguments")?.remove("implFile");
    requests.push(("invalid indexed mode", selected_without_impl));

    // @step When stale malformed mismatched unpaired or incompatible selectors are submitted
    // @step Then every rejected request leaves exact sidecar bytes unchanged
    for (label, args) in requests {
        let linked = request(temporary.path(), "link-coverage", args.clone());
        assert!(!linked.success, "{label} unexpectedly linked: {linked:?}");
        assert_eq!(std::fs::read(temporary.path().join(SIDECAR))?, original, "{label}");
        if !matches!(label, "indexed append" | "invalid indexed mode") {
            let mut unlink_args = args;
            unlink_args.as_object_mut().ok_or("argument object")?.remove("implFile");
            let unlinked = request(temporary.path(), "unlink-coverage", unlink_args);
            assert!(!unlinked.success, "{label} unexpectedly unlinked: {unlinked:?}");
            assert_eq!(std::fs::read(temporary.path().join(SIDECAR))?, original, "{label}");
        }
    }
    let mut all = valid;
    all["all"] = json!(true);
    let result = request(temporary.path(), "unlink-coverage", all);
    assert!(!result.success, "index plus all must not select a deletion scope");
    assert_eq!(std::fs::read(temporary.path().join(SIDECAR))?, original);
    Ok(())
}
