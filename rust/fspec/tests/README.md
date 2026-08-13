# codelet-fspec integration tests

This directory contains end-to-end integration tests for the `fspec`
binary. Each `*.rs` file is a separate cargo integration-test binary;
they share the helpers in `common/mod.rs`.

## Test inventory

| Test file | What it covers |
|---|---|
| `daemon_mode.rs` | `fspec daemon` headless boot, port banner contract, WS surface |
| `client_mode.rs` | `fspec client` thin frontend against a running daemon |
| `combined_smoke.rs` | `fspec` combined-mode boot (TUI + WS in one process) — `#[ignore]`'d (needs `/dev/tty`) |
| `daemon_lifecycle_rpc011.rs` | RPC-011 daemon.json schema + lifecycle |
| `stale_daemon_json_rpc011.rs` | RPC-011 stale-PID detection |
| `status_subcommand_rpc011.rs` | `fspec status` health snapshot |
| `cargo_shape.rs` | Workspace dependency-graph regression guards |
| `no_napi_dependency.rs` | RPC-067 — proves the fspec binary has zero NAPI dep arrows |
| `cross_frontend_parity.rs` | **RPC-066** — drives every slash command end-to-end against a stub LLM provider and asserts the chunk stream matches a pinned golden |

## RPC-066 — Cross-frontend parity test

`cross_frontend_parity.rs` is the foundational regression net for the
Rust-frontend port (RPC-002 epic, RPC-030 Phase 8.2). It boots a real
`fspec daemon` subprocess against a deterministic stub `LlmProvider`,
drives a scripted run of every user-facing slash-command equivalent
through a `WebSocketFspecBackend`, normalises the captured chunk stream
(timestamps / UUIDs / correlation IDs / tool-call IDs → placeholders),
and asserts byte-equality against `tests/fixtures/cross_frontend_run.jsonl`.

The end-to-end test bodies are `#[ignore]`'d in the default cargo
invocation because they:

  1. spawn the real `fspec daemon` subprocess (slow, needs the
     `test-stub-provider` cargo feature compiled in)
  2. exercise the full SessionManager + agent-loop wiring against a
     real (deterministic) provider

To run the full suite:

```
cargo test --features test-stub-provider -p codelet-fspec --test cross_frontend_parity -- --include-ignored
```

The source-shape and normalisation-pipeline tests inside the file run on
every default `cargo test` invocation (they have no `#[ignore]` and no
subprocess dependency).

## Regenerating cross_frontend_run.jsonl

The golden fixture at `tests/fixtures/cross_frontend_run.jsonl` is the
pinned reference chunk stream. Re-record it whenever the legitimate
chunk emission shape changes (new chunk variant, new field, etc.):

```
rm rust/fspec/tests/fixtures/cross_frontend_run.jsonl
FSPEC_RPC_066_REGENERATE=1 \
  cargo test --features test-stub-provider \
             -p codelet-fspec --test cross_frontend_parity \
             scenario_scripted_run_matches_golden -- --include-ignored
```

The regeneration codepath captures the full scripted run, normalises it,
and writes the JSONL file. It exits successfully without asserting
against the file — re-run the test WITHOUT `FSPEC_RPC_066_REGENERATE` to
confirm the freshly-written golden actually round-trips.

When the fixture is missing AND `FSPEC_RPC_066_REGENERATE` is unset, the
test fails with a clear hint pointing at the regeneration command above.

## Future: TS-recorded reference fixture

The current `cross_frontend_run.jsonl` is a **Rust-pinned golden** — it
captures the chunk stream produced by the Rust `fspec daemon` against
the stub provider. The original RPC-066 acceptance criteria
("Capture chunks. Assert the chunk stream matches the equivalent
TS-frontend run against the same stub provider") calls for a
TypeScript-recorded reference fixture instead.

The TS-side fixture is deferred to a follow-up card (the next RPC card
in the backlog after RPC-068) because:

  1. The TS Ink frontend does not yet have a stub-provider boot recipe.
     A separate card needs to document how to launch the TS frontend
     against the same `register_stub_provider()` equivalent.
  2. The Rust-pinned golden is the highest-value asset right now — the
     Rust frontend has reached structural parity with the TS reference
     (RPC-029..065 are all done), so any regression in the Rust chunk
     stream is caught by the current golden.

Once the TS-side stub boot recipe is documented, the follow-up card
will:

  1. Record a fresh `ts_reference_run.jsonl` from the TS frontend.
  2. Replace `cross_frontend_run.jsonl` with that file (verbatim).
  3. Re-run the Rust-side test to confirm byte-equality.

Track the follow-up under the next available RPC story id in the
rust-frontend epic.
