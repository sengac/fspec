# AST Research — PROV-143: Preserve Thinking toggle wiring

Scope: the code paths the new `preserve_thinking` flag must flow through.

## Entry points & wiring (verified via AST search + Read)

| Concern | Location |
|---|---|
| Wire field | `ProfileDefinition.preserve_thinking: Option<bool>` — `rust/rpc-types/src/lib.rs` (+ `preserve_thinking_enabled()` predicate) |
| On-disk field | `ProfileDef.preserve_thinking: Option<bool>` — `rust/sessions/src/profile_persistence.rs` (`merge_profile` writes `preserveThinking`) |
| Profile load | `LocalServerProfile.preserve_thinking` — `rust/sessions/src/profile_sections.rs` |
| Wire→disk bridge | `profile_def_from_wire` — `rust/sessions/src/conversions.rs` |
| TUI form field | `ProfileForm.preserve_thinking` + `PROFILE_FORM_FIELDS[7]` — `rust/fspec-tui/src/views/provider_settings/profile_form.rs` |
| TUI toggle routing | `PRESERVE_THINKING_FIELD_INDEX` + `toggle_on_key` — `rust/fspec-tui/src/views/provider_settings/profile_form_streaming.rs` |
| Config loader | `profile_definition_from_value` reads `preserveThinking` — `rust/fspec-tui/src/views/provider_settings/profiles_config.rs` |
| Session seeding | `create_background_session_inner` sets `Session.preserve_thinking_enabled` — `rust/sessions/src/session_creation_helper.rs` |
| Session flag | `Session.preserve_thinking_enabled` — `rust/cli/src/session/mod.rs` |
| Choke point | `RigAgent::with_preserve_thinking` / `outgoing_history` — `rust/core/src/rig_agent.rs` |
| Strip fn | `strip_reasoning_from_history` — `rust/core/src/history_strip.rs` |
| Call sites | `rust/agent-loop/src/dispatch.rs`, `rust/agent-loop/src/agent_loop.rs` |

## Key invariant
`strip_reasoning_from_history(&history, preserve)` is a pure function over the clone;
the live session history is never mutated. `preserve_thinking_enabled()` is the single
source of truth for "absent ⇒ stripped".

