# RPC-346 — AST research: backend custom-model persistence

Confirms the change surface in `codelet/sessions/src/profile_sections.rs`
and the TS behaviour being ported.

## Current Rust state (read-only)
- `CustomModelDef` (profile_sections.rs:86-89): only `pub id: String`,
  derives `Debug, Clone, Deserialize` — NO `Serialize`, NO other fields.
- `LocalServerProfile` (profile_sections.rs:68-83): has `custom_models:
  Vec<CustomModelDef>` (serde rename "customModels", default).
- `load_local_server_profiles` (profile_sections.rs:105-134): read-only;
  reads `~/.fspec/fspec-config.json` → providers.openai.profiles.<name>.
  Uses `fspec_user_dir()` (FSPEC_USER_DIR env or HOME/.fspec) (:92-96).
- NO writer/save function anywhere in the sessions crate (verified: only a
  TS comment at :104 references `saveProfile`).

## TS reference being ported
- `src/tui/services/customModelCrudService.ts`:
  - `saveCustomModel(providerId, profileName, definition, originalModelId?)`
    (:30-68): getProfile → no-op+warn if missing; `originalModelId` present =>
    replace entry with that id (EDIT), else append (ADD); saveProfile.
  - `deleteCustomModel(providerId, profileName, modelId)` (:77-107): filter out
    id; if remaining length 0 => set customModels = undefined (omit key); save.
- `src/utils/profile-management.ts` `saveProfile` (:39-89): guard providerId ===
  'openai' (else throw); read whole fspec-config.json, ensure
  providers.openai.profiles objects, set profiles[name] = config, write WHOLE
  config back (preserves unrelated keys).
- `CustomModelDefinition` (provider-config.ts:95-112) fields (camelCase JSON):
  id (required), displayName?, facade? ('openai'|'codex'|'claude'|'gemini'|'zai'),
  contextWindow?, maxOutputTokens?, compactionThreshold? ({type:'tokens'|
  'percentage', value:number}), reasoning?, hasVision?.

## Change surface for RPC-346 (this card)
1. Extend `CustomModelDef` → full definition matching CustomModelDefinition,
   add `Serialize` (keep `Deserialize`), serde camelCase rename + skip-None so
   optional fields are omitted. id stays required.
2. Add `save_custom_model(provider_id, profile_name, def, original_model_id:
   Option<&str>)` and `delete_custom_model(provider_id, profile_name, model_id)`
   — sync std::fs read-modify-write of the WHOLE config Value (preserve unrelated
   keys), mirroring saveProfile/deleteCustomModel. Guard to "openai". Missing
   profile = no-op. Empty array after delete => omit the customModels key.
3. Reuse existing `fspec_user_dir()` for path resolution (offline-testable via
   FSPEC_USER_DIR).

Sync (not async) chosen to match the existing read path
(`load_local_server_profiles` uses std::fs); the async RPC bridge is RPC-347.

## Out of scope (later cards)
- RPC trait / NAPI / Action variants / wire type → RPC-347.
- UI keybinds / form / confirm views → RPC-344.
</parameter>
</invoke>
