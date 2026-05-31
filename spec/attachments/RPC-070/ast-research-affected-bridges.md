# RPC-070 — AST research: every sync→async bridge in handle_impl.rs

Performed via the AstGrep tool on `codelet/sessions/src/handle_impl.rs`.

## Direct `Handle::block_on` (4 matches)

`pattern='$RT.block_on($EXPR)' language=rust`

| Line:Col | Receiver | Method body |
|---------:|----------|-------------|
| 78:25   | `tokio::runtime::Handle::current()` | `create_session` |
| 620:20  | `tokio::runtime::Handle::current()` | `create_isolated_session` |
| 877:29  | `runtime` (built via `Handle::try_current()`) | `test_provider_connection` |
| 1293:5  | `tokio::runtime::Handle::current()` | `loop_block_on` helper |

All four are the panic-prone idiom when invoked from inside an executing tokio future on a multi-thread runtime (the live tarpc dispatcher).

## Helper-mediated bridges (3 matches via `loop_block_on`)

`pattern='loop_block_on($EXPR)' language=rust`

| Line:Col | Caller |
|---------:|--------|
| 1251:9   | `loop_add` |
| 1262:23  | `loop_cancel` |
| 1275:23  | `loop_list` |

All three forward through `loop_block_on` (line 1293) and therefore inherit the panic.

## `block_in_place` usage today

`pattern='tokio::task::block_in_place($EXPR)' language=rust` — **No matches** in this file. The canonical safe pattern from `codelet/tools/src/schedule/handler.rs:21` is not yet used here.

## Conclusion

Six call sites + one shared helper need the `tokio::task::block_in_place(|| Handle::current().block_on(...))` wrapper. `test_provider_connection` additionally needs the redundant `Handle::try_current()` removed.

Pre-existing safe usages of the same pattern that we mirror:

- `codelet/sessions/src/session_manager.rs:573` — pre-tool-use hook bridge.
- `codelet/sessions/src/session_manager.rs:819` — post-tool-use hook bridge.
- `codelet/napi/src/agent_loop.rs:982,1044` — V8-callback bridges.
- `codelet/napi/src/agent_manager_handler.rs:154,178` — agent manager bridges.
