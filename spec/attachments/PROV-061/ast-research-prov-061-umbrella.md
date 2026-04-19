# AST Research — PROV-061 Rhai-Scriptable Custom Provider Type (Epic Umbrella)

This is an **umbrella epic** whose implementation is entirely delivered by six child work units.
The parent carries no source code of its own; all production code lives under child work units.
This document consolidates AST research already performed per child, so that the parent
satisfies the "AST research performed during discovery" gate required to close the epic.

## Child Work Units (All DONE)

| Child | Title | AST Research Attachment |
|---|---|---|
| PROV-062 | Provider config loader and Rhai script compiler | `spec/attachments/PROV-062/ast-research-prov-062.md` |
| PROV-063 | Custom provider HTTP request/response lifecycle | `spec/attachments/PROV-063/ast-research-prov-063.md` |
| PROV-064 | Custom provider streaming SSE bridge | `spec/attachments/PROV-064/ast-research-prov-064.md` |
| PROV-065 | Custom provider Rhai-scriptable system prompts | `spec/attachments/PROV-065/ast-research-prov-065.md` |
| PROV-066 | Custom provider Rhai-scriptable tool facades | `spec/attachments/PROV-066/ast-research-prov-066.md` |
| PROV-067 | Custom provider ProviderManager integration | `spec/attachments/PROV-067/ast-research-provider-type-callsites.md` |

## Consolidated AST Touchpoints

### Provider type surface (101 ProviderType call sites — see PROV-067 research)
- `codelet/providers/src/manager.rs` — `ProviderType` enum gains `Custom(String)` variant; `Copy` derive removed; `as_str(&self) -> &str` borrows from inner String
- `ProviderType::FromStr` — falls through to custom provider registry before erroring
- `map_provider_id_to_type()` — recognises custom provider slug and returns `ProviderType::Custom(slug)`
- `ProviderCredentials` — gains `custom_available: HashMap<String, bool>` + `has_custom(&self, name: &str) -> bool`
- `provider_limits_resolver()` — adds `ProviderType::Custom(_)` arm
- `detect_default_provider()` — unchanged; custom providers never auto-select

### New module tree `codelet/providers/src/custom/`
- `config.rs` — `ProviderConfig`, `AuthConfig`, `ModelDef`, `Defaults`, `SystemPromptConfig` (PROV-062)
- `discovery.rs` — `discover_provider_configs()` scanning `~/.fspec/providers/` + `.fspec/providers/` (PROV-062)
- `script_loader.rs` — `ScriptLoader` caching `Arc<rhai::AST>` by path+mtime (PROV-062)
- `provider.rs`, `request_bridge.rs`, `response_bridge.rs`, `http.rs`, `error_mapping.rs`, `rhai_call.rs` — HTTP lifecycle (PROV-063)
- `stream.rs`, `stream_convert.rs`, `stream_http.rs`, `provider_stream.rs` — SSE streaming (PROV-064)
- `system_prompt.rs` — `RhaiSystemPromptFacade` (PROV-065)
- `tool_facade.rs`, `tool_resolve.rs`, `tool_presets.rs` — `RhaiToolFacadeAdapter` + presets (PROV-066)
- `custom_provider.rs`, `management.rs`, `mod.rs` — `CustomProvider::create_rig_agent`, `list/show/validate/test/init` commands (PROV-067)

### NAPI surface (PROV-067)
- `codelet/napi/src/session_manager.rs` — `list_providers`, `show_provider`, `validate_provider`, `test_provider`, `init_provider` NAPI bindings; `session_set_model_profile()` sets `OPENAI_BASE_URL` / `OPENAI_API_KEY` / `OPENAI_MODEL` for openai-facade custom providers

## Verification

- `cargo build --workspace` — clean
- `cargo test --workspace --exclude codelet-napi` — 2783 passed, 0 failed
- `cargo test -p codelet-napi --lib` — 142 passed, 0 failed
- All six feature files validate; 100% scenario coverage across the epic
- Review findings: `spec/attachments/PROV-061/review-findings.md` (no critical correctness defects; specification-alignment follow-ups tracked for future work)
