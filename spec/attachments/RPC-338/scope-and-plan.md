# RPC-338 — Model selector profile sections + unreachable markers

**Split out of RPC-337.** RPC-337 builds the full-screen model selector
mode-view (`views/model_selector/`) and the shared full-screen shell.
During RPC-337 specifying we discovered that two pieces of TS parity
**cannot be implemented without a wire-type change** that RPC-337's
research explicitly scopes out ("Wire types `ProviderInfo` /
`ModelEntry` … already drive both TS and Rust selectors — **unchanged**").
This card carries that deferred work.

## Why this was deferred from RPC-337

The original TypeScript `ModelSelectorView.tsx` renders a richer data
model than the Rust wire types currently expose:

| Concept | TS source | Rust wire type | Status |
|---------|-----------|----------------|--------|
| 📁 Profile (local-server) sections | `ProviderSection.profileName` / `profileConfig` (`src/tui/types/provider.ts:29,77`) | **absent** from `ProviderInfo` (`codelet/rpc-types/src/lib.rs:362`) | ❌ needs wire change |
| `(unreachable)` provider marker | `ProviderSection.isUnreachable` (`src/tui/types/provider.ts:33`) | **absent** from `ProviderInfo` | ❌ needs wire change |
| `[C]` custom badge | `customModelIds` / per-model | `ModelEntry.is_custom` **exists** (`lib.rs:347`) | ✅ stays in RPC-337 |
| `(current)` marker | `currentModelId` | available via `get_model_info` | ✅ stays in RPC-337 |

So `[C]` and `(current)` remain in RPC-337 (backable today). Profiles +
unreachable move here because they require a cross-crate wire change
plus a server-side data source.

## Scope of this card

### 1. Wire-type extension (`codelet/rpc-types/src/lib.rs`)
Extend `ProviderInfo` (and/or introduce a `ProviderSection`-equivalent)
to carry, per the TS `ProviderSection` shape:
- `profile_name: Option<String>` — present for local-server profile
  sections (drives the 📁 icon + `"provider: profile"` label).
- `is_unreachable: bool` — drives the red `(unreachable)` marker.
- (assess) any `profile_config` fields actually consumed by rendering.

Keep the `#[cfg_attr(feature = "napi", napi_derive::napi(object))]` +
`Serialize`/`Deserialize` + `Default` pattern used by the surrounding
shapes. `napi(object)` does NOT support discriminated enums — follow the
existing `Option<String>` convention (see `masked_key`/`source` on
`ProviderCredentialInfo`).

### 2. napi bindings
Mirror the new fields in `codelet/napi/src/models/napi_bindings.rs`
(`NapiProviderModels` and friends) so the JS surface stays in sync.

### 3. Server-side data source
Populate the new fields in the `list_providers()` implementation
(provider registry / `codelet-providers`) so profile sections and
reachability actually flow over the wire. Identify where the TS side
computes `profileName` / `isUnreachable` and port the equivalent.

### 4. Rendering in `views/model_selector/` (depends on RPC-337)
Once RPC-337 lands the mode-view + its row→Line builder:
- Render 📁 profile sections **alongside** cloud providers, labelled
  like the TS view (`📁 provider: profile-name`).
- Render a red `(unreachable)` marker on unreachable provider headers.
- **Restore the `📁 Profile (local server)` segment of the legend line.**
  RPC-337 ships the legend as `[R] Reasoning | [V] Vision | [C] Custom`
  (no profile segment, since profiles aren't rendered there); this card
  appends ` | 📁 Profile (local server)` for full TS parity.
- Keep each file < 300 LoC (project source-shape rule).

## Acceptance criteria to capture during this card's Example Mapping
(seed — refine via `fspec add-rule` / `add-example`)

- **Rule:** `ProviderInfo` (or new section type) carries
  `profile_name: Option<String>` and `is_unreachable: bool`, propagated
  through rpc-types → napi → server `list_providers()`.
- **Rule:** Profile (local-server) sections render with a 📁 icon
  alongside cloud providers, matching the TS `ProviderSection` data model.
- **Rule:** Unreachable provider headers render a red `(unreachable)`
  marker.
- **Example:** A local-server profile provider shows with a 📁 icon and
  its profile-qualified label.
- **Example:** An unreachable provider header shows a red `(unreachable)`
  marker; its models remain listed/navigable per TS behaviour.
- **Example:** Both transports (in-process + websocket) return the new
  fields identically (cross-transport parity).

## References
- TS data model: `src/tui/types/provider.ts` (`ProviderSection`,
  `profileName`, `isUnreachable`, `customModelIds`).
- TS rendering: `src/tui/components/ModelSelectorView.tsx`
  (lines 150-187 section render; 178-183 unreachable; 168-172 profile).
- Rust wire types: `codelet/rpc-types/src/lib.rs:341` (`ModelEntry`),
  `:362` (`ProviderInfo`).
- Parent card: RPC-337 (full-screen model selector mode-view + shared
  shell). This card `dependsOn` RPC-337.
