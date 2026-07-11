# Review Findings — Compaction Batch (CMPCT-039/040/041 + RPC-421) — 2026-07-09

> NOTE (fix worker): this attachment was found EMPTY (0 bytes) at fix time,
> despite the supervisor's write at 2026-07-09T01:55 recording 5505 bytes —
> the content was apparently lost after writing (the Write tool call args are
> not recoverable from session history). The findings themselves were fully
> restated in the supervisor's fix assignment message, which was used as the
> authoritative source. Original verdicts: RPC-421 FAIL (C1-C4 + W1),
> CMPCT-039 WARN (W1-W3), CMPCT-040 WARN (W1-W2 + observation),
> CMPCT-041 WARN (W1-W6; W1 spun off as CMPCT-042).

## Fix Results

### RPC-421 (FAIL → fixed)

- **C1 — three artifacts falsely claimed `format_compaction_notice` stays in
  dispatch_slash_commands.rs** (it lives in `dispatch_stream_chunks.rs:242-251`,
  sole caller `:166`):
  - Work-unit architecture note [1]: removed and re-added corrected via Fspec
    (now states MOVED to dispatch_stream_chunks.rs :242-251, sole caller :166,
    slash handler no longer references the helper).
  - `single-sourced-compaction-notice.feature` doc string: replaced via
    `add-architecture` with the corrected two-paragraph text.
  - `slash-command-compact.feature:19`: doc string replaced via
    `add-architecture`; final paragraph now reads "was MOVED by RPC-421 from
    app/dispatch_slash_commands.rs to app/dispatch_stream_chunks.rs (:242-251)";
    all other paragraphs preserved verbatim.
- **C2 — stale implMappings 108-139 on 3 slash-command-compact scenarios**:
  re-linked "/compact calls backend.compact_session…", "/compact emits an error
  notice…on Err", "/compact with no current session is a silent no-op" to the
  actual Compact arm `dispatch_slash_commands.rs:65-97` (verified by reading the
  file) via unlink-coverage + link-coverage. Also corrected their testLines to
  the verified spans 133-154 / 213-234 / 240-267.
- **C3 — stale testLines on 3 scenarios**: SessionFooter renders → 346-382,
  SessionFooter omits → 388-417, CompactionComplete clears → 424-474 (all
  verified against test bodies; closing braces confirmed by awk). While
  re-linking, the CompactionComplete scenario's implMapping (previously
  dispatch_stream_chunks.rs 96-120 — drifted onto FooterStateUpdate code) was
  re-pointed at the actual CompactionComplete arm 125-170. Two additional
  drifted testLines found and fixed in the same file: "/compact Ok emits no
  success notice…" 156-206 → 162-206 and "only affects the focused session…"
  269-331 → 274-331.
- **C4 — rust-tui-compact-real-compaction unknown-session scenario**: testLines
  282-305 → 299-322 (verified: attribute :299, fn :300, file end :322); impl
  range updated 261-341 → 261-350 (compact_session now closes at :350).
- **W1 — @RPC-421 tag registration**: verified ALREADY registered as
  `@rpc-421` (Technical Tags: "RPC-421: single-sourced compaction notice +
  honest compact_session acknowledgement") — registered during the original
  arc, consistent with the lowercase `@cmpct-039/040/041` convention.
  validate-tags passes for both RPC-421 features; no action needed.

### CMPCT-039 (WARN → fixed)

- **W1 — stale architecture doc string**: rewrote both the work-unit
  architecture note [0] (remove + re-add) and the
  `compression-ratio-clamping.feature` doc string (`add-architecture`) to state
  post-RPC-421 reality: live helper callers are BOTH inject_summary_handler
  twins (CompactionComplete producers feeding every ratio-formatting
  notice/badge path) plus debug-only recovery_compaction.rs:450; the
  compact_session RPC twins and repl_loop no longer call the helper — they ship
  the acknowledgement sentinel directly and the REPL prints no ratio.
  (Verified by grepping all `compression_ratio(` call sites.)
- **W2 — helper impl coverage drift**: re-linked all 4 helper scenarios from
  interactive_helpers.rs 188-194 → 190-196 (verified: fn at :190, closing brace
  :196). Scenarios 5-6 (handle_impl 334-349 / session_bindings 3131-3146)
  verified still accurate — untouched.
- **W3 — example map sync**: examples [4] and [5] removed + re-added with
  "(Amended by RPC-421) … compression_ratio 0.0 (now the acknowledgement
  sentinel)" wording matching the feature comments. Assumption [1]: fspec has
  NO remove-assumption command, so a superseding assumption was ADDED stating
  growth is NOT recoverable from the RPC result fields and real data ships
  only on the CompactionComplete chunk producers (deviation documented in the
  assumption text itself).

### CMPCT-040 (WARN → fixed)

- **W1 — comments-only impl range**: scenarios 1+3 of
  `compaction-badge-sign-integrity.feature` re-linked from
  dispatch_stream_chunks.rs 145-152 to 148-155 (CMPCT-040 comment 148-152 +
  executable clamp :153 + store :154-155, verified).
- **W2 — ink scenario wording**: all three ink scenarios now have proper
  Given/When/Then structure ("When the header renders" / "When the handler
  stores the result"); scenario 2's Then steps are behavioral (no literal
  source strings): "the manual /compact write site stores a compactionReduction
  clamped to a minimum of 0", "the retry dialog write site stores…", "no write
  site stores the raw unclamped compressionRatio", "the SessionHeader never
  applies an absolute value to compactionReduction". Test @step comments
  updated to match EXACTLY (source-shape assertion approach kept per NAPI-010
  precedent, with an explanatory comment); tests re-run: 4/4 pass. Coverage
  re-linked to new spans (73-92 / 94-135 / 137-153) and to the shifted
  AgentView write sites (2766-2769 and 5611-5613).
- **Observation — AgentView.tsx:978 chunk-path write site**: added the
  CMPCT-040/CMPCT-039 upstream-clamped comment; `npm run build` passes.

### CMPCT-041 (WARN → fixed; W1 skipped per instruction)

- **W1**: SKIPPED — already filed as CMPCT-042 by the supervisor;
  gemini_continuation.rs token-state logic untouched.
- **W2 — unprotected gemini_continuation.rs raw seeds**: BOTH done — (a)
  invariant comments added at both display-basis re-seed sites stating they
  MUST NOT be fed tracker cache-inclusive totals; (b) the wiring guard in
  cmpct041_seed_cache_double_count_test.rs extended to scan
  gemini_continuation.rs: exactly 2 `StreamingTokenDisplay::new` calls, each
  sourcing from a display-basis snapshot (`current_display.*` / `cont_final.*`,
  whitespace-normalised match), and never from `session.token_tracker`.
- **W3 — test-span drift on all 4 token-accounting scenarios**: re-linked to
  verified spans 82-103 / 183-213 / 219-245 / 251-291 (spans re-measured after
  the W5 guard rework shifted the file).
- **W4 — missing flush impl mapping**: added
  recovery_compaction.rs 165-203 (verified: fn :165, closing brace :203 —
  reviewer's 165-204 was off by one) to the flush scenario.
- **W5 — semantically-recycled @step comments on the two wiring guards**:
  removed ALL @step comments from
  `stream_loop_seed_sites_route_through_audited_constructor` (cli) and
  `all_four_pre_compaction_writers_route_through_shared_accessors`
  (agent-loop); both now carry an explicit "SUPPLEMENTARY STRUCTURAL GUARD —
  NOT a Gherkin scenario test" doc comment naming the behavioral test that
  carries the scenario's @step coverage. Scenario coverage re-linked to the
  behavioral tests only; link-coverage passes.
- **W6 — test-support gating documentation**: architecture note added to
  CMPCT-041 and a "Test gating:" paragraph appended to the
  `pre-compaction-snapshot-basis-unification.feature` doc string: behavioral
  parity tests require `--features test-support`; a default run executes only
  the structural guard.

## Final Verification

- [x] `cargo test -p codelet-sessions` — all green (44 binaries, 0 failures;
      includes rpc418_compact_session 4/4)
- [x] `cargo test -p codelet-fspec-tui --no-fail-fast` — 234 binaries ok;
      only the known pre-existing 300-LoC/source-shape failures (12 test fns
      across 10 binaries, all pinning scrollback.rs 307 / agent.rs 300 /
      provider_settings/mod.rs 301 — other workers' unstaged files). Zero delta.
- [x] napi compaction suites (`--features __full_runtime`):
      cmpct038_measurement_basis_test 7/7, cmpct039_ratio_clamp_test 6/6,
      rpc421_honest_ack_test 3/3
- [x] `cargo test -p codelet-cli --test cmpct041_seed_cache_double_count_test`
      — 5/5 (incl. extended gemini guard)
- [x] `cargo test -p codelet-agent-loop --test cmpct041_pre_compaction_basis_test`
      — default 1/1 (guard only), `--features test-support` 3/3
- [x] `npx vitest run …/compaction-badge-sign-integrity.test.tsx` — 4/4
- [x] `npm run build` — passes (Rust release + vite bundle)
- [x] `cargo fmt -p codelet-cli -p codelet-agent-loop --check` — clean
- [x] `cargo clippy -p codelet-cli -p codelet-agent-loop --tests` — 0 warnings
- [x] `fspec validate` — all 1592 feature files valid
- [x] `fspec validate-tags` — exactly the 482 pre-existing violations
      (baseline unchanged; all six batch features clean)
- [x] `show-coverage` 100% on all six features:
      single-sourced-compaction-notice 4/4, honest-compaction-acknowledgement
      3/3, compression-ratio-clamping 6/6, compaction-badge-sign-integrity 3/3,
      compaction-badge-sign-integrity-ink 3/3, token-accounting-cache-integrity
      4/4, pre-compaction-snapshot-basis-unification 2/2 (plus amended siblings
      slash-command-compact 8/8 and rust-tui-compact-real-compaction 4/4)
- [x] `audit-coverage` clean on all nine features above
- [x] All four work units (RPC-421, CMPCT-039, CMPCT-040, CMPCT-041) back in
      done; all six batch feature files carry @done and no @wip
