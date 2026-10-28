//! Feature: spec/features/agentview-shift-right-create-session-dialog.feature
use super::*;

#[test]
fn shift_right_with_work_unit_binding_renders_generic_strings() {
    // @step Given an App in AgentView with one open session bound to WorkUnitContext with id "RPC-097"
    let mut app = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.agent_view_store_mut().set_work_unit_context(
        sid("s-1"),
        WorkUnitContext {
            id: "RPC-097".into(),
            title: "Test card".into(),
            status: "specifying".into(),
        },
    );
    app.navigator_mut().active_view = ViewMode::Agent;
    // @step When the user presses Shift+Right
    app.dispatch(Action::SessionNext);
    let painted = buffer_to_string(&render_app_buffer(&mut app));
    // @step Then the rendered dialog title is "Start New Agent?"
    assert!(
        painted.contains("Start New Agent?"),
        "missing generic title: {painted}"
    );
    // @step And the rendered description is "Begin a fresh AI conversation, not linked to any task."
    assert!(
        painted.contains("Begin a fresh AI conversation, not linked to any task."),
        "missing generic description: {painted}"
    );
    assert_eq!(
        app.agent_view_store()
            .work_unit_context_for(&sid("s-1"))
            .map(|context| context.id.as_str()),
        Some("RPC-097")
    );
}
