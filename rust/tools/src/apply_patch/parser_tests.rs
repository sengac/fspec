use super::{parse_patch, Hunk, PatchOp};

#[test]
fn test_parse_add_file() -> Result<(), String> {
    let patch = "*** Begin Patch\n*** Add File: /tmp/test/new_file.rs\n+fn main() {\n+    println!(\"hello\");\n+}\n*** End Patch";
    assert_eq!(
        parse_patch(patch)?,
        vec![PatchOp::Add {
            path: "/tmp/test/new_file.rs".into(),
            lines: vec![
                "fn main() {".into(),
                "    println!(\"hello\");".into(),
                "}".into(),
            ],
        }]
    );
    Ok(())
}

#[test]
fn test_parse_update_file() -> Result<(), String> {
    let patch = "*** Begin Patch\n*** Update File: /tmp/test/existing.rs\n@@ fn main() {\n-    println!(\"old\");\n+    println!(\"new\");\n*** End Patch";
    assert_eq!(
        parse_patch(patch)?,
        vec![PatchOp::Update {
            path: "/tmp/test/existing.rs".into(),
            hunks: vec![Hunk {
                context_before: vec!["fn main() {".into()],
                removals: vec!["    println!(\"old\");".into()],
                additions: vec!["    println!(\"new\");".into()],
                context_after: Vec::new(),
            }],
        }]
    );
    Ok(())
}

#[test]
fn test_parse_delete_file() -> Result<(), String> {
    let patch = "*** Begin Patch\n*** Delete File: /tmp/test/to_delete.rs\n*** End Patch";
    assert_eq!(
        parse_patch(patch)?,
        vec![PatchOp::Delete {
            path: "/tmp/test/to_delete.rs".into(),
        }]
    );
    Ok(())
}

#[test]
fn test_parse_multi_file_patch() -> Result<(), String> {
    let patch = "*** Begin Patch\n*** Add File: /tmp/test/new.rs\n+// new file\n*** Update File: /tmp/test/update_me.rs\n@@ fn foo() {\n-    old();\n+    new();\n*** Delete File: /tmp/test/delete_me.rs\n*** End Patch";
    let operations = parse_patch(patch)?;
    assert_eq!(operations.len(), 3);
    assert!(matches!(&operations[0], PatchOp::Add { path, .. } if path == "/tmp/test/new.rs"));
    assert!(
        matches!(&operations[1], PatchOp::Update { path, .. } if path == "/tmp/test/update_me.rs")
    );
    assert!(matches!(&operations[2], PatchOp::Delete { path } if path == "/tmp/test/delete_me.rs"));
    Ok(())
}

#[test]
fn test_parse_malformed_no_begin() {
    let result = parse_patch("*** Add File: /foo\n+hello\n*** End Patch");
    assert!(result.is_err_and(|error| error.contains("Begin Patch")));
}

#[test]
fn test_parse_malformed_no_end() {
    let result = parse_patch("*** Begin Patch\n*** Add File: /foo\n+hello");
    assert!(result.is_err_and(|error| error.contains("End Patch")));
}

#[test]
fn test_parse_empty_patch() -> Result<(), String> {
    assert!(parse_patch("*** Begin Patch\n*** End Patch")?.is_empty());
    Ok(())
}
