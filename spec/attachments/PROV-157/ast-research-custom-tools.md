# Codex custom tool integration research

AST search `RawStreamingToolCall::new($$$ARGS)` in the Codex stream event module
finds the ordinary complete-call constructor at stream_events.rs:31. This is the
normalization boundary for custom completed input and namespace qualification.
RawStreamingToolCall preserves both item ID and call ID. The ordinary rig loop
invokes its existing execution hook before the registered executor, so no second
executor or permission path is needed.

Upstream protocol models distinguish custom_tool_call.input from JSON arguments,
carry namespace separately, and require custom_tool_call_output for results.
The initial raw executor allowlist is apply_patch; its existing argument object
contains patch. Namespace aliases must target a registered function definition.
Request definition rewriting and result normalization belong exclusively to the
Codex adapter and its typed continuation state.
