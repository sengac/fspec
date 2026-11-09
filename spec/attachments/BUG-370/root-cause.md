# BUG-370 root cause

Seen right after TOOL-027 (91724ac83): `refac019_contracts` (`ordered_public_and_implementation_contracts_retain_their_baseline`) and `board024_additive_api_contract` (`canonical_detail_api_appends_exactly_one_method_without_changing_frozen_prefixes`) both fail. Three changes caused it. Two were already broken before TOOL-027, and TOOL-027 added the third:

1. **PROV-166 (e799d469b), already failing.** It changed the default body of `get_session_model` in `FspecBackend` (`transport/backend/session_history.rs`) and in the rpc implementation (`service_implementation/session_control.rs`): the SessionModel limits went from `0` to `None`. It did not re-baseline `tests/snapshots/refac019_contracts__refac019_backend.snap` or `..._rpc_implementation.snap`. Board024 reads the same snapshots as its frozen prefix, so it reports `frozen ordered member changed: expected get_session_model` for backend and rpc_implementation.
2. **MCP-003 (c0bd721e), already failing.** It appended `configure_session_mcp_servers` to the RPC declaration, implementation, `FspecBackend` and both transports, but did not add it to `APPROVED_APPENDED_TAILS` in `tests/refac019/appended.rs`. `frozen_prefix` strips only a trailing run of approved names, so the new member stays and the snapshot gains lines.
3. **TOOL-027 (91724ac83).** It appended `control_exec_stdin` on the same surfaces, with the same omission.

TOOL-027 did not run these two contract binaries before it committed. That is how it went in red.

## Fix
- Add `configure_session_mcp_servers` and `control_exec_stdin` to `APPROVED_APPENDED_TAILS` and name both cards in its doc. That list is the designed approval mechanism for appended members. Nothing else about the comparison changes.
- Re-baseline only the `get_session_model` digest in the backend and rpc_implementation snapshots to the PROV-166 body, and record that approval in the REFAC-019 feature description. Every other digest and the header must stay byte-identical. The snapshots are regenerated into `.snap.new` with `INSTA_FORCE_PASS=1` and diffed, and only the `get_session_model` line is copied over. If any other line differs, the fix stops there.
- The assertions themselves are unchanged.
