# RPC-020 — Slash command palette + @file search popup

## TypeScript reference

### SlashCommandPalette
`src/tui/components/SlashCommandPalette.tsx` (172 lines)

Triggered when the user types `/` as the first character of an empty
input line. Renders a floating popup above the input box with the
filtered list of available slash commands.

Commands (from `src/tui/hooks/useSlashCommandInput.ts` — search for the
command registry):
- `/help` — show help
- `/clear` — clear scrollback
- `/resume` — resume an existing session (opens session picker)
- `/search` — search command history
- `/model` — open model selector
- `/thinking` — open thinking level dialog
- `/role` — set/clear session role
- `/quit` — quit the TUI
- `/isolation` — toggle isolation (worktree)
- `/blocklist` — open blocklist editor
- `/compact` — manually compact the session
- `/debug` — toggle debug capture
- `/providers` — open provider settings

The palette filters by prefix match as the user types. Arrow keys
navigate; Enter executes. Esc dismisses.

### FileSearchPopup
`src/tui/components/FileSearchPopup.tsx` (170 lines)

Triggered when the user types `@` followed by characters. Renders a
floating popup with file path suggestions matching the typed prefix.
Backed by `useFileSearchInput.ts` which calls `glob()` or a similar
file-search NAPI helper.

Selecting a result inserts the file path into the input at the `@`
position (replacing the `@<typed>` substring with the chosen path).

## Current Rust state

Neither feature exists. The single-line input from RPC-012 only accepts
plain text submission.

## Target Rust behavior

### Slash command registry

New file: `codelet/fspec-tui/src/views/agent/slash_commands.rs`.

```rust
#[derive(Debug, Clone)]
pub struct SlashCommand {
    pub name: &'static str,
    pub description: &'static str,
    pub action: SlashCommandAction,
}

#[derive(Debug, Clone)]
pub enum SlashCommandAction {
    Help,
    Clear,
    Resume,
    Search,
    Model,
    Thinking,
    Role,
    Quit,
    Isolation,
    Blocklist,
    Compact,
    Debug,
    Providers,
}

pub const SLASH_COMMANDS: &[SlashCommand] = &[ /* mirror TS registry */ ];

pub fn filter_commands(prefix: &str) -> Vec<&'static SlashCommand> { ... }
```

### SlashCommandPopup widget

`codelet/fspec-tui/src/views/agent/slash_command_popup.rs`:
```rust
pub struct SlashCommandPopup {
    pub filter: String,           // text after the leading `/`
    pub selected_index: usize,
    pub matches: Vec<&'static SlashCommand>,
}

impl SlashCommandPopup {
    pub fn on_filter_change(&mut self, prefix: &str) { ... }
    pub fn select_next(&mut self) { ... }
    pub fn select_prev(&mut self) { ... }
    pub fn selected(&self) -> Option<&SlashCommand> { ... }
    pub fn render(&self, area: Rect, buf: &mut Buffer) { ... }
}
```

The popup renders above the input box as a tui-popup overlay (already
in use for HelpDialog / DisconnectDialog via `Compositor::push`).

### File search popup

`codelet/fspec-tui/src/views/agent/file_search_popup.rs`:
```rust
pub struct FileSearchPopup {
    pub filter: String,             // text after the `@`
    pub anchor_offset: usize,       // byte offset of `@` in the input
    pub matches: Vec<String>,       // file paths
    pub selected_index: usize,
}
```

### New FspecBackend method

```rust
#[async_trait]
pub trait FspecBackend: Send + Sync {
    // existing methods...

    /// RPC-020: search the workspace for files whose path matches the
    /// prefix. Returns at most `limit` matches sorted by relevance.
    /// Mirrors the TS `useFileSearchInput` glob path.
    async fn search_files(&self, prefix: String, limit: u32) -> Result<Vec<String>>;
}
```

### New RPC method

```rust
// codelet/rpc/src/lib.rs
async fn search_files(prefix: String, limit: u32) -> Vec<String>;
```

### Service impl

Delegates to `codelet_core::file_search::search(cwd, prefix, limit)` (new
helper, mirrors the TS `glob` call but in Rust using the `ignore` or
`tinyglobby`-equivalent Rust crate).

### NAPI exports

Additive:
- `napi::search_files(cwd, prefix, limit) -> Vec<String>` —
  used by TS's `useFileSearchInput` so both TUIs converge on the same
  search backend.

### Input wiring

Extend the `MultiLineInput` from RPC-019 to detect:
- A leading `/` on an empty line → emit `Action::OpenSlashPalette`.
- A `@` character → emit `Action::OpenFileSearch(anchor_offset)`.

`App::dispatch` pushes the appropriate popup widget into the Compositor
at `Priority::Foreground`. The popup intercepts keyboard events
(up/down/enter/esc) and re-emits actions:
- `SlashCommandSelected(action)` — App routes to the matching handler.
- `FileSearchSelected(path)` — App replaces the input's `@<filter>`
  substring with the chosen path.

## Slash command action handlers — scope guard

This card delivers the PALETTE + FILE-SEARCH UX, NOT all the command
handlers. The handler implementations land in their own cards:
- `/model` → RPC-022 (modal dialogs)
- `/thinking` → RPC-022
- `/role` → RPC-022
- `/resume` / `/search` → RPC-021 (multi-session + history)
- `/help` → trivial — implement in this card.
- `/clear` → trivial — implement in this card.
- `/quit` → trivial — implement in this card.
- `/isolation`, `/blocklist`, `/compact`, `/debug`, `/providers` →
  emit a `not yet implemented` notice for now; full implementations are
  future RPC-002 children.

## RPC/NAPI boundary contract

```
TS AgentView → useFileSearchInput → glob() / NAPI file search
                                  → already wired

Rust TUI → FspecBackend::search_files
       → FspecService::search_files [tarpc]
       → codelet_core::file_search::search(cwd, prefix, limit) [shared impl]
```

## Existing TypeScript behavior preserved

- `src/tui/components/SlashCommandPalette.tsx` — UNCHANGED.
- `src/tui/components/FileSearchPopup.tsx` — UNCHANGED.
- `src/tui/hooks/useSlashCommandInput.ts` — UNCHANGED.
- `src/tui/hooks/useFileSearchInput.ts` — UNCHANGED.

## Acceptance criteria sketch

- Typing `/` on an empty input line opens the slash command popup.
- The popup filters live as the user types more characters.
- Arrow up/down navigates the popup; Enter executes; Esc dismisses.
- Implemented handlers in THIS card: `/help`, `/clear`, `/quit`.
- Unimplemented handlers display `Not yet implemented (see RPC-NNN)`
  in the scrollback as a `notice` chunk.
- Typing `@` in the input opens the file search popup with results
  matching the typed prefix.
- Selecting a file replaces `@<filter>` in the input with the chosen
  path.
- A new RPC method `FspecService::search_files(prefix, limit)` exists
  and is tested against both transports.
- The TS AgentView still works unchanged.
