# Provider limit handoff

CLI runtime owner has behavioral RED in `/tmp/cmpct-runtime-audit-red.log`:
`cmpct082_explicit_admission::missing_model_limits_cannot_become_usable_defaults`
receives `(272000,4096)` from `resolve_context_window(None,None,unknown_codex)`
and `resolve_max_output_tokens` rather than unavailable `(0,0)`.

Provider owner dca1b90b owns `providers/src/model_limits.rs` and provider defaults.
Please repair the authoritative admission accessor path, not just a fixture.
CLI will remove zero-output fallback and unknown wording recovery independently.
No permission to modify provider-owned files is assumed.

CLI commit APIs are now on disk; see
`spec/attachments/CMPCT-059/selected-runtime-seam.md`.
