# AST Research — RPC-410 HITL Wire Protocol Parity

## Queries run (AstGrep tool)

### 1. `fn get_hitl_request($$$ARGS) -> $RET { $$$BODY }` (rust, codelet/)

- `codelet/sessions/src/handle_impl.rs:826` — real impl; currently slices `questions[0]` and hardcodes `allow_text_input: true`. **Target of §3.2 pass-through rewrite.**
- `codelet/core/src/session_manager_handle.rs:580` — trait default (`None`); type-only, compiles with new shapes unchanged.
- `codelet/core/src/session_manager_handle.rs:1951` — `StubSessionManagerHandle` (stores/returns seeded value by type; no field access → compiles unchanged).

### 2. `pub struct HitlRequest { $$$FIELDS }` (rust, codelet/)

- `codelet/tools/src/request_user_input.rs:55` — internal tool type `{questions: Vec<HitlQuestion>}` — **already parity-correct, unchanged**.
- `codelet/rpc-types/src/lib.rs:1103` — wire type `{id, question, header, options, allow_text_input}` — **replaced by §3.1 TS-parity shape**.

## Grep survey of all `Hitl*` consumers (ripgrep)

Construction/field-access sites that must change:
- `codelet/rpc-types/src/lib.rs` (types + in-file round-trip tests :1619-1651)
- `codelet/rpc-types/tests/rpc036_widen_types.rs` (:175-190, :397-420, :562-590 construct old shapes)
- `codelet/sessions/src/handle_impl.rs` (:748-852 — both mapping fns)
- `codelet/sessions/tests/rpc408_hitl_response_answer_mapping.rs` (pins old heuristic — rewritten)
- `codelet/sessions/tests/handle_impl.rs:170` (constructs wire HitlResponse{id,value})
- `codelet/fspec-tui/src/components/hitl_dialog.rs` (renders single-question shape; submit builds {id,value})
- `codelet/fspec-tui/src/app/dispatch_pause_hitl.rs` (type pass-through only)
- `codelet/fspec-tui/tests/{pause_hitl_rpc053.rs, agent_input_paste_routing_rpc403.rs, rpc037_cross_transport_parity.rs, common/mod.rs}` (construct old wire shapes in fixtures)

Type-only forwarding (compiles with new shapes, no logic change):
- `codelet/rpc/src/lib.rs` (tarpc service defn :451,:458 + default :1717-1732)
- `codelet/fspec-tui/src/transport/{mod.rs, websocket.rs, embedded.rs}` (trait defaults + tarpc forwarding)
- `codelet/core/src/session_manager_handle.rs` (trait default + stub storage)

Unchanged (internal types, out of scope per dossier §3.5):
- `codelet/tools/src/**` (request_user_input.rs, facade/*), `codelet/agent-loop/src/agent_loop.rs`,
  `codelet/sessions/src/background_session.rs`, `codelet/napi/src/**` (uses its own `NapiHitlRequestState` /
  `HitlResponseInfo` mapped directly to tools-crate types — verify compile only),
  `codelet/providers/src/custom/tool_dispatch_extras.rs`.

## Conclusion

Compile-time propagation from the rpc-types change reaches exactly: sessions/handle_impl.rs,
sessions tests, rpc-types tests, fspec-tui hitl_dialog + tests fixtures. napi does not import
`codelet_rpc_types::Hitl*` at all (own NAPI types → tools-crate internal types), so §3.5 holds.
