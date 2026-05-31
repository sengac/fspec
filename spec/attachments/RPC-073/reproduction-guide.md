# RPC-073 — Reproduction Guide

> How to reproduce each bug on the `codelet-integration` branch using the
> freshly built fspec Rust binary.

---

## Prerequisites

```bash
cd /Users/rquast/projects/fspec/codelet
cargo build --release
cd ..
# Binary lives at: codelet/target/release/fspec
```

`~/.fspec/fspec-config.json` must exist with at least one configured provider
(anthropic credentials work for the happy path).

---

## Bug 1: `/clear` panics

### Steps

```bash
./codelet/target/release/fspec
```

1. Wait for `PORT=NNNNN` banner.
2. BoardView appears — navigate to any DONE Work Unit (e.g. RPC-071).
3. Press `Enter` to open a Work Agent.
4. Type any text and press Enter (e.g. `hello`). Wait for the assistant
   response or for the provider to error.
5. Type `/clear` and press Enter.

### Expected

Session scrollback clears. Status returns to Idle. No process crash.

### Actual

```
thread 'tokio-rt-worker' (NNNN) panicked at sessions/src/background_session.rs:1156:36:
Cannot block the current thread from within a runtime.
```

The process crashes; the alt-screen is left in a broken state until the user
runs `reset`.

---

## Bug 2: `?` opens HelpDialog while typing

### Steps

```bash
./codelet/target/release/fspec
```

1. Open any DONE card's Work Agent.
2. With the input field focused, type any message that contains `?`,
   e.g. `is this card done?`.

### Expected

The `?` character is inserted into the input buffer alongside the rest of
the message.

### Actual

The HelpDialog popup is pushed onto the Compositor and the `?` is consumed
by the app-shortcut handler. Pressing `q` to dismiss the dialog then quits
the app entirely (because `q` is also a trapped app-shortcut). The user has
to press `Esc` to dismiss the dialog and continue typing — but they still
cannot include `?` in any message.

The same issue affects the BoardView's slash-detail page if the user types
`?` to search for help syntax — `?` is unreachable from any context except
as a global help-popup trigger.

---

## Bug 3: Model selector list is empty

### Steps

```bash
./codelet/target/release/fspec
```

1. Open any DONE card's Work Agent.
2. Press the model-selector shortcut (`/model` or whatever opens
   `OpenModelDialog`).

### Expected

A list of providers from `~/.fspec/fspec-config.json` AND
`~/.fspec/credentials/` AND `~/.fspec/providers/` shows up, with the
currently-selected model (`anthropic/claude-opus-4-7`) highlighted.

### Actual

The dialog opens with an empty list. The `list_providers` RPC returns
`Vec::new()` (see root-cause-analysis.md §3).

---

## Confirmation that RPC-072 wiring works

Despite the model dialog being broken, the agent loop IS wired and IS using
the model from `tui.lastUsedModel`:

In the same test session, the user typed `is this card done` and saw the
following chunks arrive in the AgentView scrollback:

```
user> is this card done
[error] provider error: [claude] API error: Rig completion failed: HttpError:
        Invalid status code 429 Too Many Requests with message: {"type":"error",...}
[done]
```

The `[error]` came from the actual anthropic API (429 rate limit). The
`[done]` chunk arrived through `BackgroundSession::handle_output` →
chunks_tx broadcast → AgentView dispatch_rpc045. That is RPC-072's headline
flow working end-to-end.
