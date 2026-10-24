# PROV-160 implementation and validation

Date: 2026-10-20

## Recovered context

- Original implementation session: `3405740c-f116-40d7-9944-16794322528b`.
- Previous recovery session: `68b285f8-c4e9-4c37-b211-c792eaa2c624`.
- Original status/CLI worker: `7e41fab7-1d32-4a91-accc-00fe50ab8e4f`.
- Current recovery session: `fa06abd0-4c9a-4194-860d-3186fb67b99a`.

Recovered the existing specifications, failing-test history, implementation and local validation logs rather than restarting the card. Preserved the approved policy: five stream retries, separate cancellable offline waits of 5/10/20/40/60 seconds for all sessions, the 120-second network idle deadline, selected transport and full-current-DAG input.

## Additional fixes during recovery

- DNS resolution failures previously escaped connection classification on both transports. A typed system resolver now preserves offline-wait ownership without treating arbitrary TLS/protocol errors as connection failures. WebSocket DNS, TCP and TLS setup remain within the existing setup deadline.
- DeepSearch and compaction collectors previously swallowed terminal Codex errors containing generic network-error text. They now use the same provider/type-aware ownership check as the CLI. Updated the NAPI mirror and existing collector tests consistently; hard task deadlines remain unchanged.
- Both fixes followed specification, observed failing regression, then implementation. Repaired stale scenario coverage and added implementation mappings.

## Final review fixes

- Network-interface-down setup failures now enter cancellable offline waiting on both transports, including Unix ENETDOWN and Windows WSAENETDOWN.
- Permanent WebSocket protocol, UTF-8, capacity and binary-message failures now terminate without replacement requests. Abrupt connection loss still uses bounded same-transport recovery; send and heartbeat failures share the classifier.
- Live NAPI dispatch now initializes provider session extensions, registers the existing session status observer and attaches the provider's session state before building an agent. This shares recovery state with the watchdog for foreground and subordinate sessions, including provider switches.
- Added executable regressions before each production fix. The NAPI test imports the actual production helper and exercises a real Codex provider through an unavailable loopback proxy rather than manually injecting a recovery handle.

## Passing validation

- Final combined scoped run: **54 tests passed** across 16 integration targets in `codelet-providers`, `codelet-cli` and `codelet-agent-loop`.
- Provider targets: `cmpct081_explicit_transport`, `cmpct082_physical_admission`, `prov149_codex_history`, `prov149_codex_interruption`, `prov154_codex_deadline`, `prov154_codex_recovery`, `prov159_live_clients`, `prov160_codex_dns`, `prov160_codex_offline`, `prov160_codex_reconnection`, `prov160_network_failure_classification`.
- Agent-loop targets: `prov149_codex_reload`, `prov160_napi_session_recovery`, `prov160_reconnect_status`, `prov160_subagent_recovery_ownership` (the latter also executes 18 existing collector/configuration tests).
- CLI target: `prov160_reconnect_watchdog`.
- `cargo check` and library Clippy pass for those three packages plus `codelet-sessions`, with build concurrency limited to four and debug information disabled.
- The repository's 1970 existing feature files and both new review specifications pass Gherkin syntax validation; changed Rust files pass scoped formatting checks; `git diff --check` passes.
- All eleven PROV-160 feature coverage audits pass: 25 scenarios have 25 test mappings and 70 production mappings, with each feature reporting 100% coverage. New specifications carry `@done` rather than `@wip`.

Final logs: `/tmp/prov160-completion-final-tests.log`, `/tmp/prov160-completion-final-check.log`, `/tmp/prov160-completion-final-clippy.log`, `/tmp/prov160-completion-final-format.log`. Earlier feature and coverage validation: `/tmp/prov160-final-coverage.log`, `/tmp/prov160-final-features.log`. RED/GREEN evidence: `/tmp/prov160-dns-red.log`, `/tmp/prov160-dns-final.log`, `/tmp/prov160-subagent-ownership-red.log`, `/tmp/prov160-subagent-ownership-green.log`, `/tmp/prov160-classification-red.log`, `/tmp/prov160-classification-green.log`, `/tmp/prov160-napi-session-red.log`, `/tmp/prov160-napi-session-green.log`.

## Broader validation limitations

- Provider library suite: 351 passed and four failures in untouched legacy Codex default-capacity assertions (expecting 272000, receiving 0). See `/tmp/prov160-recovered-unit.log`.
- Legacy `prov156_codex_websocket` tests still assume removed delta/fallback behavior and omit current admission limits. Four failed and two remained pending; stopped that test binary rather than changing unrelated obsolete expectations. The current explicit-transport tests pass. See `/tmp/prov160-dns-green.log`.
- NAPI compilation still stops in `rust/rpc-types/src/lib.rs:1499`: `RequestContextSnapshot` lacks `ToNapiValue`/`FromNapiValue`. Reproduced after the final review fix in `/tmp/prov160-napi-check.log`; the actual NAPI recovery helper passes its executable integration test independently.
- Repository-wide tag validation reports 526 feature files with violations outside the changed Codex specifications. Package-wide formatting also reports unrelated existing differences; those files were not reformatted.

PROV-160's scoped acceptance criteria and quality checks pass, so the card is complete. The unrelated broader failures above remain documented rather than being hidden or expanded into this card. No commit, branch creation or live-provider request was performed.
