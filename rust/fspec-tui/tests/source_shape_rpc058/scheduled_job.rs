/// Scenario: ScheduledJob wire type is exported from codelet-rpc-types
#[test]
fn rpc_types_exports_scheduled_job_wire_type() {
    // @step Given the file rust/rpc-types/src/lib.rs is compiled
    let path = workspace_root().join("rust/rpc-types/src/lib.rs");
    let source = fs::read_to_string(&path).expect("read rpc-types/src/lib.rs");
    let normalised = normalise(&source);

    // @step Then it declares a public struct named "ScheduledJob"
    assert!(
        source.contains("pub struct ScheduledJob"),
        "rpc-types/src/lib.rs should declare pub struct ScheduledJob"
    );

    // @step And ScheduledJob has fields named name, cron, timezone, job_type, status
    // @step And ScheduledJob has fields named created_at, last_run_at, last_run_status
    // @step And ScheduledJob has fields named role, prompt, command, overlap_policy
    for field in [
        "pub name:",
        "pub cron:",
        "pub timezone:",
        "pub job_type:",
        "pub status:",
        "pub created_at:",
        "pub last_run_at:",
        "pub last_run_status:",
        "pub role:",
        "pub prompt:",
        "pub command:",
        "pub overlap_policy:",
    ] {
        assert!(
            normalised.contains(field),
            "ScheduledJob should declare field {field:?}"
        );
    }
}
