use codelet_cli::compaction_dag::build_generate_compaction_prompt;

#[test]
fn compaction_prompts_define_empty_and_strict_file_metadata() {
    let target = uuid::Uuid::new_v4();
    let durable = r#"<dag-node depth="D2" turns="0-0" label="decision">Keep bytes</dag-node>"#;
    for existing in [None, Some((durable.to_string(), 0))] {
        let prompt = build_generate_compaction_prompt(target, existing);
        assert!(prompt.contains("<dag-files></dag-files>"));
        assert!(prompt.contains("No prose, backticks, or explanations"));
        assert!(prompt.contains("- path/to/file.rs (Created)"));
        assert!(prompt.contains("exactly Created, Modified, or Deleted"));
    }
}
