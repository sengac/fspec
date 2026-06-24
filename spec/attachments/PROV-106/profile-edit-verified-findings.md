# PROV-106 — VERIFIED findings: "Enter on a profile shows a blank input area"

**Date:** 2026-06-19
**Source:** DeepSearch of `src/tui/` (TS reference) vs
`codelet/fspec-tui/src/views/provider_settings/` (Rust port), cross-checked by
reading the actual source. NOTHING here is fabricated — file/line refs included.

---

## 0. Myth-busting: the "fireworks.ai" row is NOT hardcoded/fake

The `fireworks → https://api.fireworks.ai/inference` row the user sees is **real
data read from their own `~/.fspec/fspec-config.json`**. Verified:
`grep -rn fireworks codelet/**/src` finds the literal `fireworks` ONLY in test
fixtures and comments — never in non-test Rust source. The live loader
`provider_settings/profiles_config.rs::load_openai_profiles()` reads
`<HOME>/.fspec/fspec-config.json` then `<cwd>/spec/fspec-config.json`
(project-over-user merge) and pulls `providers.openai.profiles.*`. So the READ
half (PROV-100) genuinely works — that's exactly why the user's real profile
appears.

## 1. The ACTUAL defect (the "blank input area")

Pressing **Enter on a profile row** routes here —
`provider_settings/list_actions.rs::enter_on_nav_item`, `NavItemKind::Profile`
arm (lines 56–63):

```rust
NavItemKind::Profile { .. } => {
    view.mode = ProviderSettingsMode::Detail {
        provider_id,                       // = "openai" (the parent), profile_name DISCARDED
        sub: DetailSub::Summary { last_status: None },
    };
    ...
}
```

It opens a **read-only `Detail::Summary`** keyed by the *parent provider*
("openai"), completely ignoring which profile was selected. `render_detail`'s
Summary branch (`detail.rs:202-217`) prints only `display_name / provider_id /
credential type / models / configured`. There is **no editable form, no
profile name, no baseUrl/apiKey fields** → to the user this is a useless,
"blank input area" that is "clearly not wired up." Correct diagnosis.

The module header (`list_actions.rs:17-22`) already documents this as a known
parity gap: *"the Rust frontend has no profile-create, profile-edit, OAuth-login,
OAuth-disconnect or per-profile-delete modes."*

## 2. What TypeScript actually does (the parity target)

### Enter on a profile row → real EDIT form
`src/tui/inputHandlers/listModeHandler.ts:139-145` → `initializeEditProfile(...)`
(`src/tui/utils/providerSettingsHelpers.ts:72-87`):
- `setFormValues({ ...config })` (prefill from the stored ProfileConfig)
- `setProfileName(profileName)`, `formFieldIndex=0`, `isEditingName=false`
- mode = `{ type: 'edit-profile', providerId, profileName }`

### Enter on "+ Add Profile" → real CREATE form
`listModeHandler.ts:147` → `initializeNewProfile(...)`
(`providerSettingsHelpers.ts:52-67`):
- `setFormValues({ baseUrl: DEFAULT_PROFILE_BASE_URL /* http://localhost:8888 */, apiKey: '' })`
- `setProfileName('')`, `formFieldIndex=0`, `isEditingName=true` (cursor starts in NAME)
- mode = `{ type: 'create-profile', providerId }`

### The form fields (`PROFILE_FORM_FIELDS`, `constants/providerSettings.ts:12-18`)
`baseUrl, apiKey, contextWindow, maxOutputTokens, compactionThreshold` — PLUS the
profile **name** (separate, gated by `isEditingName`).
> `customModels` is NOT a form field — it is managed by the separate model-selector
> CRUD (`customModelCrudService.ts`, already ported in RPC-344/RPC-346). PROV-106
> does the baseUrl/apiKey/contextWindow/maxOutputTokens/compactionThreshold form ONLY.

### Form interaction (`src/tui/inputHandlers/profileFormModeHandler.ts`)
- Up/Down move `formFieldIndex`; Up from field 0 (create mode) re-enters NAME editing.
- **Tab is intentionally IGNORED** (TUI-084).
- Esc → back to `list`.
- Char input filtered to printable ASCII 32..=126 (`filterPrintableChars`).
- `contextWindow`/`maxOutputTokens` → `parseInt` (NaN → undefined).
- `compactionThreshold` → `parseCompactionThreshold` ("80%", "200000", …).
- **Save (Enter)** only if `baseUrl && apiKey && name` all truthy; builds
  `{ baseUrl, apiKey, ...optional(contextWindow,maxOutputTokens,compactionThreshold) }`
  → `saveProfileConfig(providerId, name, config)` → back to `list`.

### Delete (`d` on a profile row)
`listModeHandler.ts:169-174` → mode `delete-profile` → `deleteConfirmModeHandler.ts:46-53`
`y` confirms → `removeProfile(providerId, profileName)`; `n`/Esc cancels.

### Read/write of `~/.fspec/fspec-config.json` (`src/utils/profile-management.ts`)
- **saveProfile** (lines 39-89): **guards `providerId === 'openai'`** (else throws);
  reads the **user** file `<getFspecUserDir()>/fspec-config.json` ONLY (not the
  merged view); ensures `providers.openai.profiles`; sets
  `profiles[profileName] = profileConfig`; `writeConfig('user', config)`. Missing
  file → start from `{}`.
- **deleteProfile** (lines 97-132): same read; `delete profiles[profileName]`;
  `writeConfig('user', config)`. Missing file/profile → no-op return.

### Config JSON shape (exact)
```jsonc
{ "providers": { "openai": { "profiles": {
  "fireworks": {
    "baseUrl": "https://api.fireworks.ai/inference/v1",
    "apiKey": "sk-...",
    "contextWindow": 131072,                                   // optional
    "maxOutputTokens": 4096,                                   // optional
    "compactionThreshold": { "type": "percentage", "value": 80 }, // optional
    "customModels": [ ... ]                                    // managed elsewhere; PRESERVE on edit
  }
} } } }
```

## 3. Rust gap summary (what PROV-106 must build)

| Concern | TS | Rust today | PROV-106 must |
|---|---|---|---|
| Enter on profile | edit-profile form (prefilled) | read-only Summary placeholder | open real edit form |
| Enter on +Add Profile | create-profile form | explicit no-op (`Consumed`) | open real create form |
| `d` on profile | delete-profile confirm → remove one key | no-op | per-profile delete confirm |
| Form fields | name + 5 ProfileConfig fields | none | same 5 + name |
| Read config | full merged ProfileConfig (openai-only) | only `baseUrl` for the display string | read full ProfileConfig per profile |
| WRITE config | saveProfile/deleteProfile (user file, openai-guarded) | **DOES NOT EXIST** | add backend write path |
| customModels on edit | preserved (form never touches it) | n/a | MUST preserve existing customModels on profile edit |

## 4. Backend note (important)

There is currently **no profile write path** in the Rust backend.
`sessions/handle_impl.rs::set_provider_credentials` writes `credentials.json`
(api keys), NOT `fspec-config.json` profiles. `sessions/profile_sections.rs`
has `save_custom_model`/`delete_custom_model` (whole-file read-modify-write over
a `preserve_order` `serde_json::Value`, openai-guarded) — REUSE that exact
read-modify-write helper style to add `save_profile`/`delete_profile`
(write `providers.openai.profiles.<name>` whole-object; preserve sibling keys
incl. `customModels`, `theme`, other profiles). Mirror TS: user file only,
openai-guarded, missing file → `{}`.

## 5. ACDD constraints (unchanged)
- Strict 100% ACDD: feature file → failing tests → impl.
- Tests fully OFFLINE: temp config dir (path-injectable), full
  write→read→modify→write cycle, sibling-key preservation assertions, NO env
  mutation, NO real `~/.fspec`.
- New `ProviderSettingsMode`/`DetailSub` variant(s) for create/edit/delete;
  reuse `CustomModelForm` patterns where sensible.
- The existing PROV-102 test `enter_on_openai_profile_opens_openai_detail_not_anthropic`
  asserts the OLD placeholder (Enter→Detail/Summary). It MUST be updated within
  this work unit to assert the new edit-form mode (this is an intentional,
  in-ACDD behavior change — not a regression).
- Files < 300 LoC (watch `provider_settings/mod.rs` ~296 — may need PROV-107
  extraction first/concurrently); clippy `-D warnings`; cargo fmt clean; build
  incl. downstream core+napi.
- **NO git.** Work directly in the working tree.
