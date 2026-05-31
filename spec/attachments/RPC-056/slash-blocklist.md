# RPC-056 — `/blocklist` view + blocklist RPC surface

**Parent:** RPC-030 · **Phase:** 7.3 · **Estimate:** 5 pts · **Depends on:** RPC-055

## Goal

Port the TS `/blocklist` flow (see `handleBlocklistMode` referenced from `AgentView.tsx` line 2757). New child view `BlocklistView` in `codelet/fspec-tui/src/views/`. New RPC surface (`blocklist_list`, `blocklist_add`, `blocklist_remove`, `blocklist_update`) backed by `codelet-tools` (already NAPI-free).

## Backend trait additions

```rust
fn blocklist_list(&self) -> Vec<BlocklistEntry>;
fn blocklist_add(&self, entry: BlocklistEntryInput) -> Result<BlocklistEntry, String>;
fn blocklist_remove(&self, id: &str) -> Result<(), String>;
fn blocklist_update(&self, id: &str, update: BlocklistEntryInput) -> Result<BlocklistEntry, String>;
```

New wire types in `codelet/rpc-types/src/lib.rs`:

```rust
pub struct BlocklistEntry {
    pub id: String,
    pub pattern: String,      // e.g. "rm -rf *"
    pub category: String,     // "bash" | "file_path" | …
    pub description: Option<String>,
    pub enabled: bool,
}

pub struct BlocklistEntryInput {
    pub pattern: String,
    pub category: String,
    pub description: Option<String>,
    pub enabled: bool,
}
```

## Implementation

Delegate to `codelet_tools::blocklist::*` (audit the actual module name — likely `codelet_tools::facade::blocklist` or similar). All implementations are storage-CRUD against `~/.fspec/blocklist.json` (or wherever the TS frontend stores it).

## Frontend view

`codelet/fspec-tui/src/views/blocklist/mod.rs`:

```
┌── Blocklist ────────────────────────────────┐
│ [✓] bash      rm -rf *           "Dangerous │
│ [✓] file_path /etc/passwd        "Sensitive │
│ [ ] bash      sudo *             "Disabled  │
│                                             │
│ [a] add  [e] edit  [d] delete  [Space] tog  │
└─────────────────────────────────────────────┘
```

Key bindings:
- `a` → open `BlocklistEntryDialog` to add a new entry.
- `e` → edit the focused entry.
- `d` → delete (with confirm).
- `Space` → toggle enabled.
- `j/k` / arrows → navigate.

## Slash command wiring

```rust
SlashCommandAction::Blocklist => {
    self.emit_action(Action::OpenBlocklistView);
}

Action::OpenBlocklistView => {
    let backend = self.backend.clone();
    let sender = self.dispatch_sender.clone();
    tokio::spawn(async move {
        let entries = backend.blocklist_list().await.unwrap_or_default();
        let _ = sender.send(Action::BlocklistLoaded { entries });
    });
    self.navigator.go_to_blocklist();
}
```

## Acceptance criteria

1. New trait methods exist on all three layers.
2. `codelet/sessions` delegates to `codelet-tools::blocklist`.
3. `/blocklist` opens BlocklistView showing all entries.
4. Add / edit / delete / toggle all work and persist.
5. Disabled entries are skipped by the bash and file-path pre-tool hooks (already in `codelet_tools::pre_tool_hook` — verify this still works after the lift).
6. Integration test in `codelet/fspec-tui/tests/blocklist_view.rs`.

## Risks

- The blocklist `pre_tool_hook` may currently be registered via NAPI (`register_pre_tool_hook` at line 43 of original `session_manager.rs`). Confirm post-lift it's still wired in `codelet-sessions`.

## Out of scope

- Auto-import of common security patterns (a TS feature, scope follow-up).
