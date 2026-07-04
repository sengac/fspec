# PROV-130 — Provider section ordering + default-model selection parity

**Parent:** PROV-126 · **Discrepancy #3** · **Type:** bug · **Depends on:** PROV-129

## Problem

Rust assembles sections **cloud/canonical first**, then custom providers, then local
profiles **last** (`handle_impl.rs:996`). TS assembles them in the opposite priority:

```ts
// modelInitializationService.ts:196-200
[...profileSections, ...customSections, ...cloudSections]
```

Both Rust and TS auto-select the **first section that has models** as the default model.
Because the section order is inverted, the auto-selected default model differs between
the two implementations.

## Scope of THIS card

1. Reorder the Rust section assembly to match TS: **profiles → custom → cloud**.
2. Verify the first-available default-model selection now matches TS (the default becomes
   the first model of the first profile/custom section when present, else the first cloud
   section).
3. Land after PROV-127/128/129 so ordering is validated against the final, correctly
   populated + Codex-synthesized section set.

**Out of scope:** dropping empties (PROV-127), gating (PROV-128), Codex synthesis (PROV-129).

## Acceptance criteria (example-map seeds)

- **Rule:** Sections are ordered profiles first, then custom providers, then cloud
  providers.
- **Rule:** The auto-selected default model is the first model of the first section in the
  ordered list.
- **Example:** With a local profile and cloud creds present, the profile section appears
  before cloud sections and the default model comes from the profile.
- **Example:** With no profiles/custom providers, cloud sections appear in canonical order
  and the default comes from the first populated cloud section.

## Key files

- `codelet/sessions/src/handle_impl.rs:996` — section assembly order + local profile append.
- `codelet/fspec-tui/src/views/model_selector/state.rs` — default/initial selection.
- TS reference: `src/tui/services/modelInitializationService.ts:196-200`.
