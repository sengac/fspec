#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/create-prefix-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `create-prefix`
// (RPC-213). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "create-prefix".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

fn write_prefixes(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("prefixes.json"), raw).expect("write prefixes.json");
}

fn read_prefixes_raw(project_root: &Path) -> String {
    fs::read_to_string(project_root.join("spec/prefixes.json")).expect("read spec/prefixes.json")
}

const AUTH_ONLY_JSON: &str = r#"{
  "prefixes": {
    "AUTH": {
      "prefix": "AUTH",
      "description": "Auth features",
      "createdAt": "2026-06-01T00:00:00.000Z"
    }
  }
}"#;

// ---------- scenarios ----------

#[test]
fn successful_registration_creates_prefixes_file_with_new_entry() {
    // Scenario: Successful registration creates spec/prefixes.json with the new entry

    // @step Given an empty project root with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch create-prefix with args prefix='AUTH' and description='Auth features'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "prefix": "AUTH", "description": "Auth features" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then spec/prefixes.json exists and contains the prefix entry 'AUTH' with description 'Auth features'
    let on_disk: Value =
        serde_json::from_str(&read_prefixes_raw(tmp.path())).expect("on-disk JSON parse");
    let auth = &on_disk["prefixes"]["AUTH"];
    assert_eq!(auth["prefix"].as_str(), Some("AUTH"));
    assert_eq!(auth["description"].as_str(), Some("Auth features"));

    // @step Then the returned JSON includes a non-empty createdAt timestamp matching the ISO-8601 UTC format
    let data = parse_data(&result.data);
    let created_at = data["createdAt"]
        .as_str()
        .expect("createdAt field present and string");
    assert!(
        !created_at.is_empty(),
        "createdAt must be non-empty; got: {created_at}"
    );
    // Cheap ISO-8601 shape check: YYYY-MM-DDTHH:MM:SS.sssZ
    let re = regex_lite("^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}\\.[0-9]{3}Z$");
    assert!(
        re(created_at),
        "createdAt did not match ISO-8601 UTC shape; got: {created_at}"
    );

    // @step Then the returned JSON fields appear in the order success, prefix, description, createdAt
    let raw_keys = top_level_key_order(&result.data);
    assert_eq!(
        raw_keys,
        vec![
            "success".to_string(),
            "prefix".to_string(),
            "description".to_string(),
            "createdAt".to_string(),
        ],
        "field order must be success, prefix, description, createdAt; got: {raw_keys:?}\nraw: {}",
        result.data
    );
}

#[test]
fn lowercase_prefix_is_rejected_before_any_file_io() {
    // Scenario: Lowercase prefix is rejected before any file IO occurs

    // @step Given an empty project root with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch create-prefix with args prefix='auth' and description='bad case'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "prefix": "auth", "description": "bad case" }),
    ));

    // @step Then the dispatcher returns success=false with an error message that does NOT include the outer-catch wrap
    // Parity with `src/commands/create-prefix.ts:28-30`: the regex
    // validation throws OUTSIDE the outer try/catch, so the
    // `"Failed to create prefix: "` prefix is NOT applied to this path
    // (only to errors thrown inside the try block).
    assert!(!result.success, "expected success=false; got {result:?}");
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        !msg.contains("Failed to create prefix"),
        "regex-validation error must NOT include the outer-catch wrap; got: {msg}"
    );

    // @step Then the error message contains the substring 'Prefix must be 2-6 uppercase letters'
    assert!(
        msg.contains("Prefix must be 2-6 uppercase letters"),
        "error must mention 'Prefix must be 2-6 uppercase letters'; got: {msg}"
    );

    // @step Then spec/prefixes.json does not exist after the call
    assert!(
        !tmp.path().join("spec/prefixes.json").exists(),
        "spec/prefixes.json must NOT be created on validation failure"
    );
}

#[test]
fn prefix_shorter_than_two_is_rejected() {
    // Scenario: Prefix shorter than two characters is rejected

    // @step Given an empty project root with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch create-prefix with args prefix='A' and description='too short'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "prefix": "A", "description": "too short" }),
    ));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Prefix must be 2-6 uppercase letters'
    assert!(!result.success, "expected success=false; got {result:?}");
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        msg.contains("Prefix must be 2-6 uppercase letters"),
        "error must mention 'Prefix must be 2-6 uppercase letters'; got: {msg}"
    );

    // @step Then spec/prefixes.json does not exist after the call
    assert!(!tmp.path().join("spec/prefixes.json").exists());
}

#[test]
fn prefix_longer_than_six_is_rejected() {
    // Scenario: Prefix longer than six characters is rejected

    // @step Given an empty project root with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch create-prefix with args prefix='ABCDEFG' and description='too long'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "prefix": "ABCDEFG", "description": "too long" }),
    ));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Prefix must be 2-6 uppercase letters'
    assert!(!result.success, "expected success=false; got {result:?}");
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        msg.contains("Prefix must be 2-6 uppercase letters"),
        "error must mention 'Prefix must be 2-6 uppercase letters'; got: {msg}"
    );

    // @step Then spec/prefixes.json does not exist after the call
    assert!(!tmp.path().join("spec/prefixes.json").exists());
}

#[test]
fn prefix_with_digits_is_rejected() {
    // Scenario: Prefix containing digits is rejected

    // @step Given an empty project root with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch create-prefix with args prefix='AB1' and description='has digit'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "prefix": "AB1", "description": "has digit" }),
    ));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Prefix must be 2-6 uppercase letters'
    assert!(!result.success, "expected success=false; got {result:?}");
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        msg.contains("Prefix must be 2-6 uppercase letters"),
        "error must mention 'Prefix must be 2-6 uppercase letters'; got: {msg}"
    );

    // @step Then spec/prefixes.json does not exist after the call
    assert!(!tmp.path().join("spec/prefixes.json").exists());
}

#[test]
fn duplicate_prefix_is_rejected_and_file_is_unchanged() {
    // Scenario: Duplicate prefix is rejected and the existing file is left untouched

    // @step Given spec/prefixes.json contains AUTH (description 'Auth features')
    let tmp = TempDir::new().expect("tempdir");
    write_prefixes(tmp.path(), AUTH_ONLY_JSON);
    let before = read_prefixes_raw(tmp.path());

    // @step When I dispatch create-prefix with args prefix='AUTH' and description='Different desc'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "prefix": "AUTH", "description": "Different desc" }),
    ));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Prefix AUTH already exists'
    assert!(!result.success, "expected success=false; got {result:?}");
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        msg.contains("Prefix AUTH already exists"),
        "error must mention 'Prefix AUTH already exists'; got: {msg}"
    );

    // @step Then spec/prefixes.json is byte-identical to its pre-call content
    let after = read_prefixes_raw(tmp.path());
    assert_eq!(
        before, after,
        "spec/prefixes.json must be untouched on duplicate error"
    );
}

#[test]
fn appending_second_prefix_preserves_insertion_order() {
    // Scenario: Appending a second prefix preserves insertion order

    // @step Given spec/prefixes.json contains AUTH (description 'Auth features')
    let tmp = TempDir::new().expect("tempdir");
    write_prefixes(tmp.path(), AUTH_ONLY_JSON);

    // @step When I dispatch create-prefix with args prefix='UI' and description='User interface'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "prefix": "UI", "description": "User interface" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then spec/prefixes.json contains both AUTH and UI as keys
    let after_raw = read_prefixes_raw(tmp.path());
    let after: Value = serde_json::from_str(&after_raw).expect("parse on-disk");
    let prefixes = after["prefixes"].as_object().expect("prefixes object");
    assert!(prefixes.contains_key("AUTH"), "AUTH key missing");
    assert!(prefixes.contains_key("UI"), "UI key missing");

    // @step Then in the on-disk JSON the AUTH entry appears before the UI entry
    let auth_pos = after_raw.find("\"AUTH\"").expect("AUTH key present in raw");
    let ui_pos = after_raw.find("\"UI\"").expect("UI key present in raw");
    assert!(
        auth_pos < ui_pos,
        "AUTH must appear before UI in on-disk JSON; AUTH={auth_pos} UI={ui_pos}\n{after_raw}"
    );
}

#[test]
fn malformed_prefixes_json_escalates_parse_error() {
    // Scenario: Malformed prefixes.json escalates a structured parse error

    // @step Given spec/prefixes.json exists but contains the malformed bytes '{ not valid json'
    let tmp = TempDir::new().expect("tempdir");
    write_prefixes(tmp.path(), "{ not valid json");
    let before = read_prefixes_raw(tmp.path());

    // @step When I dispatch create-prefix with args prefix='AUTH' and description='Auth features'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "prefix": "AUTH", "description": "Auth features" }),
    ));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse prefixes.json'
    assert!(!result.success, "expected success=false; got {result:?}");
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        msg.contains("Failed to parse prefixes.json"),
        "error must mention 'Failed to parse prefixes.json'; got: {msg}"
    );

    // @step Then spec/prefixes.json is byte-identical to its pre-call content
    let after = read_prefixes_raw(tmp.path());
    assert_eq!(before, after, "malformed file must not be overwritten");
}

#[test]
fn successful_dispatcher_returns_canonical_json_shape() {
    // Scenario: Successful dispatcher path returns the canonical JSON shape

    // @step Given an empty project root with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch create-prefix with args prefix='AUTH' and description='Auth features'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "prefix": "AUTH", "description": "Auth features" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then the DispatchResult.data parses as JSON whose root object has fields success=true, prefix='AUTH', description='Auth features', and a createdAt string
    let data = parse_data(&result.data);
    assert_eq!(data["success"].as_bool(), Some(true));
    assert_eq!(data["prefix"].as_str(), Some("AUTH"));
    assert_eq!(data["description"].as_str(), Some("Auth features"));
    assert!(
        data["createdAt"].is_string(),
        "createdAt must be a string; got {data:?}"
    );

    // @step Then the createdAt field value matches the regex '^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}.[0-9]{3}Z$'
    let created_at = data["createdAt"].as_str().unwrap();
    let re = regex_lite("^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}\\.[0-9]{3}Z$");
    assert!(
        re(created_at),
        "createdAt did not match ISO-8601 shape; got: {created_at}"
    );
}

#[test]
fn shared_infrastructure_is_reused_without_duplication() {
    // Scenario: Shared infrastructure is reused without duplication

    // @step Given the codelet/fspec-core crate is built
    // (precondition — this test only runs when the crate compiles)

    // @step When I inspect codelet/fspec-core/src/commands/create_prefix.rs
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands/create_prefix.rs");
    let src = fs::read_to_string(&path).expect("read create_prefix.rs");

    // @step Then the source declares it uses `ensure_prefixes_file` and `write_json_atomic` from the shared io modules
    assert!(
        src.contains("ensure_prefixes_file"),
        "create_prefix.rs must reference `ensure_prefixes_file`; got:\n{src}"
    );
    assert!(
        src.contains("write_json_atomic"),
        "create_prefix.rs must reference `write_json_atomic`; got:\n{src}"
    );

    // @step Then the source does NOT contain the substring 'FspecCoreError::NotYetPorted'
    assert!(
        !src.contains("FspecCoreError::NotYetPorted"),
        "create_prefix.rs must no longer be a NotYetPorted stub"
    );

    // @step Then the source does NOT inline any std::fs::write or serde_json::to_writer call for spec/prefixes.json
    assert!(
        !src.contains("std::fs::write"),
        "create_prefix.rs must NOT call std::fs::write directly — use the shared write helper"
    );
    assert!(
        !src.contains("serde_json::to_writer"),
        "create_prefix.rs must NOT call serde_json::to_writer directly — use the shared write helper"
    );
}

// ---------- tiny inline helpers (test-only) ----------

/// A minimal regex-like predicate used by the createdAt shape check.
/// We avoid pulling in the `regex` crate just for tests; the pattern set
/// is fixed and known.
fn regex_lite(pat: &str) -> impl Fn(&str) -> bool {
    // Hand-roll for the only pattern we use:
    // ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}\.[0-9]{3}Z$
    // (millisecond fraction — matches TS `new Date().toISOString()`)
    let expected = pat.to_string();
    move |s: &str| -> bool {
        if expected != "^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}\\.[0-9]{3}Z$" {
            panic!("unsupported test regex: {expected}");
        }
        if s.len() != 24 {
            return false;
        }
        let bytes = s.as_bytes();
        // YYYY-MM-DDTHH:MM:SS.sssZ
        let digit = |i: usize| bytes[i].is_ascii_digit();
        digit(0)
            && digit(1)
            && digit(2)
            && digit(3)
            && bytes[4] == b'-'
            && digit(5)
            && digit(6)
            && bytes[7] == b'-'
            && digit(8)
            && digit(9)
            && bytes[10] == b'T'
            && digit(11)
            && digit(12)
            && bytes[13] == b':'
            && digit(14)
            && digit(15)
            && bytes[16] == b':'
            && digit(17)
            && digit(18)
            && bytes[19] == b'.'
            && digit(20)
            && digit(21)
            && digit(22)
            && bytes[23] == b'Z'
    }
}

/// Extract the order of top-level keys from a pretty-printed JSON object.
/// We need this because `serde_json::Value::Object` is alphabetical
/// (`BTreeMap`); but the on-disk pretty JSON preserves the producer's
/// field order. Crude byte-scan: track brace depth; at depth==1 a
/// `"key":` is a top-level key.
fn top_level_key_order(json: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let bytes = json.as_bytes();
    let mut depth: i32 = 0;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'{' | b'[' => {
                depth += 1;
                i += 1;
            }
            b'}' | b']' => {
                depth -= 1;
                i += 1;
            }
            b'"' if depth == 1 => {
                // Found a string candidate; consume up to the unescaped
                // closing quote.
                let mut j = i + 1;
                let mut buf = String::new();
                while j < bytes.len() && bytes[j] != b'"' {
                    if bytes[j] == b'\\' && j + 1 < bytes.len() {
                        buf.push(bytes[j + 1] as char);
                        j += 2;
                        continue;
                    }
                    buf.push(bytes[j] as char);
                    j += 1;
                }
                i = j + 1;
                // Look ahead for `:` (whitespace tolerant) to confirm
                // this string was a key (not a value).
                let mut k = i;
                while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                    k += 1;
                }
                if k < bytes.len() && bytes[k] == b':' {
                    keys.push(buf);
                }
            }
            _ => {
                i += 1;
            }
        }
    }
    keys
}
