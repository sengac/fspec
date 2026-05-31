# RPC-056 — AST Research: /blocklist view + blocklist RPC surface

This document captures the AST-level findings that drive the
implementation of RPC-056. The goal is to land a `blocklist_list` RPC
at every layer of the dual-transport stack and a new Navigator-owned
`BlocklistView` that pairs with it — full TS-Ink parity with
`src/tui/components/BlocklistListView.tsx`.

## 1. Source of truth on the TS side

`src/tui/components/BlocklistListView.tsx` (275 lines, BLOCK-004) renders
a full-screen overlay with:

* a left-pane list of rules (50% width) with `●`/`○` glyphs reflecting
  the session-disabled set,
* a right-pane details panel (50% width) with id, action (colourised),
  source, pattern (wrap), reason (wrap), guidance (wrap, green), and
  the Session Status field,
* an empty-state placeholder when no rules are configured (lines 150–
  160) listing the two config paths,
* keyboard nav: `j/k` + arrows for selection, `Enter`/`Space` to
  toggle, `Esc` to close.

`src/tui/components/AgentView.tsx` line 2757 reaches `handleBlocklistMode()`
which loads the rules via `blocklistLoad(process.cwd())` (a NAPI call)
and pushes the overlay. The disabled-rule `Set<string>` is owned by
the AgentView component, not persisted to disk.

Conclusion: the TS frontend is read-only + in-memory session toggle.
This is the parity target for RPC-056.

## 2. Existing Rust blocklist module (codelet-tools)

`codelet/tools/src/blocklist/` exposes:

```rust
pub use config::{BlocklistAction, BlocklistConfig, BlocklistRule};
pub use middleware::{
    allow_for_session, check_bash_command, check_command_raw, check_file_path,
    clear_session_allowances, init_blocklist, is_session_allowed,
    load_blocklist_config, project_config_path, reload_blocklist,
    system_config_path, BlockedError,
};
```

`BlocklistRule` (config.rs, lines 20–33):

```rust
pub struct BlocklistRule {
    pub id: String,
    pub pattern: String,
    pub action: BlocklistAction,       // Block | Allow | Prompt
    pub reason: String,
    pub guidance: Option<String>,
}
```

`BlocklistAction` (config.rs, lines 9–17) is serialised lowercase
(`"block" | "allow" | "prompt"`).

`system_config_path()` returns `~/.fspec/blocklist.json`
`project_config_path(root)` returns `<root>/.fspec/blocklist.json`
`load_blocklist_config(Some(root))` merges system + project configs and
LOSES the per-rule provenance — so RPC-056's `blocklist_list` MUST
load both configs separately and tag each rule with its `source`.

The middleware already wires `pre_tool_hook` for `check_bash_command`
and `check_file_path`. RPC-056 does NOT modify this wiring (the
session-disabled set is a UI-only affordance in this slice, matching
TS parity).

## 3. Pattern-matching across existing RPC dispatch slices

The most analogous existing slices are RPC-054 (`/provider`) and
RPC-055 (`/debug`):

| Layer | RPC-054 (`/provider`) | RPC-055 (`/debug`) |
|-------|-----------------------|---------------------|
| Wire type | `ProviderCredentialInfo` (rpc-types/src/lib.rs:393) | — |
| Handle trait method | `list_provider_credentials` (core/session_manager_handle.rs:515) | `set_debug_directory` (core/session_manager_handle.rs:407) |
| Stub counter | `list_provider_credentials_calls()` (core/session_manager_handle.rs:745) | `set_debug_directory_calls()` (core/session_manager_handle.rs:758) |
| Tarpc method | `list_provider_credentials` (rpc/src/lib.rs:354) | `set_debug_directory` (rpc/src/lib.rs:299) |
| Backend trait | `list_provider_credentials` (transport/mod.rs:486) | `set_debug_directory` (transport/mod.rs:409) |
| Sessions impl | `list_provider_credentials` (sessions/handle_impl.rs:716) | `set_debug_directory` (sessions/handle_impl.rs:402) |
| Dispatch helper | `app/dispatch_rpc054.rs` (276 lines) | `app/dispatch_rpc055.rs` (68 lines) |
| Slash arm in dispatch_rpc020 | `OpenProviderSettingsView` (dispatch_rpc020.rs:110-115) | `handle_slash_debug()` call (dispatch_rpc020.rs:116-119) |
| Cross-transport test | `tests/rpc054_cross_transport_parity.rs` | `tests/rpc055_cross_transport_parity.rs` |
| Source-shape test | `tests/source_shape_rpc054.rs` | `tests/source_shape_rpc055.rs` |
| View (when applicable) | `views/provider_settings/mod.rs` + Navigator ViewMode::ProviderSettings | — (debug has no view) |

RPC-056 follows RPC-054's pattern (it has a child view) rather than
RPC-055's (which only emits notices). The view file goes at
`codelet/fspec-tui/src/views/blocklist/mod.rs` and the Navigator gains
a `ViewMode::Blocklist` variant alongside `ViewMode::ProviderSettings`.

## 4. SessionManager workspace cwd shape

`codelet/sessions/src/session_manager.rs::SessionManager::new()`
(line 182) does NOT store a workspace path. The existing
`handle_impl.rs::create_session` (line 68) resolves the project root
via `std::env::current_dir()`. The new `blocklist_list` impl follows
the same pattern — pass `Some(&std::env::current_dir()?)` (or
`None` as a fall-through) to `system_config_path()` and
`project_config_path()` and tag rules accordingly.

## 5. AgentViewStore extension point

The existing AgentViewStore (codelet/fspec-tui/src/store/agent_view.rs)
already carries per-session maps for model_info, thinking_level,
work_unit_context, debug_enabled, input_draft, etc. RPC-056 adds
`blocklist_disabled_by_session: HashMap<SessionId, HashSet<String>>`.
The view reads/writes through accessor methods on the store so the
state lifts above the view's own lifetime — matching the TS
`useState<Set<string>>` that lives on AgentView, not on
BlocklistListView.

## 6. New surface area for RPC-056

| File | New code |
|------|----------|
| `codelet/rpc-types/src/lib.rs` | `pub struct BlocklistRuleInfo` |
| `codelet/core/src/session_manager_handle.rs` | `fn blocklist_list(&self) -> Vec<BlocklistRuleInfo>` (trait default + StubSessionManagerHandle override + counter) |
| `codelet/rpc/src/lib.rs` | `async fn blocklist_list() -> Vec<BlocklistRuleInfo>` (tarpc service + impl) |
| `codelet/fspec-tui/src/transport/mod.rs` | `async fn blocklist_list() -> Result<Vec<BlocklistRuleInfo>>` (FspecBackend default) |
| `codelet/fspec-tui/src/transport/embedded.rs` | `blocklist_list` override forwarding to tarpc client |
| `codelet/fspec-tui/src/transport/websocket.rs` | `blocklist_list` override forwarding to tarpc client |
| `codelet/sessions/src/handle_impl.rs` | `blocklist_list` impl loading system + project configs separately |
| `codelet/fspec-tui/src/views/blocklist/mod.rs` | NEW — `BlocklistView` + `BlocklistEvent` + `derive_category` |
| `codelet/fspec-tui/src/views/mod.rs` | re-export `blocklist::*` |
| `codelet/fspec-tui/src/views/navigator.rs` | `ViewMode::Blocklist` + `Navigator.blocklist` field + routing |
| `codelet/fspec-tui/src/store/agent_view.rs` | `blocklist_disabled_by_session` map + accessors |
| `codelet/fspec-tui/src/components/mod.rs` | `Action::OpenBlocklistView`, `Action::CloseBlocklistView`, `Action::BlocklistRulesLoaded`, `Action::ToggleBlocklistRule` |
| `codelet/fspec-tui/src/app/dispatch_rpc020.rs` | replace the `Blocklist` arm's notice fallback with `Action::OpenBlocklistView` dispatch |
| `codelet/fspec-tui/src/app/dispatch_rpc056.rs` | NEW — `try_dispatch_rpc056` + `handle_open_blocklist_view` + `handle_close_blocklist_view` + `handle_blocklist_rules_loaded` + `handle_toggle_blocklist_rule` |
| `codelet/fspec-tui/tests/source_shape_rpc056.rs` | NEW source-shape regression |
| `codelet/fspec-tui/tests/rpc056_cross_transport_parity.rs` | NEW cross-transport parity test |
| `codelet/fspec-tui/tests/blocklist_view_rpc056.rs` | NEW view rendering + key handling + dispatch tests |

## 7. Out of scope (deferred)

* CRUD (add / edit / delete / update) on the blocklist config JSON
  files — the attachment's `blocklist_add` / `blocklist_remove` /
  `blocklist_update` are deferred. The user explicitly chose TS-parity
  scope.
* Wiring the session-disabled set into the existing
  `pre_tool_hook` (or any other enforcement mechanism). The TS
  frontend does NOT do this either — the disabled set is purely
  cosmetic. A follow-up card can add real enforcement when the broader
  rule-management UX lands.
* Auto-import of common security patterns.
