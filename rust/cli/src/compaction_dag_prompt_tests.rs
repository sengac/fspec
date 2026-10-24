use super::build_generate_compaction_prompt;

#[test]
fn fresh_prompt_names_the_target_session() {
    let target = uuid::Uuid::new_v4();
    let prompt = build_generate_compaction_prompt(target, None);
    assert!(prompt.contains("Build a hierarchical summary DAG of your session"));
    assert!(prompt.contains(&format!("SessionSearch(session_id: \"{target}\", show, ")));
    assert!(prompt.contains(&format!("SessionSearch(session_id: \"{target}\", search, ")));
    assert!(prompt.contains(&format!("Session {target} has been preserved on disk")));
    assert!(!prompt.contains("Your conversation history has been preserved on disk"));
    assert!(prompt.contains("Output the complete DAG"));
    assert!(!prompt.contains("inject_summary"));
}

#[test]
fn incremental_prompt_embeds_existing_dag_and_turn_offset() {
    let target = uuid::Uuid::new_v4();
    let existing = "<system-reminder>\n<!-- type:compaction-dag -->\n<dag-node depth=\"D2\" turns=\"0-45\" label=\"Architecture\">ok</dag-node>\n</system-reminder>";
    let prompt = build_generate_compaction_prompt(target, Some((existing.to_string(), 45)));
    assert!(prompt.contains("do NOT rebuild from scratch"));
    assert!(prompt.contains(existing));
    assert!(prompt.contains("PRESERVE all existing D2"));
    assert!(prompt.contains("PROMOTE existing D0 (Detailed) nodes to D1"));
    assert!(prompt.contains("start_turn: 46"));
    assert!(!prompt.contains("inject_summary"));
}
