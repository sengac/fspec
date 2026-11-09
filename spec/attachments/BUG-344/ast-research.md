# BUG-344 AST research

## fspec astgrep --pattern 'pub(super) async fn run_openai_turn($$$A) -> $R { $$$B }' --lang rust --path rust/agent-loop/src/agent_loop/provider_openai.rs
rust/agent-loop/src/agent_loop/provider_openai.rs:5:1:pub(super) async fn run_openai_turn(


## fspec astgrep --pattern 'pub(super) async fn run_custom_turn($$$A) -> $R { $$$B }' --lang rust --path rust/agent-loop/src/agent_loop/provider_custom.rs
rust/agent-loop/src/agent_loop/provider_custom.rs:5:1:pub(super) async fn run_custom_turn(


## fspec astgrep --pattern '$P.create_rig_agent($$$A)' --lang rust --path rust/agent-loop/src/agent_loop
rust/agent-loop/src/agent_loop/provider_openai.rs:21:25:provider.create_rig_agent(
rust/agent-loop/src/agent_loop/provider_custom.rs:89:13:stub.create_rig_agent(session.id, role_preamble.as_deref(), thinking_config_value)

