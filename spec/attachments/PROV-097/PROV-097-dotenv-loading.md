# PROV-097 — `fspec` binary never loads `.env`

## Symptom
Providers whose API key is only present in a `<cwd>/.env` file render as
`(not configured)` in the Rust `fspec` TUI, while the TS build shows them as
`✓ … [env]`.

## Root cause
The TUI the user runs is the pure-Rust `fspec` binary:

```
codelet/fspec/src/main.rs::main()
  → combined::run(cli.workspace).await       (codelet/fspec/src/combined.rs)
      → App::new(Arc::new(backend))          (codelet_fspec_tui)
      → app.run(...)
```

**Neither `fspec/src/main.rs` nor `combined.rs` calls `dotenvy::dotenv()`.**

Confirmed `dotenvy::dotenv()` call sites in the workspace (non-test):
- `codelet/cli/src/main.rs:9`        — different binary (`codelet`, not `fspec`)
- `codelet/sessions/src/session_manager.rs:425` — only at *session creation*
- `codelet/sessions/src/session_manager.rs:666` — only at *session creation*
- `codelet/napi/src/session_bindings.rs:2861,3470,3479,3497` — the TS/NAPI path

So in the **TS/NAPI build**, NAPI startup runs `dotenvy::dotenv()` which sets
*real* process env vars; both Rust detection AND Node `process.env` then see
them, tagged `[env]`.

In the **pure-Rust `fspec` binary**, `.env` is never loaded before Provider
Settings opens, so `ProviderCredentials::detect()` →
`std::env::var(...)` finds nothing → everything is "not configured."

## Fix direction
Call `dotenvy::dotenv()` once at `fspec` binary startup (in `main.rs` before
dispatching to `combined::run`, or at the top of `combined::run`) so `<cwd>/.env`
is loaded into the process environment before the TUI renders. Use
`let _ = dotenvy::dotenv();` (non-overriding, ignore "file not found"), mirroring
`cli/src/main.rs:9`.

## TS reference behaviour
TS additionally re-parses `<cwd>/.env` via dotenv `parse()` (NOT `config()`) as a
*lowest-priority fallback* in `getProviderConfig()` (`src/utils/credentials.ts`
lines 249-264), tagging such keys `source: 'dotenv'`. For the Rust binary the
simplest parity is to load `.env` into the real env at startup (it then surfaces
as `[env]`). Tagging `[dotenv]` distinctly is optional and overlaps PROV-098.

## Files in play
- `codelet/fspec/src/main.rs`
- `codelet/fspec/src/combined.rs`
- (reference) `codelet/cli/src/main.rs:9`

## Acceptance pointers
- Given a `<cwd>/.env` containing `ANTHROPIC_API_KEY=sk-ant-...`, when the `fspec`
  TUI opens Provider Settings, the env var is visible to `std::env::var`.
- Must be offline/deterministic: drive via a temp working dir + temp `.env`, not
  the real home or network.
