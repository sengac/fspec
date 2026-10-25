# BUG-256 ACDD validation

## Discovery and implementation review

Native Fspec records the user story, two rules, two examples, answered question,
four scenarios, and testing/implementing/validating transitions. Tests precede
the production repair. The initial scoped run compiled and produced two genuine
behavioral failures: missed-clear resume retained old generation text, and an
archive installed during partial hydration never appeared. Two unchanged-
generation and transient-failure controls already passed.

The repair preserves a typed generation mismatch from the embedded/WebSocket
backend contract to page dispatch. Known mismatch replaces per-tab hydration,
aborts old handles, rotates the nonce and resets rendered generation state before
requesting an unpinned snapshot. Old page/inspection/resume responses cannot
match the new nonce. Ordinary reconnect and transient failures keep deduplication
and displayed history. Resume/failure handling moved mechanically to a focused
sibling module so every changed Rust file stays below 300 lines.

Reviewed the complete TUI diff for generation-vs-transport classification,
request/nonce rejection, partial payload and snapshot disposal, inspector abort,
display replacement rather than snapshot concatenation, and unchanged model,
token and input-draft state. No provider, session archive or transport business
logic was duplicated or changed.

## Tests

All commands ran from `rust/`, offline, scoped, with `--profile ci-test -j2`:

| Command (all begin `cargo test --offline --profile ci-test`) | Result |
| --- | --- |
| `-p codelet-fspec-tui --test bug256_generation_recovery -j2` | 4 passed |
| `-p codelet-fspec-tui --test slash_resume_rpc049 -j2` | 5 passed |
| `-p codelet-fspec-tui --test bug244_rendered_transcript resuming_a_previously_restored_active_session_preserves_runtime_state -j2` | 1 passed, 4 intentionally filtered |
| `-p codelet-rpc-types --test transcript_fragments -j2` | 6 passed |
| `-p codelet-providers --test prov149_codex_history -j2` | 2 passed |
| `-p codelet-agent-loop --test prov149_codex_reload -j2` | 3 passed |

Total: **21 green tests**. The generation tests use real temporary stores,
SessionManager, embedded RPC, clear, archive installation, multi-page Unicode
content, and actual queued late responses. They do not open listening sockets
or call external providers. The two Codex regression targets were requested by
the native validation workflow and use existing offline fixtures.

## Quality and traceability

Successful commands:

```text
cargo check --offline --profile ci-test -p codelet-fspec-tui --lib -j2
cargo clippy --offline --profile ci-test -p codelet-fspec-tui --lib --test bug256_generation_recovery -j2
cargo check --offline --profile ci-test -p codelet-providers -p codelet-cli -p codelet-sessions -p codelet-agent-loop --lib -j2
cargo clippy --offline --profile ci-test -p codelet-providers -p codelet-cli -p codelet-sessions -p codelet-agent-loop --lib -j2
```

Clippy completes successfully with existing dependency warnings and an unused
`backends` method in the reused BUG-244 test fixture. No lint was disabled and no
unrelated warning was repaired. Targeted `rustfmt --check` and `git diff --check`
pass. Native `validate`, `validate-tags`, and `audit-coverage` pass for
`transcript-generation-recovery` (all 8 mapped files found).

The entire TUI suite was intentionally not launched: it includes socket-based
and unrelated heavyweight targets. Relevant scoped regressions are listed
explicitly rather than claiming an unrun full workspace suite.

Disk free after validation: **8.1 GiB**, above the 6 GiB floor. Changes remain
uncommitted; no release, publication, gate changes or prior checkpoint deletion.
