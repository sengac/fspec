# PROV-145 — Per-profile Loop-Detection Configuration: Research & Scope

**Date:** 2026-09-04
**Goal:** Add new fields to the /provider OpenAI profile create/edit form so the
RIG-014 streaming LLM loop detector (commit `e66f2cb1`, 2026-08-20) can be
tuned per profile: sliding-window size, repeat-count threshold, retry budget
after aborts, escalation cooldown, and the remaining detector thresholds.

---

## 1. Background — the recently added loop detection (RIG-014/015, PROV-142)

Commit `e66f2cb1` "feat(agent-loop): add streaming LLM loop detector and
per-profile auto-continue (RIG-014/015, PROV-142)" shipped:

- **`rust/agent-loop/src/stream_loop_detector.rs`** (414 lines) — a pure,
  synchronous, word-level streaming detector. Two independent instances per
  session: one for **thinking** deltas, one for **text** deltas (a loop in one
  channel must not be masked by fresh content in the other). Fed one delta at
  a time; latches on first fire; reset once per turn.
- **`rust/agent-loop/src/background_output.rs`** — `BackgroundOutput` (the
  `StreamOutput` sink) feeds every `StreamEvent::Text` / `StreamEvent::Thinking`
  delta into the channel detectors and applies the warn→abort escalation.
- **`rust/agent-loop/src/agent_loop.rs`** — on abort: the in-flight provider
  stream is cancelled (`session.interrupt()`), the degenerate tail is dropped,
  a marker note is appended to the persisted assistant message, and a
  corrective note is staged on the session and injected as a User message at
  the top of the next turn. The turn is then **auto-continued** with a
  synthetic "Continue" input, bounded by a retry cap; the counter resets on
  genuine user input.
- **RIG-015** — behavioral test (test-support stub provider with a looping
  stream) proving the abort stops the stream mid-flight and re-prompts.
- Feature files: `spec/features/streaming-loop-detection.feature`,
  `spec/features/streaming-loop-abort-behavioral-test.feature`.
- Research + POC evidence: `spec/attachments/RIG-014/` (research report, POC
  summary + source, two arXiv PDFs).

All thresholds today are **hard-coded constants**:
`LoopDetectorConfig::default()` and `RIG014_LOOP_ABORT_COOLDOWN_SECS` +
`RIG014_MAX_LOOP_ABORT_RETRIES`. The detector was explicitly designed to be
configurable ("Per-model tuning is possible without code changes") but nothing
exposes that yet. **This card adds the per-profile configuration path.**

---

## 2. All configurable aspects of the loop detection

### 2.1 Detector thresholds — `LoopDetectorConfig` (stream_loop_detector.rs:72-122)

| # | Field | Type | Default | Meaning |
|---|---|---|---|---|
| 1 | `window` | `usize` | 160 | Bounded sliding window size **in words** (the user's "window size") |
| 2 | `ngram_sizes` | `Vec<usize>` | `[3, 5, 8]` | Tail n-gram sizes to check |
| 3 | `max_repeats` | `usize` | 10 | Tail n-gram must appear ≥ this many times in the window (the user's "how many times it must repeat") |
| 4 | `min_unique_ratio` | `f64` | 0.15 | Diversity collapse fires when unique-word ratio falls below this |
| 5 | `diversity_min_window` | `usize` | 40 | Diversity signal only evaluated once window holds ≥ this many words |
| 6 | `min_long_match` | `usize` | 16 | Long verbatim suffix length (words) |
| 7 | `min_long_match_repeats` | `usize` | 3 | ⚠️ **Dead field** — stored in config but never read by `check()` (see §7, item 1) |
| 8 | `min_words_before_check` | `usize` | 12 | Minimum-evidence guard: no signal evaluated before this many words |
| 9 | `period_len` | `usize` | 24 | Periodicity window length (last P words vs the P before them) |
| 10 | `period_min_matches` | `f64` | 0.85 | Periodicity fires when word-pair match ratio ≥ this |

Defaults are POC-validated (research report §5.4: detection latency 15–35
words after loop onset, zero false positives on 7 synthetic generators).

### 2.2 Escalation policy — `LoopEscalationPolicy` (stream_loop_detector.rs:271-316)

- **`cooldown`** — the ONLY policy parameter. Created with
  `Duration::from_secs(RIG014_LOOP_ABORT_COOLDOWN_SECS)` where
  `RIG014_LOOP_ABORT_COOLDOWN_SECS: u64 = 30`
  (`background_output.rs:41`).
- Policy semantics (fixed, not parameterized): first trigger → **Warn**
  (streaming continues, `tracing::warn` only — note: no user-visible status
  chunk is emitted today, §7 item 4); a re-trigger **within the cooldown**,
  or a **second distinct signal type** (regardless of timing) → **Abort**.
  A re-trigger after the cooldown warns again (new episode).

### 2.3 Abort-retry budget — the user's "how many times it can be interrupted before it won't retry anymore"

- `const RIG014_MAX_LOOP_ABORT_RETRIES: usize = 10;` +
  `let mut loop_abort_retry_count: usize = 0;`
  (`agent_loop.rs:88-91`, `codelet-agent-loop` only).
- After each loop abort the agent loop auto-continues with a synthetic
  `"Continue"` input (reusing the compaction-watchdog retry-input mechanism,
  CMPCT-020). After 10 consecutive aborts without genuine user input it gives
  up, logs a warning, and discards the staged corrective note.
- The counter **resets on real user input** (`agent_loop.rs:261-264`), so the
  cap is "per user turn", not per session.
- **Drift risk:** this block exists ONLY in `codelet-agent-loop`. The NAPI
  twin `rust/napi/src/agent_loop.rs` has a separate `BackgroundOutput`
  (line 1471) with **no loop detectors at all** (no RIG-014 references
  anywhere in `rust/napi/src`). The NAPI binary does not run the detector
  today — see §7 item 2.

### 2.4 Per-turn reset semantics (fixed, not configurable)

- Detectors + `loop_abort_fired` flag reset at the start of each turn
  (`reset_turn_loop_detectors()`, `background_output.rs:98`), so window state
  never carries across turns.
- Within a turn the detector **latches**: once a signal fires, subsequent
  `feed` calls keep returning it until reset.
- The abort path appends `build_loop_abort_marker_note()` to persisted
  content and stages `build_loop_abort_recovery_message(...)` on the
  `BackgroundSession` (`set_pending_loop_abort_note`,
  `background_session.rs:1649-1671`).

### 2.5 Channels covered

Only streamed **text** and **thinking** deltas. Tool-call argument text is
deliberately out of scope (RIG-014 research §6 Q2 — separate
tool-call-loop story).

---

## 3. The /provider OpenAI profile view (target of the new fields)

### 3.1 Form state — `rust/fspec-tui/src/views/provider_settings/`

| File | Role |
|---|---|
| `profile_form.rs` (267 lines) | `PROFILE_FORM_FIELDS: [&str; 9]` (Base URL, API Key, Context Window, Max Output Tokens, Compaction Threshold, Streaming, Auto-Continue, Preserve Thinking, Max Images); `ProfileForm` struct with one raw string per text field + `streaming`/`preserve_thinking` bools; `new_create()` / `from_definition()` seeding; `focused_text_mut()` index routing (0-4, 6, 8 = text; 5, 7 = toggles); `field_value()`; `build_definition()` (returns `Err(hint)` on invalid numerics → save rejected, form stays open) |
| `profile_form_parse.rs` (78 lines) | Pure parse/format helpers: `opt_num`, `parse_auto_continue`, `parse_max_images` (empty ⇒ `None`, "0" ⇒ sentinel, non-numeric ⇒ `Err` with hint) |
| `profile_form_render.rs` (126 lines) | `placeholder_for(idx)` dim hints; `field_line`; render loop iterates `PROFILE_FORM_FIELDS` |
| `profile_form_streaming.rs` (64 lines) | Boolean toggle routing: `is_streaming_field(idx)` (5 and 7), Space/Left/Right flip, everything else swallowed |
| `profile_form_submit.rs` (100 lines) | Enter → `build_definition()` → `Action::SaveProfile`; `Err(hint)` → `view.set_status(hint)` + form stays open |
| `profile_form_paste.rs` (44 lines) | Paste routing (printable-ASCII gate) |
| `profiles_config.rs` (273 lines) | `profile_definition_from_value()` — reads the stored JSON into `ProfileDefinition` for edit-form prefill (`autoContinue`/`maxImages` read as `as_u64 → u32`) |

Module ceiling: **300 lines per file** (workspace rule). `profile_form.rs`
is already at 267 — a 4-field addition (struct fields, two match arms,
parse calls, seeding ×2) adds ~35-45 lines → **refactor required first**
(e.g. extract the loop-detection sub-fields into a small helper struct or a
new `profile_form_loop_detection.rs` module, mirroring the
`profile_form_parse` split-out pattern).

### 3.2 Save dispatch

`rust/fspec-tui/src/app/dispatch_provider_settings_profiles.rs` →
`handle_save_profile` → `backend.save_profile(...)` — generic over
`ProfileDefinition`; **no changes needed** for new flat fields.

### 3.3 Recent tickets that touched this view (2 weeks)

| Ticket | Date | What it added | Relevance |
|---|---|---|---|
| **PROV-144** (`ea204a3a`, 2026-09-04) | newest | `maxImages` field (9th), placeholder "4 (default), 0 = no vision", `parse_max_images`, tool-layer budget wiring | **The template to copy** — flat `Option<u32>` profile field end-to-end |
| **PROV-143** (`f2c50a3e`, 2026-08-29) | | `preserveThinking` bool toggle (8th), toggle routing in `profile_form_streaming.rs` | Template for the boolean "enabled/disabled" sub-toggle |
| **PROV-142** (`e66f2cb1`, 2026-08-20) | | `autoContinue` field (7th) + session seeding in `session_creation_helper.rs` + **the loop detector itself** | Session-seeding template; detector under configuration |
| **PROV-139/140** | older | `streaming` toggle (6th) + `OPENAI_STREAMING` env bridge | Toggle + env-bridge precedent |
| **PROV-135/136/137/138** | older | placeholders, rename, paste, API-key masking | Form plumbing |
| **Mux work** (`5300a929` etc.) | 2026-08-31 | touched provider_settings only incidentally (nav dispatch tests) | — |

Feature files to read before implementing: `profile-auto-continue-form.feature`,
`profile-auto-continue-persistence.feature`,
`profile-auto-continue-session-seeding.feature`,
`profile-auto-continue-wire-schema.feature`,
`per-profile-max-images-*.feature` (5 files),
`profile-preserve-thinking-*.feature` (4 files),
`provider-profile-streaming-*.feature` (3 files),
`streaming-loop-detection.feature`,
`streaming-loop-abort-behavioral-test.feature`.

---

## 4. End-to-end flow of a new profile field (established pattern)

Every recent field (PROV-139 → 144) followed this exact path — **this card
follows the same path**:

```
TUI form (fspec-tui)
  profile_form.rs        field + seed + parse in build_definition
  profile_form_parse.rs  parse helper (empty ⇒ None; non-numeric ⇒ Err hint)
  profile_form_render.rs placeholder
  profiles_config.rs     prefill read (as_u64 → u32 / as_bool)
        │  Action::SaveProfile { definition }
        ▼
Wire (codelet-rpc-types)
  ProfileDefinition      flat Option<u32> / Option<bool> field
  (napi(object) — plain struct required; f64 fields exist elsewhere in the
   file (e.g. ContextFillInfo.effective_tokens), so float thresholds are OK)
        ▼
RPC surface (sessions handle_impl.rs:1478/1507 save_profile / rename_profile)
        │  profile_def_from_wire (conversions.rs)
        ▼
Disk (codelet-sessions)
  ProfileDef             (profile_persistence.rs)
  merge_profile          set_or_remove("camelCaseKey", ...) — None removes key
  fspec-config.json      providers.openai.profiles.<name> (openai-only guard)
        │  read back via
        ▼
Read paths
  LocalServerProfile     (profile_sections.rs, serde camelCase +
                         de_opt_u32_lenient for numeric fields)
  profiles_config.rs     (TUI prefill)
        │  at session creation / model switch
        ▼
Runtime
  session_creation_helper.rs seeds BackgroundSession fields
  (auto_continue precedent: lines 259-288)
```

Key facts:

- **On-disk key naming** is camelCase: `autoContinue`, `preserveThinking`,
  `maxImages` → loop fields would be e.g. `loopDetectionWindow`,
  `loopDetectionMaxRepeats`, ...
- **`None` removes the key** on save (read-modify-write); absent keys mean
  "use defaults" everywhere (backward compatible with existing profiles — no
  migration needed).
- `apply_profile_env_vars` (model_resolution.rs) bridges
  `baseUrl/apiKey/contextWindow/streaming` into `OPENAI_*` env vars —
  irrelevant here: the detector is client-side, so **no env bridge needed**.

---

## 5. Integration points for the runtime wiring

### 5.1 Detector construction (per turn)

`BackgroundOutput::with_provider(session, provider)` —
`background_output.rs:78-92` — constructs:

```rust
StreamLoopDetector::new()          // hardcoded LoopDetectorConfig::default()
StreamLoopDetector::new()
LoopEscalationPolicy::new(Duration::from_secs(30))  // hardcoded
```

Call site: `agent_loop.rs:520-522` (`codelet-agent-loop`), **per turn**, with
`session: Arc<BackgroundSession>` in scope. `BackgroundSession` holds
`provider_id: RwLock<Option<String>>` and `model_id: RwLock<Option<String>>`
(`background_session.rs:277-279`), so the profile can be re-resolved at this
point and **mid-session model switches are picked up automatically** (the
detector is rebuilt every turn).

### 5.2 Where the profile lookup happens

`codelet-sessions` already re-reads `fspec-config.json` per access:
- `crate::profile_sections::load_local_server_profiles()` (used by
  `resolve_profile_max_images`, `apply_profile_env_vars`, and the PROV-142/143
  seeds in `session_creation_helper.rs`).
- Precedent: `resolve_profile_max_images(pm)` (model_resolution.rs:83) parses
  the composite `openai:<profile>/<model>` string, looks up the profile, and
  returns the field.

**Recommendation:** add `resolve_profile_loop_detection(session) ->
Option<LoopDetectorConfig-shaped profile values>` to `codelet-sessions`
(same file/module family as `resolve_profile_max_images`). Non-profile
sessions → `None` → defaults (current behavior).

⚠️ **Crate direction check:** `LoopDetectorConfig` lives in
`codelet-agent-loop`, which already depends on `codelet-sessions`
(agent-loop/Cargo.toml) — so `codelet-sessions` **cannot** return that type
without a cycle. Two clean options:
1. (Preferred) Return flat `Option<u32>`/`Option<f64>` values from
   `codelet-sessions`; `codelet-agent-loop` assembles the
   `LoopDetectorConfig` at `with_provider`.
2. Move `LoopDetectorConfig` to `codelet-rpc-types` (it's already a plain
   serde-friendly struct; `rpc-types` is depended on by everyone) and fill it
   in sessions. Option 1 keeps the change smaller.

### 5.3 Retry-cap storage

`loop_abort_retry_count` + `RIG014_MAX_LOOP_ABORT_RETRIES` live in the
`agent_loop` function body (loop-local, resets only in-memory per turn). To
make the cap configurable, store it on `BackgroundSession` (like
`continue_budget`, an atomic or a plain field set by
`session_creation_helper.rs`) and read it in the retry block. Seeding
happens in `create_background_session_inner` alongside the PROV-142/143
seeds (lines 195-288).

### 5.4 Mid-session model switch

No extra work needed if the config is resolved per turn (§5.1): switching from
`openai:loose/model` to `openai:strict/model` takes effect on the next turn's
detector. (Contrast: `max_images` is tool-layer registry state refreshed at
all 4 set-sites — not needed here because the detector rebuilds each turn.)

---

## 6. Recommended form design

### 6.1 Field set — options

**Option A (recommended core): enable toggle + 3 numeric fields** — matches
the user's explicit asks exactly and keeps the form compact:

| Position | Label | Kind | Empty means | Placeholder |
|---|---|---|---|---|
| 10 | "Loop Detection" | bool toggle (PROV-139/143 pattern: Space/Left/Right) | `true` (on — detector is on today for ALL sessions, so absent must mean ON to preserve behavior) | "Enabled" |
| 11 | "Loop Window" | u32 text | default 160 (words) | "160 (default, words)" |
| 12 | "Loop Repeat" | u32 text | default 10 | "10 (default)" |
| 13 | "Loop Retries" | u32 text | default 10 | "10 (default)" |

- `0` sentinels: "Loop Window" 0 / "Loop Repeat" 0 → invalid (reject save,
  mirror `parse_auto_continue` hint style). "Loop Retries" 0 → explicit
  "abort once, never auto-retry" (reasonable, keep as valid).
- Toggle off ⇒ the numeric fields are ignored (detectors not constructed —
  cheap: `feed_loop_detectors` early-returns, or `with_provider` is handed a
  disabled flag). **This also gives a clean per-profile OFF switch that does
  not exist today.**

**Option B (full exposure): all 10 detector thresholds + cooldown** — 14 new
lines in the form (the form currently has 9; at ~13 rows total it still
renders, but the 300-line ceiling in `profile_form.rs` forces a larger
refactor). Could be staged as a follow-up card after A proves the pattern.

**Recommendation:** ship **Option A** on this card; add a second card
("advanced loop-detection thresholds") for the remaining 6 parameters
(`ngram_sizes`, `min_unique_ratio`, `diversity_min_window`,
`min_long_match`, `min_words_before_check`, `period_len`,
`period_min_matches`, cooldown) behind the same flat-fields pattern. The wire
schema (`ProfileDefinition`) is trivially extensible, so Option B costs
little later.

### 6.2 Open questions (red cards)

1. **Toggle default semantics.** Detector is ON today for every session.
   Absent `loopDetectionEnabled` must mean ON (like `streaming`), not OFF.
   Confirm the "0/off sentinel" style used for `autoContinue` is NOT
   wanted here (i.e. the toggle is a plain bool, not a numeric budget field).
2. **Retry cap semantics.** Keep per-user-turn reset (today's behavior) and
   only change the cap value, or make it a per-session total? (Per-turn is
   the natural reading of "interrupted N times then stop"; recommend
   per-turn.)
3. **NAPI twin.** The NAPI `BackgroundOutput` has no detectors at all. Do we
   (a) out of scope — document the drift and add a follow-up card
   "port RIG-014 to the napi twin", or (b) include a minimal port?
   (Recommend (a); the `fspec` Rust binary — `FspecAgentHooks` — is the
   production loop.)
4. **Warn visibility.** The Warn escalation outcome only logs
   (`tracing::warn`) — no TUI status chunk is emitted. Should a profile
   option (or this card, as a small side quest) emit a `StreamChunk` status
   on warn? (Recommend: out of scope; separate card if wanted.)
5. **Float thresholds in the form.** `min_unique_ratio` /
   `period_min_matches` are f64. The form's printable-ASCII gate accepts
   `.`; `parse::<f64>()` with range validation (0.0 < x ≤ 1.0) works.
   Confine to Option B.
6. **ngram_sizes.** The form would need comma-separated-list parsing
   (e.g. "3,5,8") — new parse helper. Confine to Option B.

---

## 7. Known defects / risks found during research

1. **`min_long_match_repeats` is dead code** — declared in
   `LoopDetectorConfig` (default 3, documented as "fires on the 6th copy")
   but `StreamLoopDetector::check()` (signal 3) fires on the FIRST verbatim
   earlier match. If Option B exposes it, the signal must be fixed first
   (or the field documented as reserved). Small fix, good to bundle.
2. **NAPI twin drift** — `rust/napi/src/agent_loop.rs` `BackgroundOutput`
   (line 1471) predates RIG-014 and has no loop detection, no abort-retry
   loop, no corrective-note injection. Any profile-based config would be
   silently ignored by the NAPI binary. (See §6 Q3.)
3. **`rust/napi/index.d.ts` is stale** — `ProfileDefinition` (line 3457)
   still ends at `autoContinue?` (PROV-142, Aug 20); it is missing
   `preserveThinking` (PROV-143, Aug 29) and `maxImages` (PROV-144, Sep 4).
   The NAPI d.ts generation step did not run for the last two fields. This
   card should include "regenerate/verify index.d.ts" as a validation step.
4. **`profile_form.rs` is 267 lines** — near the 300-line ceiling. A
   4-field addition needs the extract-helper treatment (precedent:
   `profile_form_parse.rs`, `profile_form_streaming.rs`,
   `profile_form_submit.rs`, `profile_form_paste.rs` were all split out for
   exactly this reason).
5. **Window-size interaction:** `window < 2*min_long_match` or
   `window < 2*period_len` disables signals 3/4 implicitly (they guard on
   `window.len() >= 2*m`). Small window values + default thresholds = fewer
   signals, not an error. The form hint should say "words".
6. **Performance:** signal 1 is O(n·window) per word; at window=160 and
   n≤8 that's ≈ 1300 comparisons/word worst case — negligible, but a huge
   user-typed window (e.g. 10000) grows linearly. Cap the form value
   (e.g. 16..=2000) with a rejection hint, mirroring the compaction-threshold
   range-check precedent (`profile_form_parse.rs:31-40`).

---

## 8. Scope of changes (Option A)

### 8.1 Wire — `rust/rpc-types/src/lib.rs`
Add to `ProfileDefinition` (all flat, `napi(object)`-safe):
```rust
/// PROV-145: per-profile loop-detection config. None (absent) ⇒
/// LoopDetectorConfig::default() + detector ON (today's behavior).
pub loop_detection_enabled: Option<bool>,   // absent ⇒ true (ON)
pub loop_detection_window: Option<u32>,    // words; default 160
pub loop_detection_max_repeats: Option<u32>, // default 10
pub loop_detection_max_retries: Option<u32>, // default 10
```
+ canonical predicates (`loop_detection_enabled() -> bool` =
`unwrap_or(true)`, and `loop_detection_config()` returning a
`LoopDetectorConfig`-shaped struct or flat values, per §5.2).

### 8.2 Persistence — `rust/sessions/`
- `profile_persistence.rs::ProfileDef` — same 4 fields; `merge_profile`
  `set_or_remove` for `loopDetectionEnabled` / `loopDetectionWindow` /
  `loopDetectionMaxRepeats` / `loopDetectionMaxRetries` (None removes key).
- `conversions.rs::profile_def_from_wire` — copy through.
- `profile_sections.rs::LocalServerProfile` — serde camelCase fields
  (`de_opt_u32_lenient` for the u32s, like `autoContinue`); bool uses the
  plain `as_bool` path.
- New resolver in `model_resolution.rs`:
  `resolve_profile_loop_detection(&BackgroundSession|&ProviderManager) ->
  (Option<bool>, Option<u32>, Option<u32>, Option<u32>)` (flat, per §5.2).

### 8.3 Runtime — `rust/agent-loop/`
- `background_output.rs`: `with_provider` takes an extra
  `loop_config: Option<(LoopDetectorConfig, Duration)>` (or a small
  `LoopDetectionWiring` struct); constructs
  `StreamLoopDetector::with_config(...)` /
  `LoopEscalationPolicy::new(cooldown)`; when disabled,
  `feed_loop_detectors` is a no-op (single early-return flag).
- `agent_loop.rs`: resolve the profile values before
  `BackgroundOutput::with_provider` (session's `provider_id` + `model_id`
  locks → `resolve_profile_loop_detection`); read the retry cap from
  `BackgroundSession` instead of the `const`.
- `rust/sessions/src/background_session.rs`: new field for the retry cap
  (atomic or plain) + setter; `session_creation_helper.rs` seeds it alongside
  the PROV-142/143 seeds.

### 8.4 TUI — `rust/fspec-tui/src/views/provider_settings/`
- `profile_form.rs` — 4 new fields + match arms (after refactor, §7 item 4).
- `profile_form_parse.rs` — `parse_loop_detection_window` (16..=2000),
  `parse_loop_detection_max_repeats` (1..=1000),
  `parse_loop_detection_max_retries` (0..=1000); empty ⇒ `None`.
- `profile_form_streaming.rs` — add index 9 (toggle) to
  `is_streaming_field` + `toggle_on_key`.
- `profile_form_render.rs` — placeholders for indices 9-12.
- `profiles_config.rs::profile_definition_from_value` — read the 4 keys.
- No changes: submit dispatch, save dispatch, paste (text fields are
  automatic; toggle is swallowed by the existing `is_streaming_field` guard).

### 8.5 Spec / tests (ACDD)
- Feature files (capability-named, mirroring PROV-144's 5-file set):
  - `per-profile-loop-detection-form.feature`
  - `per-profile-loop-detection-persistence.feature`
  - `per-profile-loop-detection-wire-schema.feature`
  - `per-profile-loop-detection-session-wiring.feature`
- Test files (mirror existing names):
  - `rust/rpc-types/tests/prov145_loop_detection_flag.rs`
  - `rust/sessions/tests/prov145_loop_detection_persistence.rs`
  - `rust/sessions/tests/prov145_loop_detection_seed.rs` (seed test follows
    `prov142_session_seed.rs` — temp `FSPEC_USER_DIR` + `FSPEC_HOME` +
    `CODEX_HOME` isolation, offline `models.json` fixture)
  - `rust/agent-loop/tests/rig014_profile_config_applied.rs` (detector
    respects profile thresholds; `test-support` feature, StubModel loop
    hook from RIG-015)
  - `rust/fspec-tui/tests/prov145_loop_detection_form.rs` (pure-state form
    tests following `prov142_auto_continue_form.rs`)
- **Validation step:** regenerate/verify `rust/napi/index.d.ts` (currently
  stale — §7 item 3).

---

## 9. Effort estimate

| Chunk | LoC (approx) |
|---|---|
| Wire + predicates (rpc-types) | 40 |
| Persistence + conversion + LocalServerProfile (sessions) | 60 |
| Resolver (sessions/model_resolution) | 40 |
| Runtime wiring (agent-loop + background_session + creation helper) | 80 |
| TUI form + parse + render + prefill | 90 |
| Feature files (4 × ~50) | 200 |
| Tests (5 files) | 450 |
| **Total** | **~960 LoC** |

**Estimate: 8 story points** (2-4 h). Splits cleanly if the team prefers two
cards: (1) wire/persistence/runtime (~5), (2) TUI form (~3) — but one card is
simpler since the field set is small (Option A).

---

## 10. Out of scope (explicit)

- Advanced thresholds (Option B fields) — follow-up card, §6.
- NAPI-twin port of the detector — §6 Q3 / §7 item 2.
- Warn-visibility status chunks — §6 Q4.
- Tool-call-loop detection (vtcode-style normalized-args hashing) — separate
  RIG-014 research item.
- Per-model (customModels[]) loop thresholds — the profile level is the
  supported granularity (matches streaming/autoContinue/maxImages).
- Cooldown tuning — stays in Option B (needs a seconds unit on the label).
