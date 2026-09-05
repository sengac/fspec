# AST Research — PROV-145 Per-profile Loop Detection Configuration

**Date:** 2026-09-04
**Method:** Direct code exploration (Read/Grep/AstGrep) of every integration point in
the research map (spec/attachments/PROV-145/PROV-145-research.md §4–§8),
superseding the earlier attachment with verified line-level detail.

---

## 1. Wire layer — `rust/rpc-types/src/lib.rs`

- `ProfileDefinition` struct: **line 476**, fields end at `max_images` (line 512,
  PROV-144). 4 new flat fields append after it.
- Canonical predicates `impl ProfileDefinition`: **lines 515–549**
  (`streaming_enabled`, `auto_continue_enabled`, `preserve_thinking_enabled`,
  `max_images_limit`). 4 new predicates append:
  `loop_detection_enabled() -> bool` = `unwrap_or(true)`;
  `loop_detection_window() -> u32` = `unwrap_or(160)`;
  `loop_detection_max_repeats() -> u32` = `unwrap_or(10)`;
  `loop_detection_max_retries() -> u32` = `unwrap_or(10)`.
- `napi(object)` derive is cfg-gated (`#[cfg_attr(feature = "napi", ...)]`,
  line 474) — flat `Option<bool>` / `Option<u32>` fields keep the projection a
  plain struct.

## 2. Persistence — `rust/sessions/`

- `ProfileDef` struct: `profile_persistence.rs` **line 35**, ends
  `max_images` (line 62). 4 new `Option<...>` fields append.
- `merge_profile()`: `profile_persistence.rs` **line 176** — 4 new
  `set_or_remove(profile, "loopDetectionEnabled" | "loopDetectionWindow" |
  "loopDetectionMaxRepeats" | "loopDetectionMaxRetries", ...)` calls
  (None removes the key; `Some(false)` for the toggle IS written).
- `profile_def_from_wire()`: `conversions.rs` **line 165** — copy the 4 wire
  fields through.
- `LocalServerProfile`: `profile_sections.rs` **line 83** — 4 new serde fields
  (camelCase renames; `de_opt_u32_lenient` for the u32s, plain bool for
  enabled; all `default`).
- `save_profile_at` / `rename_profile_at` need NO changes (they call
  `merge_profile`).

## 3. Resolver — `rust/sessions/src/model_resolution.rs`

Existing resolvers (AstGrep verified):
```
model_resolution.rs:23:  pub fn resolve_model_vision(pm: &ProviderManager) -> bool
model_resolution.rs:83:  pub fn resolve_profile_max_images(pm: &ProviderManager) -> Option<u32>
model_resolution.rs:118: pub fn apply_model_selection(pm, model) -> Result<ResolvedModelLimits, String>
model_resolution.rs:259: pub fn apply_profile_env_vars(provider, profile_name, model)
```
New: `resolve_profile_loop_detection(pm: &ProviderManager) ->
LoopDetectionProfile` (flat struct `{ enabled: Option<bool>, window: Option<u32>,
max_repeats: Option<u32>, max_retries: Option<u32> }` — all-None for
non-profile selections). CANNOT return `LoopDetectorConfig` (crate cycle:
agent-loop → sessions). `ProviderManager::selected_model_string()`
(`providers/src/manager.rs:689`) rebuilds the composite
`openai:<profile>/<model>` when a profile is recorded — same pattern as
`resolve_profile_max_images`.

## 4. Runtime — `rust/agent-loop/`

- `background_output.rs`:
  - `RIG014_LOOP_ABORT_COOLDOWN_SECS` const — **line 41** (stays; cooldown
    tuning is the follow-up card).
  - `BackgroundOutput` struct — **lines 55–75** (detector fields 63–74).
  - `with_provider(session, provider)` — **lines 78–92**: constructs
    `StreamLoopDetector::new()` ×2 + `LoopEscalationPolicy::new(30s)`
    hardcoded. Gains a `loop_config` parameter (e.g.
    `Option<LoopDetectionWiring>`); `None` ⇒ today's behavior (default
    config, enabled). When `enabled=false`, `feed_loop_detectors`
    early-returns (single flag).
  - `feed_loop_detectors` — **line 114** (early-return site).
- `agent_loop.rs`:
  - `loop_abort_retry_count` + `RIG014_MAX_LOOP_ABORT_RETRIES` consts —
    **lines 90–91**.
  - Reset on genuine user input — **lines 263–265**.
  - Per-turn provider/model read — **lines 346–356** (`inner.lock()`,
    `current_provider_name()`, `current_model_id()`); inner
    `provider_manager()` at `cli/src/session/mod.rs:194`.
  - `BackgroundOutput::with_provider(session_for_output, current_provider)`
    — **line 522**. Per-turn profile resolution goes here (before the
    with_provider call): read `session.inner`'s provider manager →
    `resolve_profile_loop_detection` → build the wiring. The retry cap is
    read from the same resolution (default 10) replacing the const.
- Call sites of `with_provider` (Grep-verified):
  - `rust/agent-loop/src/agent_loop.rs:522` (production — gets the new arg)
  - `rust/fspec-tui/tests/cont008_goal_back_sync_test.rs:306,370,409`
    (tests — updated to pass `None` or defaults)
  - `rust/napi/src/agent_loop.rs:613` — the NAPI twin has its OWN local
    `BackgroundOutput` struct (`napi/src/agent_loop.rs:1471`) with NO loop
    detectors; does NOT import `codelet_agent_loop` in that file (verified:
    zero `codelet_agent_loop` references). **Unaffected.**

## 5. Session layer — `rust/sessions/src/background_session.rs`

- `pending_loop_abort_note` field — **line 482**; accessors
  `set/take/has_pending_loop_abort_note` — **lines 1649–1671**.
- Per-turn retry cap: NOT a session field. Resolution happens per turn in
  `agent_loop.rs` (the cap comes from `resolve_profile_loop_detection` at the
  same site as the detector config). No `BackgroundSession` change needed —
  simpler than the research doc's §5.3 option (which proposed seeding
  `session_creation_helper.rs`); the per-turn resolution already covers
  mid-session switches. The loop-local counter in `agent_loop.rs` keeps its
  per-user-turn reset (line 263).

## 6. TUI — `rust/fspec-tui/src/views/provider_settings/`

- `profile_form.rs` (267 lines — 33 under the 300-line ceiling):
  - `PROFILE_FORM_FIELDS: [&str; 9]` — **line 36** → `[&str; 13]`
    (append "Loop Detection", "Loop Window", "Loop Repeat", "Loop Retries").
  - `ProfileForm` struct — **lines 51–80**: +1 bool
    (`loop_detection: bool`, default `true` in `new_create`), +3 raw strings.
  - `new_create()` — **lines 84–105**; `from_definition()` — **lines 111–134**
    (prefill: toggle via `def.loop_detection_enabled()`, numerics via
    `opt_num(def.loop_detection_window)`-style effective-value read —
    mirrors PROV-144's `max_images_limit()` prefill at line 132).
  - `focused_text_mut()` — **lines 139–153** (new text indices 11, 12, 13).
  - `field_value()` — **lines 204–217** (toggle idx 10 →
    `streaming_label(self.loop_detection)`; numerics 11–13 → raw strings).
  - `build_definition()` — **lines 227–257**: +3 parse calls (window:
    1..=2000, max_repeats: 1..=1000, max_retries: 0..=1000 — 0 is the
    explicit "never auto-retry" sentinel; empty ⇒ None; non-numeric ⇒
    `Err(hint)`) + the 4 fields in the returned `ProfileDefinition` (toggle
    written as explicit `Some(bool)`, PROV-143 pattern).
  - **Refactor required** (research §7 item 4): ~40 lines added would
    breach 300. Extract to a new sibling module
    `profile_form_loop_detection.rs` (parse helpers + seed/format helpers,
    mirroring `profile_form_parse.rs`), registered in
    `provider_settings/mod.rs` (line ~37–43 module block).
- `profile_form_streaming.rs` (64 lines):
  - `is_streaming_field` — **line 24** (+ index 10);
  - `toggle_on_key` — **lines 56–63** (+ arm for index 10).
- `profile_form_render.rs`:
  - `placeholder_for` — **lines 54–66**: +3 dim hints for idx 11–13
    ("160 (default, words)", "10 (default)", "10 (default)").
- `profiles_config.rs`:
  - `profile_definition_from_value` — **lines 130–184**: +4 reads
    (`as_bool` for enabled; `as_u64 → u32` for the numerics, maxImages
    pattern lines 168–171) + struct fields.
- Untouched: `profile_form_submit.rs` (submit/Err-hint path generic),
  `profile_form_paste.rs` (printable-ASCII gate generic),
  `app/dispatch_provider_settings_profiles.rs` (`handle_save_profile`
  generic over `ProfileDefinition`).

## 7. Test fixtures to mirror

- Form: `rust/fspec-tui/tests/prov144_max_images_form.rs` (pure-state,
  key-driven via `ProviderSettingsView`).
- Wire: `rust/rpc-types/tests/prov144_max_images_wire.rs`.
- Persistence: `rust/sessions/tests/prov144_max_images_persistence.rs`
  (temp `save_profile_at` + `load_local_server_profiles` with `FSPEC_USER_DIR`
  EnvGuard).
- Resolution: `rust/sessions/tests/prov144_max_images_resolution.rs`
  (`ProviderManager::with_provider_and_model` + `set_model_direct_with_profile`
  + `#[serial]`).
- Session seeding: `rust/sessions/tests/prov142_session_seed.rs`
  (full `SessionManager` + offline `models.json` fixture +
  `FSPEC_USER_DIR`/`CODEX_HOME`/`FSPEC_HOME` isolation +
  `reset_stores_for_tests()`).
- Runtime behavioral: `rust/agent-loop/tests/rig015_loop_abort_behavioral.rs`
  (test-support feature, `stub_provider` + `stub_model::set_looping_stream_hook`
  + `FspecAgentHooks` + chunks broadcast).

## 8. Defects / drift confirmed during exploration

1. `min_long_match_repeats` (`stream_loop_detector.rs:96`) is a dead field —
   `check()` signal 3 fires on the FIRST verbatim earlier match. Out of scope
   (follow-up card with Option B).
2. NAPI twin has no loop detectors at all (`napi/src/agent_loop.rs` local
   `BackgroundOutput`, struct at line 1471) — documented drift, follow-up
   card. NOT affected by this change.
3. `rust/napi/index.d.ts` (line 3457 `ProfileDefinition`) is stale (missing
   `preserveThinking` + `maxImages` from PROV-143/144) — pre-existing; verify
   in VALIDATING, fix only if the regeneration tooling is available locally.
4. `profile_form.rs` at 267 lines — needs the helper-extract treatment
   (item 6 above).
