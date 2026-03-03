# PROV-029: Anthropic OAuth Provider Incorrectly Shows Profiles

## Screenshot

See `stillwrong.png` attached to this card.

The screenshot shows the Provider Settings TUI with Anthropic expanded:

```
▼ Anthropic ✓ OAuth [Claude] (1 profile)
      ✓ OAuth [Claude]
      🔑 Login with Claude (browser)
      🔑 Login with Claude (headless)
      📁 anthropic → http://localhost:8888        ← THIS IS WRONG
```

## What's Wrong

Anthropic is an **OAuth provider** (`authType: 'oauth'` in the provider registry). OAuth providers authenticate via OAuth token exchange — they have no business showing profiles. Profiles are a concept for **local server configurations** (vLLM, Ollama, custom OpenAI-compatible endpoints) where you point a `baseUrl` at a local server and provide an `apiKey`.

Yet Anthropic is showing:
1. **A profile row** (`📁 anthropic → http://localhost:8888`) — a profile that shouldn't exist
2. **A profile count** in the header (`(1 profile)`) — misleading to the user
3. **The profile's baseUrl is `http://localhost:8888`** — this is the local OAuth form server port, not a real API endpoint

## Root Cause: Stale Config Data

The user config at `~/.fspec/fspec-config.json` contains:

```json
{
  "providers": {
    "anthropic": {
      "profiles": {
        "anthropic": {
          "baseUrl": "http://localhost:8888",
          "apiKey": "sk-ant-oat01-..."
        }
      }
    }
  }
}
```

Key observations:
- The `apiKey` value (`sk-ant-oat01-*`) is an **OAuth access token** (the `oat01` segment = OAuth token), not a regular Anthropic API key
- The `baseUrl` is `http://localhost:8888` — the port used by the local OAuth form server in `claude_oauth_server.rs`
- This profile was likely created accidentally during OAuth development/testing, or there's a code path that saved OAuth results as a profile

## Root Cause: Missing Guards (6 locations)

### 1. `buildNavItems()` — Missing profile display guard
**File:** `src/tui/hooks/useProviderSettingsState.ts`, lines 187-193

```typescript
// This iterates ALL profiles regardless of provider type
for (const profile of provider.profiles) {
  items.push({
    type: 'profile',
    providerId: provider.id,
    profileName: profile.name,
  });
}
```

The "Create Profile" button IS correctly guarded on line 195:
```typescript
if (!isOAuthProvider(provider.id)) {
  items.push({ type: 'add-profile', providerId: provider.id });
}
```

But **existing profiles still render** — the guard only prevents creating NEW profiles.

**Fix:** Wrap the profile iteration in `if (!isOAuthProvider(provider.id))`.

### 2. `reload()` — Missing profile load guard
**File:** `src/tui/hooks/useProviderSettingsState.ts`, line 304

```typescript
// Loads profiles for ALL providers, including OAuth ones
const profiles = await loadProviderProfiles(providerId);
```

**Fix:** Skip `loadProviderProfiles()` for OAuth providers, or pass empty array.

### 3. `saveProfile()` — Missing creation guard
**File:** `src/utils/provider-config.ts`, line 479

```typescript
export async function saveProfile(
  providerId: string,
  profileName: string,
  profileConfig: ProfileConfig
): Promise<void> {
  // No validation — accepts ANY providerId
```

**Fix:** Add `if (isOAuthProvider(providerId)) throw new Error(...)` at the top.

### 4. Profile count in header — Missing display guard
**File:** `src/tui/components/ProviderSettingsPanel.tsx`, lines 518-524

```tsx
{profileCount > 0 && (
  <Text dimColor={!isSelected}>
    {' '}({profileCount} profile{profileCount !== 1 ? 's' : ''})
  </Text>
)}
```

**Fix:** Add `&& !isOAuthProvider(item.providerId)` to the condition.

### 5. Legacy `ProviderSettingsView.tsx` — Same profile display issues
**File:** `src/tui/components/ProviderSettingsView.tsx`, line 153

Same pattern — iterates profiles without checking OAuth status.

### 6. No config migration/cleanup
When a provider's `authType` changes from `api-key` to `oauth`, any existing profiles in the config become stale orphans. There's no migration to clean them up.

**Fix:** Either add a migration that removes profiles for OAuth providers on startup, or have `reload()` silently ignore them.

## Recommended Fix Order

1. **Guard `buildNavItems()`** — immediate visual fix, profiles stop showing
2. **Guard `reload()`** — don't even load profiles for OAuth providers
3. **Guard `saveProfile()`** — prevent future accidental creation
4. **Guard header count** — no misleading "(N profiles)" text
5. **Clean up stale config** — remove `providers.anthropic.profiles` from config
6. **Consider migration** — auto-clean profiles for OAuth providers on app startup
