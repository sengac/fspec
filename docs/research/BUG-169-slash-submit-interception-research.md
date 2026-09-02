# Research: Slash-command autocomplete not intercepted on typed submit

**Work unit:** BUG-169
**Date:** 2026-09-02
**Crate:** `codelet-fspec-tui` (`rust/fspec-tui/`)
**Status:** Research complete — fix scoped, not yet implemented

---

## 1. Symptom

Reported by user (verbatim):

> if i type `/provide` the slash commands come up and if i press enter it will show
> the provider view, but if i type `/provide` then press the tab key to make it
> select the whole word, then press enter, it will send the text `/provider` to
> the llm rather than intercepting it as a command

Two input paths to the same command behave differently:

| Input sequence | Observed behaviour | Expected behaviour |
|---|---|---|
| `/provide` → **Enter** (popup open) | ✅ ProviderSettingsView opens | ✅ |
| `/provide` → **Tab** (popup closes, buffer now `/provider`) → **Enter** | ❌ `/provider` sent to the LLM | ✅ ProviderSettingsView opens |
| `/provider` typed fully by hand → **Enter** (popup auto-closed by Tab, or never had a popup) | ❌ sent to the LLM | ✅ ProviderSettingsView opens |

The same defect class affects **every** slash command whose intercept logic does
not live in `parse_slash_command` — see §4 for the full registry audit.

---

## 2. Event flow trace (exactly what happens, step by step)

All paths below are in `rust/fspec-tui/`.

### 2.1 The working path — Enter while the popup is open

```
User types /, p, r, o, v, i, d, e
  └─ AgentView::handle_event (views/agent/dispatch.rs:50)
       ├─ handle_popup_key(key) → popup.handle_key('p') → PopupOutcome::Ignored
       └─ input.handle_event_gated(...) → inserts 'p' → Continued
       └─ sync_popups() → classify_buffer("/provide") → PopupTrigger::OpenSlash("provide")
            (views/agent/popups.rs:39) — popup filter updated, popup STAYS OPEN
User presses Enter
  └─ handle_popup_key(Enter) (views/agent/dispatch_popups.rs:21)
       └─ SlashCommandPopup::handle_key (views/agent/slash_command_popup.rs:259)
            └─ selected() == Some(provider) → PopupOutcome::Selected(Provider)
  └─ dispatch_popups.rs:24 → emit(Action::SlashCommandSelected(Provider)); input.reset()
  └─ App::dispatch (app/dispatch.rs:224) → handle_slash_command(Provider)
       (app/dispatch_slash_commands.rs:104) → Action::OpenProviderSettingsView ✅
```

### 2.2 The broken path — Tab, then Enter

```
User types /provide   (popup open, "provider" is the highlighted match)
User presses Tab
  └─ handle_popup_key(Tab)
       └─ SlashCommandPopup::handle_key → PopupOutcome::Filled("/provider")
            (slash_command_popup.rs:298)
  └─ dispatch_popups.rs:30-34:
       self.slash_popup = None;            ← POPUP PERMANENTLY CLOSED
       self.input.set_value("/provider");  ← buffer now holds the full word
User presses Enter
  └─ handle_popup_key → slash_popup is None → falls through (None)
  └─ Tab toggle? No. Esc? No. …
  └─ input.handle_event_gated(Enter) → InputEventOutcome::Submitted("/provider")
       (multiline_input_enter.rs:61-67 — plain Enter always submits)
  └─ sync_popups() → classify_buffer("/provider") → OpenSlash("provider")
       but the submitted value has already been emitted — the popup
       re-opening AFTER submission is cosmetic only (a stale ghost)
  └─ emit(Action::InputSubmitted("/provider"))
  └─ App::dispatch (app/dispatch.rs:20) → handle_input_submitted("/provider")
       (app/dispatch_slash_commands.rs:198)
       └─ parse_slash_command("/provider") (app/slash_parser.rs:94)
            └─ returns SlashCommandParse::NotASlashCommand  ← THE GAP
       └─ NotASlashCommand arm (dispatch_slash_commands.rs:287) → {}
       └─ falls through to tokio::spawn(backend.send_input(session, "/provider"))
            → the literal string "/provider" is sent to the LLM ❌
```

### 2.3 Why the popup doesn't save the Enter

Two independent reasons:

1. **Tab destroys the popup.** `PopupOutcome::Filled` sets
   `self.slash_popup = None` (`dispatch_popups.rs:31`). There is no mechanism
   to "re-arm" the popup after a Tab fill. `sync_popups()` re-opens it on the
   *next* edit keystroke — but Enter is not an edit keystroke, it is handled
   by the textarea first and the popup's `handle_key` is only consulted if the
   popup is `Some` at the top of `handle_event`. After Tab it is `None`.

2. **Even if the popup could be re-opened on the same Enter event, it runs
   AFTER the textarea has already produced `Submitted`.** The dispatch order
   in `AgentView::handle_event` (`views/agent/dispatch.rs:209-217`) is:
   `input.handle_event_gated(event)` → `sync_popups()` → `emit(InputSubmitted)`.
   The popup can never retroactively swallow an already-emitted submission.
   (After the submit the buffer is cleared — `multiline_input_enter.rs:66`
   calls `input.reset()` before returning `Submitted` — so `sync_popups()`
   sees an empty buffer and the popup simply stays closed.)

---

## 3. Root cause

The TUI has **two dispatch paths** for slash commands, and the second one only
knows about a subset of the registry.

| Path | Triggered by | Routing |
|---|---|---|
| **A. Palette selection** | Enter *while the popup is open* | `SlashCommandPopup::handle_key` → `Action::SlashCommandSelected(action)` → `App::handle_slash_command` (`app/dispatch_slash_commands.rs:27`) — an **exhaustive** match over all 21 `SlashCommandAction` variants |
| **B. Typed submit** | Enter *with the popup closed* (after Tab-fill, Esc-dismiss, or after typing a space/arg) | `MultiLineInput` → `Action::InputSubmitted(text)` → `App::handle_input_submitted` (`app/dispatch_slash_commands.rs:198`) → `parse_slash_command(text)` (`app/slash_parser.rs:94`) — a **partial** parser covering only 11 of the 21 commands |

`parse_slash_command` recognizes, in order:

| Family | Recognized |
|---|---|
| bare `/model` | ✅ `OpenModelDialog` |
| bare `/thinking`, `/thinking <level>` | ✅ `OpenThinkingDialog` / `SetThinkingLevel` / `InvalidThinkingLevel` |
| `/role`, `/role clear`, `/role <text>` | ✅ `OpenRoleDialog` / `ClearRole` / `SetRole` |
| `/schedule …` | ✅ `ScheduleSubcommand` |
| `/loop …` | ✅ `LoopSubcommand` |
| `/continue …` | ✅ `ContinueSubcommand` |
| `/goal …` | ✅ `GoalSubcommand` |
| `/update …` | ✅ `UpdateSubcommand` |
| `/mux …` | ✅ `MuxCommand` |
| everything else | ❌ `NotASlashCommand` → `backend.send_input(text)` |

Commands **missing from path B** (sent to the LLM when typed + submitted):

- `/help`, `/clear`, `/quit` — the three commands that were wired first in
  RPC-020 (popup-only; the submit parser postdates them, RPC-022, and never
  added them back)
- `/provider` — RPC-054 wired `SlashCommandSelected(Provider)` →
  `OpenProviderSettingsView` but never extended `parse_slash_command`
- `/resume` — RPC-026 (popup → resume mode view)
- `/search` — RPC-026 (popup → search mode view)
- `/debug` — RPC-055 (popup → toggle debug capture)
- `/compact` — RPC-047 (popup → `backend.compact_session`)
- `/isolation` — RPC-060 (popup → create-session dialog, preselect Isolated)
- `/blocklist` — RPC-056 (popup → `OpenBlocklistView`)
- `/detach` — RPC-050 (popup → `handle_slash_detach`)
- `/merge-worktree` — RPC-057 (popup → `handle_slash_merge_worktree`)

**In other words, 11 of the 21 registered slash commands are uninterceptable
on the typed-submit path.** `/provider` is the reported instance; `/help`,
`/clear`, `/compact`, `/debug`, etc. all exhibit the identical defect.

---

## 4. Full registry audit (21 commands, both paths)

Source of truth: `views/agent/slash_commands.rs` (`SLASH_COMMANDS`, 21 entries,
line 95). Path A = `handle_slash_command` arms in `app/dispatch_slash_commands.rs:27-164`.
Path B = `parse_slash_command` arms in `app/slash_parser.rs:94-168`.

| # | Command | Action variant | Path A (popup Enter) | Path B (typed submit) |
|---|---|---|---|---|
| 1 | `/help` | `Help` | ✅ HelpDialog | ❌ **sent to LLM** |
| 2 | `/clear` | `Clear` | ✅ clears scrollback+input | ❌ **sent to LLM** |
| 3 | `/quit` | `Quit` | ✅ `should_quit = true` | ❌ **sent to LLM** |
| 4 | `/model` | `Model` | ✅ ModelSelector | ✅ `OpenModelDialog` (RPC-022) |
| 5 | `/thinking` | `Thinking` | ✅ ThinkingDialog | ✅ bare + inline level (RPC-022/048) |
| 6 | `/role` | `Role` | ✅ RoleDialog | ✅ bare/clear/text (RPC-063) |
| 7 | `/resume` | `Resume` | ✅ resume mode view | ❌ **sent to LLM** |
| 8 | `/search` | `Search` | ✅ search mode view | ❌ **sent to LLM** |
| 9 | `/provider` | `Provider` | ✅ ProviderSettingsView | ❌ **sent to LLM** *(reported bug)* |
| 10 | `/debug` | `Debug` | ✅ toggle debug capture | ❌ **sent to LLM** |
| 11 | `/compact` | `Compact` | ✅ `compact_session` | ❌ **sent to LLM** |
| 12 | `/isolation` | `Isolation` | ✅ create-session dialog | ❌ **sent to LLM** |
| 13 | `/blocklist` | `Blocklist` | ✅ BlocklistView | ❌ **sent to LLM** |
| 14 | `/detach` | `Detach` | ✅ detach handler | ❌ **sent to LLM** |
| 15 | `/merge-worktree` | `MergeWorktree` | ✅ merge handler | ❌ **sent to LLM** |
| 16 | `/schedule` | `Schedule` | ✅ help notice | ✅ bare → `Help` (RPC-058) |
| 17 | `/loop` | `Loop` | ✅ help notice | ✅ bare → `Help` (RPC-059) |
| 18 | `/continue` | `Continue` | ✅ toggle | ✅ bare → `Toggle` (CONT-002) |
| 19 | `/goal` | `Goal` | ✅ show | ✅ bare → `Show` (CONT-003) |
| 20 | `/update` | `Update` | ✅ check+install | ✅ bare → `CheckAndUpdate` (UPD-002) |
| 21 | `/mux` | `Mux` | ✅ MuxConfigDialog | ✅ bare + subcommands (MUX-001) |

Observations:

- Every command with a **subcommand grammar** (schedule, loop, continue,
  goal, update, mux, thinking, role) got path-B coverage because users *must*
  type arguments past the space, at which point `classify_buffer` closes the
  popup (`popups.rs:44`) and only path B can handle it.
- Every **bare-only** command was wired exclusively through the popup. Path B
  was assumed to be redundant for them — and is, until Tab/Esc breaks the
  popup invariant.
- The spec for the original popup card even *documents* the trap:
  `spec/features/rpc020-slash-and-file-popups.feature` example 11:
  *"user types '/' then presses Down, then Tab → input fills with '/clear'
  (no execute), popup closes; user can edit further or press Enter to send
  '/clear' as ordinary text."* — i.e. the defect was spec'd as intended in
  2025 (before most commands got handlers), and never re-visited.

---

## 5. Contributing factors

### 5.1 Tab closes the popup by design (and by spec)

`spec/features/rpc020-slash-and-file-popups.feature`, rule 2:
> *"Tab (fill into input WITHOUT execute)"*

`dispatch_popups.rs:30-34`:
```rust
PopupOutcome::Filled(text) => {
    self.slash_popup = None;      // closed — no re-arming mechanism
    self.input.set_value(&text);
    return Some(EventResult::consumed());
}
```

This is fine on its own — the *intent* of Tab is "complete the word, keep
typing arguments (e.g. `/role reviewer`), then submit." The bug is that the
submit-time interceptor (`parse_slash_command`) does not know about most
commands, so a completed bare command falls out of both paths.

### 5.2 Esc has the same effect

`PopupOutcome::Dismiss` → `self.slash_popup = None` with the buffer unchanged
(`"/provide"` still in the input). A subsequent Enter then also submits
`/provider` to the LLM. Same class, second trigger.

### 5.3 `handle_input_submitted` treats "no match" as "it's prose"

`app/dispatch_slash_commands.rs:287-288`:
```rust
SlashCommandParse::NotASlashCommand => {}
// …falls through…
tokio::spawn(async move {
    let _ = backend.send_input(session_for_send, text_for_send).await;
});
```

There is no secondary check such as "does the first word match a registered
slash command by name?" — so the registry and the parser have diverged.

### 5.4 The popup re-opens cosmetically after the submit (red herring)

`sync_popups()` runs after every input event, including the Enter that
submitted. It sees an already-cleared buffer (`input.reset()` at
`multiline_input_enter.rs:66`) and closes everything. If a test harness sets
the value manually without going through `reset`, a ghost popup can re-open —
irrelevant to the defect but easy to misread in tests.

---

## 6. Proposed fix

**Goal:** *any* registered slash command, submitted with its full name as the
first word (no arguments, or with recognized arguments), must route to its
`SlashCommandAction` handler — never to `backend.send_input` — regardless of
whether the popup was open, Tab-filled, or Esc-dismissed at submit time.

### 6.1 Minimal fix (recommended)

1. **`slash_parser.rs` — extend `parse_slash_command` with a registry-driven
   catch.** After the existing family branches, before the final
   `NotASlashCommand` fallback:
   - extract the first whitespace-delimited token of the trimmed text;
   - require it to start with `/`;
   - look up `&token[1..]` (case-insensitive) in `SLASH_COMMANDS` via
     `SlashCommandAction::name()`;
   - require the text to be *exactly* that token (no extra arguments) for the
     bare-only commands, since none of them accept arguments today;
   - return a new variant `SlashCommandParse::BareCommand(SlashCommandAction)`
     carrying the matched action.
2. **`dispatch_slash_commands.rs` — route it.** Add a `BareCommand(action)`
   arm in `handle_input_submitted` that calls the existing
   `self.handle_slash_command(action)` (the very handler the popup uses) and
   `return`s — no `send_input`, and per RPC-022 rule "Submitted slash
   commands DO NOT publish to persistence_add_history", no history append.
   Reusing `handle_slash_command` guarantees behaviour identical to a popup
   pick (single source of truth — no duplicated business logic, per
   AGENTS.md "Two Front Doors, One Source of Truth").
3. Leave `NotASlashCommand` as the final fallback for genuine prose and for
   `/unknown …` lines (existing tests in `slash_parser.rs:212` and
   `slash_command_wiring_rpc022.rs:88` depend on it).

Why the registry lookup instead of 11 hardcoded arms: the registry
(`SLASH_COMMANDS`) is already the single source of truth for names, the
popup filter, and the dispatch arms. A hardcoded list would immediately
diverge on the next command addition — the exact failure mode that created
this bug.

### 6.2 Alternative considered and rejected

Re-open/keep the popup alive after Tab (e.g. Tab only *fills*, popup stays
open until a space or Enter). Rejected because:
- it contradicts RPC-020 rule 2 and the spec example 11 ("popup closes");
- it breaks the legitimate `/role <text>`, `/schedule add …`, `/mux …`
  argument flows where the popup MUST go away at the first space;
- it does not fix the Esc-dismiss or plain-typing variants of the bug.

### 6.3 Out of scope (separate cards if ever wanted)

- `/provider <profile>` subcommands — no grammar exists yet (registry entry
  is bare-only, per RPC-054: *"Singular `/provider` only"*).
- Argument-carrying forms of the 11 bare-only commands (e.g.
  `/help <topic>`) — none exist.
- The TS Ink TUI — the registry comment says the Rust TUI mirrors
  `slashCommands.ts`, but no `src/tui` tree exists in this repo; the Rust
  TUI is the only consumer of `SLASH_COMMANDS`.

---

## 7. Files touched by the fix (planned)

| File | Change |
|---|---|
| `rust/fspec-tui/src/app/slash_parser.rs` | New `BareCommand(SlashCommandAction)` variant; registry lookup in `parse_slash_command` before the `NotASlashCommand` fallback; unit tests |
| `rust/fspec-tui/src/app/dispatch_slash_commands.rs` | New `BareCommand` arm in `handle_input_submitted` → routes to `handle_slash_command(action)` and returns (no `send_input`, no history append) |
| `rust/fspec-tui/src/views/agent/slash_commands.rs` | Possibly add a `SlashCommandAction::from_name(&str) -> Option<Self>` helper (single lookup site, testable in isolation) — or inline the lookup in the parser |
| New test file (e.g. `rust/fspec-tui/tests/slash_submit_intercept_bug168.rs`) | Integration tests via the `AppTestHarness` / `fresh_view` + key-event patterns in `tests/view_agent_popups_rpc020.rs` |

No changes needed in: `views/agent/dispatch_popups.rs`, `views/agent/popups.rs`,
`views/agent/slash_command_popup.rs`, `views/agent/multiline_input*.rs` — the
popup/Tab/Esc behaviour stays exactly as spec'd; only the submit-time
interception grows.

Imports note: `crate::app` already depends on
`crate::views::agent::slash_commands::SlashCommandAction`
(`dispatch_slash_commands.rs:16`), so the parser importing
`SLASH_COMMANDS` / the action enum is not a new module edge.

## 8. Test plan (ACDD: failing tests first)

New feature file: `spec/features/slash-submit-intercept-registry.feature`
(tags: `@bug @input @agent-view @tui`, work-unit tag `@BUG-169`).

Behavioural scenarios (each → integration test, `@step` comments required):

1. **Tab-then-Enter on a full command name submits the command, not the text**
   - type `/provider` (popup filters to `provider`), press **Tab** (input
     becomes `/provider`, popup closes), press **Enter**
   - then: `Action::InputSubmitted("/provider")` is NOT the terminal effect —
     the ProviderSettingsView opens (`ViewMode::ProviderSettings`), and
     `MockBackend.send_input_calls()` stays `0`.
2. **Same for every other bare-only command** (table-driven):
   `/help` → HelpDialog on compositor; `/clear` → scrollback+input reset;
   `/quit` → `should_quit`; `/resume` → resume view active;
   `/search` → search view active; `/debug` → debug toggle notice;
   `/compact` → `backend.compact_session` called; `/isolation` →
   create-session dialog (preselect Isolated); `/blocklist` → blocklist view;
   `/detach` → detach handler path; `/merge-worktree` → merge handler path.
   Assert in each: `send_input_calls() == 0`.
3. **Esc-dismiss then Enter also intercepts** (second trigger of §5.2):
   type `/provider`, **Esc** (popup closes, buffer intact), **Enter** →
   ProviderSettingsView, `send_input_calls() == 0`.
4. **No regression — arguments still flow to the LLM as prose unless
   recognized:** `/provider` with a trailing argument (`/provider openai`)
   is NOT a registered bare command-with-args → falls through to
   `send_input` (documented: only *exact* bare names intercept).
5. **No regression — unknown slash lines unchanged:** `/unknown anything`
   still → `send_input` (existing behaviour, asserted in
   `slash_command_wiring_rpc022.rs`).
6. **No regression — existing path-B families unchanged:** `/thinking high`,
   `/role clear`, `/goal show`, `/update check`, etc. still parse to their
   existing variants (re-run `slash_thinking_rpc048`, `slash_role_rpc063`,
   `loop_dispatch_rpc059`, `schedule_dispatch_rpc058`,
   `upd002_update_command_test`, `cont002_continue_command_test`).
7. **History:** intercepted bare commands do NOT append to
   `persistence_add_history` (RPC-022 rule, line 28-30 of its feature file).

Unit tests in `slash_parser.rs`:
- `parse_slash_command("/provider")` → `BareCommand(Provider)`
- `parse_slash_command("/HELP")` → `BareCommand(Help)` (case-insensitive)
- `parse_slash_command("  /clear  ")` → `BareCommand(Clear)` (trimmed)
- `parse_slash_command("/provider openai")` → `NotASlashCommand`
- `parse_slash_command("/")` → `NotASlashCommand` (empty name)
- `parse_slash_command("/unknown")` → `NotASlashCommand`
- existing `/thinking high`, `/role x`, `/mux` cases unchanged.

### Reproduction command (manual)

```
cargo run -p codelet-fspec -- --tui   # (or however the TUI is launched in dev)
# type: /provide → Tab → Enter     (bug: '/provider' goes to the LLM)
# type: /provider → Esc → Enter    (bug: same)
# type: /provider → Enter          (works: popup selects it)
```

---

## 9. Verification checklist for the fix

- [ ] `cargo check -p codelet-fspec-tui`
- [ ] `cargo clippy -p codelet-fspec-tui` (workspace denies: `unwrap_used`,
      `expect_used`, `panic`, `manual_find`, `manual_strip`, `redundant_clone`,
      `unnecessary_to_owned`, …)
- [ ] `cargo test -p codelet-fspec-tui --test slash_submit_intercept_bug168`
- [ ] `cargo test -p codelet-fspec-tui --test view_agent_popups_rpc020`
- [ ] `cargo test -p codelet-fspec-tui --test slash_command_wiring_rpc022`
- [ ] `cargo test -p codelet-fspec-tui --test provider_settings_dispatch_rpc054`
- [ ] `cargo test -p codelet-fspec-tui --test behaviour_parity_rpc065`
- [ ] `fspec validate` + `fspec validate-tags` on the new feature file
- [ ] Manual repro from §8 no longer sends `/provider` to the LLM

---

## Appendix A: Key source excerpts

### A1 — Popup Tab-fill closes the popup (`views/agent/dispatch_popups.rs:21-42`)
```rust
pub(super) fn handle_popup_key(&mut self, key: &KeyEvent) -> Option<EventResult> {
    if let Some(popup) = self.slash_popup.as_mut() {
        match popup.handle_key(key.code, key.modifiers) {
            PopupOutcome::Selected(action) => {
                self.slash_popup = None;
                self.input.reset();
                self.emit(Action::SlashCommandSelected(action));
                return Some(EventResult::consumed());
            }
            PopupOutcome::Filled(text) => {
                self.slash_popup = None;     // ← Tab: popup gone forever
                self.input.set_value(&text);
                return Some(EventResult::consumed());
            }
            PopupOutcome::Dismiss => {        // ← Esc: same, buffer kept
                self.slash_popup = None;
                return Some(EventResult::consumed());
            }
            ...
```

### A2 — Enter always submits once the popup is gone
(`views/agent/multiline_input_enter.rs:60-68`)
```rust
    // Plain Enter → submit, unless suppressed.
    if mods.is_empty() {
        if gate.suppress_enter {
            return Some(InputEventOutcome::Continued);
        }
        let buf = input.value();
        input.reset();
        return Some(InputEventOutcome::Submitted(buf));
    }
```

### A3 — The parser's gap (`app/slash_parser.rs:94-168`, tail)
```rust
pub fn parse_slash_command(text: &str) -> SlashCommandParse {
    let trimmed = text.trim();
    if trimmed == "/model" { return SlashCommandParse::OpenModelDialog; }
    ... // thinking, role, schedule, loop, continue, goal, update, mux
    SlashCommandParse::NotASlashCommand   // ← "/provider" lands here
}
```

### A4 — The submit dispatcher's fallthrough
(`app/dispatch_slash_commands.rs:287-314`)
```rust
            SlashCommandParse::NotASlashCommand => {}
        }
        // ...
        tokio::spawn(async move {
            let _ = backend.send_input(session_for_send, text_for_send).await;
        });
```

### A5 — The exhaustive path-A match (works)
(`app/dispatch_slash_commands.rs:104-110`)
```rust
            SlashCommandAction::Provider => {
                // RPC-054: open the ProviderSettingsView. Singular
                // `/provider` only — the TypeScript Ink reference
                // (slashCommands.ts) defines exactly one entry whose
                // `name` is `'provider'`; no `/providers` alias.
                let _ = self.action_tx.send(Action::OpenProviderSettingsView);
            }
```

## Appendix B: Prior art / related work units

| Work unit | What it did | Relation |
|---|---|---|
| RPC-020 | Popup + palette + Tab-fill (popup closes) | Defined the popup semantics; spec example 11 anticipated the trap |
| RPC-022 | `parse_slash_command` for `/model`, `/thinking`, `/role` | First path-B interceptor — established the "extend the parser per command" pattern |
| RPC-047, RPC-048, RPC-050, RPC-055, RPC-056, RPC-057, RPC-058, RPC-059, RPC-060, RPC-063 | Wired each command's path-A handler | Several (047 compact, 050 detach, 055 debug, 056 blocklist, 057 merge-worktree) never added path-B coverage |
| CONT-002/003, UPD-002, MUX-001 | Added subcommand families | All *do* have path-B coverage (arguments force the popup closed) |
| RPC-054 | `/provider` → ProviderSettingsView | Path A only — the reported instance |

## Summary

- **One-line root cause:** `parse_slash_command` (the submit-time interceptor in
  `handle_input_submitted`) only recognizes 11 of the 21 registered slash
  commands; every other command that reaches submit with the popup closed —
  via Tab-fill, Esc-dismiss, or plain typing — falls through to
  `backend.send_input` and is sent to the LLM verbatim.
- **Fix:** route unmatched-but-registered bare command names through the
  existing `handle_slash_command(SlashCommandAction)` handler by looking the
  first token up in the `SLASH_COMMANDS` registry, instead of letting it
  become prose.
- **Effort:** small (parser + one dispatch arm + tests); no popup/UI changes.
