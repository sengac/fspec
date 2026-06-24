# PROV-100 — Custom OpenAI profiles not loaded from `~/.fspec/fspec-config.json`

## Symptom
The TS screen shows `OpenAI API` expanded with a custom profile
`fireworks → https://api.fireworks.ai/inference` plus `+ Create new profile`.
The Rust screen shows only `+ Add Profile` — the fireworks profile is missing.

## Root cause
1. **Different store, never read.**
   TS stores custom OpenAI profiles in `~/.fspec/fspec-config.json` under
   `providers.openai.profiles`, deep-merged with project `<cwd>/spec/fspec-config.json`.
   - `loadProviderProfiles('openai')` → `src/utils/profile-management.ts:23-28`
   - `loadConfig()` merge → `src/utils/config.ts:79-90`,
     `getFspecUserDir()` = `join(homedir(), '.fspec')` (`config.ts:13-15`)
   - JSON shape:
     ```json
     { "providers": { "openai": { "profiles": {
        "fireworks": { "baseUrl": "https://api.fireworks.ai/inference",
                       "apiKey": "...", "customModels": [...] } } } } }
     ```
   **Rust has zero code reading `fspec-config.json` for provider profiles.**
   `discover_provider_configs()` (`providers/src/custom/discovery.rs:26-36`) reads a
   DIFFERENT location and shape: `~/.fspec/providers/*.json` and `.fspec/providers/*.json`
   (whole custom *providers*, not openai profiles).

2. **Dispatch always passes empty profiles.**
   `dispatch_provider_settings.rs::handle_provider_credentials_loaded` calls
   `project_display_infos(&list, &[])` — empty slice for `openai_profiles`. The
   doc comment says "empty until a list-profiles RPC exists". `project_one` only
   copies profiles for `openai`, but the caller always supplies `&[]`, so even
   OpenAI gets `profiles = []`.

## Fix direction
1. Add a way to load OpenAI profiles from `~/.fspec/fspec-config.json`
   (`providers.openai.profiles`), merged with project `<cwd>/spec/fspec-config.json`
   (project overrides user). Mirror the TS merge semantics.
   - Profile display string in TS: `"{name} → {baseUrl}"`.
   - Path resolution must be injectable for offline tests (don't hard-read real `$HOME`).
2. Wire those profile names into `project_display_infos(&list, &openai_profiles)`
   so the OpenAI provider row renders profile children + the count, plus the
   trailing add-profile row. The projection layer already supports a non-empty
   `openai_profiles` slice (`projection.rs:96-100`) and the nav-item builder
   renders `Profile { profile_name }` rows + `AddProfile`.
3. Decide where the load runs: either in the dispatch handler before
   `project_display_infos`, or behind a small backend call. Prefer a pure,
   path-injectable loader function in a sessions/providers module so it is
   unit-testable offline, then call it from the dispatch handler.

## Files in play
- NEW loader (suggest) `codelet/sessions/src/...` or `codelet/providers/src/...`
  reading `fspec-config.json` `providers.openai.profiles`
- `codelet/fspec-tui/src/app/dispatch_provider_settings.rs:69-79` (pass real profiles)
- `codelet/fspec-tui/src/views/provider_settings/projection.rs:96-100` (already supports)
- (reference TS) `src/utils/profile-management.ts`, `src/utils/config.ts`,
  `src/utils/provider-config.ts`

## Acceptance pointers
- Given `~/.fspec/fspec-config.json` with a `fireworks` openai profile, when
  Provider Settings loads and OpenAI is expanded, a `fireworks → <baseUrl>` row
  appears above `+ Add Profile`.
- Project `spec/fspec-config.json` profiles override user profiles by name.
- Deterministic/offline: inject the config dir via a temp path; no real `$HOME`,
  no network.
