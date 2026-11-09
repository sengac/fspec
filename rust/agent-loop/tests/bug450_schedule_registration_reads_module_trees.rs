//! BUG-450 — schedule tool registration checks read provider module trees.
//!
//! Feature: spec/features/schedule-registration-checks-read-module-trees.feature

#[path = "schedule_tool_test/provider_source.rs"]
mod provider_source;

#[test]
fn the_openai_providers_module_tree_registers_the_schedule_tool() -> Result<(), String> {
    // @step Given the OpenAI provider source rust/providers/src/openai.rs and its submodules
    // @step When the module tree is read
    let source = provider_source::read_provider_source("openai.rs")?;
    // @step Then it registers ScheduleTool::new(session_id)
    assert!(
        source.contains("ScheduleTool::new(session_id)"),
        "the OpenAI module tree must register ScheduleTool::new(session_id)"
    );
    Ok(())
}
