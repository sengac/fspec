use codelet_tools::unified_exec::ExecCommand;

#[cfg(unix)]
#[tokio::test]
async fn unicode_output_does_not_kill_the_tool_server() -> Result<(), Box<dyn std::error::Error>> {
    use codelet_tools::unified_exec::UnifiedExecTool;
    use rig::tool::server::ToolServer;
    use serde_json::{json, Value};
    use uuid::Uuid;

    let handle = ToolServer::new()
        .tool(UnifiedExecTool::new(Uuid::new_v4()))
        .run();

    for character in ["é", "─", "🦀"] {
        let prefix = "x".repeat(29_999);
        let payload = format!("{prefix}{character}tail");
        let args = json!({
            "action": "run",
            "command": ["printf", "%s", payload],
            "yield_time_ms": 250,
        });
        let response = handle.call_tool("unified_exec", &args.to_string()).await?;
        let result: Value = serde_json::from_str(&response)?;
        assert_eq!(result["exit_code"], 0);
        assert_eq!(
            result["output"],
            format!("{prefix}\n\n... [output truncated at 30000 characters]")
        );
        let definitions = handle.get_tool_defs(None).await?;
        assert_eq!(definitions.len(), 1);
    }
    Ok(())
}

#[test]
fn unicode_command_previews_preserve_character_boundaries() {
    for character in ["é", "─", "🦀"] {
        let prefix = "x".repeat(56);
        let command = format!("{prefix}{character}more text");
        let expected = format!("{prefix}...");
        assert_eq!(ExecCommand::Shell(command.clone()).display(), expected);
        assert_eq!(ExecCommand::Argv(vec![command]).display(), expected);
    }
}

#[test]
fn command_previews_preserve_existing_ascii_limits() {
    let command = "x".repeat(60);
    assert_eq!(ExecCommand::Shell(command.clone()).display(), command);
    assert_eq!(ExecCommand::Argv(vec![command.clone()]).display(), command);

    let command = "x".repeat(61);
    let expected = format!("{}...", "x".repeat(57));
    assert_eq!(ExecCommand::Shell(command.clone()).display(), expected);
    assert_eq!(ExecCommand::Argv(vec![command]).display(), expected);
}
