# REFAC-056 AST research

- Helpers (HEAD lines 41-110): one OnceLock static and five fns, used only by this file's tests.
- Imports the helpers need: Duration, SessionManagerHandle, SessionId, SessionManager, UnifiedExecArgs,
  UnifiedExecResult, UnifiedExecTool, rig::tool::Tool, Uuid. The tests still use Arc, Duration, SessionManagerHandle,
  ExecStdinRequest, SessionStatus, SessionManager, InternalExecStdinRequest, UnifiedExecArgs, UnifiedExecResult,
  Tool (for `.call`), serial and Uuid.
- Noticed, filed separately: `wire_exec_stdin_request_round_trips_through_serde_json` maps to no scenario;
  `close_exec` discards the close call's Result.
