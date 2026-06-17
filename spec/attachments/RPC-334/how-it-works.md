# codelet-fspec-json-error — how it works & how to use it

**Attachment for RPC-334.** Companion crate vendored from
[`format_serde_error`](https://github.com/AlexanderThaller/format_serde_error)
v0.3.0 (MIT, Alexander Thaller), trimmed and bug-fixed for fspec.

Location: `codelet/fspec-json-error/`

---

## 1. Why vendor instead of depend?

- The upstream crate is ~4 years old, pre-1.0, last released `0.3.0`, with open
  bugs and stale `serde_yaml 0.8` / `toml 0.5` deps we don't want in our tree.
- We need exactly one of its three backends (`serde_json`) and none of its
  optional `colored` output (fspec colours at the CLI bridge layer, not in
  core — keeping core colour-free makes its `Display` byte-stable for tests).
- Vendoring lets us fix the open panic bug (#20) and drop the un-idiomatic
  global mutable state (#19) without waiting on upstream.

License/attribution preserved in the crate-level doc comment in `src/lib.rs`.

## 2. What it does

Given the **raw source text** and a `serde_json::Error`, it renders the
offending line(s) with a caret under the exact error column:

```
Error:
   | {
   |   "version": "0.7.1",
   |   "workUnits": {
 4 |     "AUTH-001": { "id": "AUTH-001", status: "done" }
   |                                      ^ key must be a string at line 4 column 37
   |   }
   | }
```

Single-line input:

```
 1 | { bad
   |    ^ key must be a string at line 1 column 3
```

It uses `serde_json::Error::line()` / `.column()` for positioning, shows
`CONTEXT_LINES_DEFAULT = 3` lines of surrounding context, and shortens
over-long lines to `CONTEXT_CHARACTERS_DEFAULT = 30` chars either side of the
error (with `...` ellipses), so a corrupted *minified* file won't dump a
megabyte to the terminal.

## 3. Public API

```rust
use codelet_fspec_json_error::SerdeError;

// Build from the raw input + the serde_json error.
let err = serde_json::from_str::<serde_json::Value>(input).unwrap_err();
let pretty: String = SerdeError::from_json(input.to_string(), &err).to_string();

// Optional per-instance tuning (no global state):
let mut se = SerdeError::from_json(input.to_string(), &err);
se.set_contextualize(false)      // only the error line, no surrounding context
  .set_context_lines(1)          // N lines before/after
  .set_context_characters(20);   // window for long-line shortening
```

- `SerdeError` implements `std::error::Error` + `Display` (+ `Debug`, `Clone`).
- When the error has no location (rare), `Display` falls back to the bare
  `serde_json` message — no caret, no panic.

## 4. Differences from upstream 0.3.0

| Change | Why |
|--------|-----|
| Dropped `serde_yaml`, `toml`, `colored` features + code paths | fspec only parses JSON; colour belongs at the bridge layer |
| Removed global `AtomicBool`/`AtomicUsize` config + `set_default_*` free fns | un-idiomatic library global state (upstream #19); config is now per-instance |
| Removed buggy `get_default_contextualize` (returned `CONTEXT_LINES` as a `usize` for a bool concept) | latent bug |
| **Fixed upstream #20 "Panic on small characters count"** | see below |
| `from_json(input, &err)` constructor instead of generic `new(input, impl Into<ErrorTypes>)` | single backend, no enum needed |
| Inlined format args, no `unwrap`/`expect`/`panic` | comply with workspace deny-lints |

### The #20 panic fix (root cause)

Upstream computed the caret position as:

```rust
column = error_column - whitespace_count + ellipse_space   // underflow → panic
```

When a long line was contextualised, `error_column` had **already** been
re-based by the windowing (`context_long_line`), yet `whitespace_count` (the
common leading-indent that was stripped from the displayed text) was subtracted
**again**. With a small `context_characters` the re-based column drops below
`whitespace_count` → `usize` subtraction overflow (panic in debug).

Fix: de-indent the error column **once**, *before* windowing, and never
subtract `whitespace_count` in `format_error_information` again. This both
removes the panic and corrects the caret position for the contextualised case.
Regression test: `tests::issue_20_small_context_characters_does_not_panic`
(verbatim repro from the issue) plus `context_characters_zero_does_not_panic`.

## 5. How fspec will use it (integration design — see inventory attachment)

The single integration point is `FspecCoreError::ParseJson`. Today its `reason`
field holds `serde_json::Error::to_string()` (e.g. `key must be a string at
line 1 column 3`). The plan:

1. Add a shared helper (e.g. `io::json_error::parse_json_diagnostic(file_label,
   input, err) -> FspecCoreError::ParseJson`) that runs `input` + `err` through
   `SerdeError::from_json` and stores the rendered snippet as `reason`.
2. Route the **shared funnel** `io/locked_file.rs::read_or_init_json` through
   it — this alone upgrades every `ensure_*`-based command for free.
3. Route the per-command direct-parse sites (they already hold the raw input as
   `raw`/`buf`/`content`) through the same helper.
4. **Remove** the fabricated `Unexpected token in JSON:` prefix from the 6
   commands that prepend it (the V8-emulation leftover this card targets).

Diagnostic value, not V8 byte-parity, is the goal — this is a deliberate,
documented divergence from the TS frontend (which surfaces V8/`JSON.parse`
wording pinned to whatever Node version it runs on).
