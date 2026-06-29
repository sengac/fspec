# RPC-387 — Empty Supervisor Message Body in Subordinate View

## Symptom

When a supervisor agent sends a message to a subordinate (worker) agent, the
subordinate's TUI scrollback shows only the worker prefix:

```
[W] reviewer>
```

…with **no purple (magenta) message body** after it. The role is rendered
correctly, but the actual message text is blank.

## Root Cause: Separator Mismatch Between Backend and TUI Parser

There are two collaborating pieces that disagree on the envelope separator.

### 1. Backend produces a SPACE-separated envelope

`codelet/sessions/src/background_session.rs:230`

```rust
/// Format: `[SUPERVISOR: role | Session: id] message`
pub fn format_incoming_message(input: &IncomingMessage) -> String {
    format!(
        "[SUPERVISOR: {} | Session: {}] {}",
        input.role_name, input.source_session_id, input.message
    )
}
```

The wire string is, e.g.:

```
[SUPERVISOR: reviewer | Session: s-2] please check this
```

Note the **space** between `]` and the body — there is **no newline**.

This is the canonical wire format. It is also what the NAPI bindings assert
(`codelet/napi/src/session_bindings.rs:683`), and it is the exact string the
working TypeScript frontend consumes.

### 2. Rust TUI parser splits on NEWLINE

`codelet/fspec-tui/src/store/agent_view/session_context.rs:207`

```rust
fn parse_supervisor_envelope(raw: &str) -> (String, String) {
    let mut parts = raw.splitn(2, '\n');          // <-- splits on '\n'
    let header = parts.next().unwrap_or("");
    let body = parts.next().unwrap_or("").to_string();
    if !header.starts_with('[') {
        return ("supervisor".to_string(), raw.to_string());
    }
    let inner = header.trim_start_matches('[').trim_end_matches(']');
    let role_segment = inner.split('|').next().unwrap_or(inner).trim();
    let role = role_segment
        .strip_prefix("SUPERVISOR:")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "supervisor".to_string());
    (role, body)
}
```

Given the real backend string `"[SUPERVISOR: reviewer | Session: s-2] please check this"`:

- `splitn(2, '\n')` finds **no newline**, so `header` = the **entire string**
  and `body` = `""`.
- `header` starts with `[`, so the role is parsed correctly (`reviewer`).
- The returned body is **empty**.

The renderer at `session_context.rs:124-132` then builds:

```rust
text: format!("[W] {role}> {body}")   // -> "[W] reviewer> "
```

…producing the prefix with no body.

### 3. Why the TS reference works

`src/tui/utils/chunkProcessor.ts:83`

```ts
const match = text.match(/^\[SUPERVISOR: ([^|]+) \| Session: ([^\]]+)\]\n?/);
// content = text.slice(match[0].length)
```

The TS regex consumes the `[...]` header and an **OPTIONAL** newline (`\n?`).
For the space-separated form it matches just up to `]`, leaving the remaining
text (` please check this`) as the body. So the TS path renders the body
regardless of whether the separator is a space or a newline.

The Rust port hard-codes a newline split and therefore loses the body whenever
the separator is a space — which is the real backend's format.

### 4. Why existing tests don't catch it

Every Rust test for this function feeds the **newline** form:

- `session_context.rs:262` — `"[SUPERVISOR: reviewer | Session: s-2]\nplease check this"`
- `chunk_rendering_parity_rpc078.rs:295` — same newline form

No test exercises the actual backend output from `format_incoming_message`
(space-separated), so the mismatch is invisible to the suite. This is a test
gap that the fix must close.

## The Fix

Make `parse_supervisor_envelope` robust to **both** separators by splitting on
the header's closing `]` (mirroring the TS regex approach) and trimming any
leading separator characters (space or newline) from the body.

Suggested implementation:

```rust
/// Parse a `StreamChunk::IncomingMessage` body of the form
/// `"[SUPERVISOR: <role> | Session: <sid>]<sep><body>"` where `<sep>` is a
/// space or a newline (the backend uses a space; replay paths may use `\n`).
fn parse_supervisor_envelope(raw: &str) -> (String, String) {
    if !raw.starts_with('[') {
        return ("supervisor".to_string(), raw.to_string());
    }
    let Some(close_idx) = raw.find(']') else {
        return ("supervisor".to_string(), raw.to_string());
    };
    let header = &raw[..close_idx]; // excludes ']'
    let body = raw[close_idx + 1..]
        .trim_start_matches(['\n', ' '])
        .to_string();
    let inner = header.trim_start_matches('[');
    let role_segment = inner.split('|').next().unwrap_or(inner).trim();
    let role = role_segment
        .strip_prefix("SUPERVISOR:")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "supervisor".to_string());
    (role, body)
}
```

### Behaviour table after fix

| Input | role | body |
|-------|------|------|
| `[SUPERVISOR: reviewer \| Session: s-2] please check this` (space — real backend) | `reviewer` | `please check this` |
| `[SUPERVISOR: reviewer \| Session: s-2]\nplease check this` (newline — replay) | `reviewer` | `please check this` |
| `raw body without header` (no `[`) | `supervisor` | `raw body without header` |
| `[SUPERVISOR: reviewer \| Session: s-2]` (no separator/body) | `reviewer` | `` (empty) |

## Files Involved

| File | Role |
|------|------|
| `codelet/sessions/src/background_session.rs:230` | Backend producer (space separator) — unchanged |
| `codelet/fspec-tui/src/store/agent_view/session_context.rs:207` | **Parser to fix** |
| `codelet/fspec-tui/src/store/agent_view/session_context.rs:124-132` | Renderer that builds `[W] role> body` |
| `src/tui/utils/chunkProcessor.ts:83` | TS reference (parity target) |

## Scope / Non-Goals

- **In scope:** Fix the Rust parser so the body is populated for the
  space-separated backend format; add regression tests covering the space form.
- **Out of scope:** Changing the backend wire format (it is shared with the
  working TS path and asserted by NAPI tests). The leading-space cosmetic in TS
  is not replicated — the Rust fix trims it for a cleaner render.
