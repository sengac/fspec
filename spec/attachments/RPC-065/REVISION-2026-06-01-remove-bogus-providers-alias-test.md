# RPC-065 — REVISION: remove `/providers` alias parity test

**Status:** RPC-065 re-opened on 2026-06-01. The behaviour-parity suite asserts that
`/providers` is an alias for `/provider`, but no such alias exists in the TypeScript
Ink reference frontend.

---

## The defect

`codelet/fspec-tui/tests/behaviour_parity_rpc065.rs` contains a test
`slash_providers_alias_activates_provider_settings_view` (lines ~293-312) which dispatches
`SlashCommandAction::Providers` and asserts the view flips to `ViewMode::ProviderSettings`.
This test references a "legacy /providers alias" — but the TypeScript Ink frontend's
`src/tui/utils/slashCommands.ts` defines exactly one provider entry: `name: 'provider'`.
No `/providers` alias has ever existed in the reference frontend.

The test was added to lock in an alias the Rust implementation invented. The fix is to
delete the test (the singular `/provider` test
`slash_provider_activates_provider_settings_view` continues to provide coverage).

## Affected files

```
codelet/fspec-tui/tests/behaviour_parity_rpc065.rs
  Lines ~272-291 — DELETE the "/provider" section comment block (keep)
  Lines ~293-312 — DELETE the entire `slash_providers_alias_activates_provider_settings_view`
                    test, including its TS-REF and DEEP-REF doc comments

spec/features/  (if any rpc065-* feature file references the alias)
  - Audit feature files; remove any scenario named "Alias /providers activates ProviderSettings"
    or similar.
```

## Acceptance criteria

1. The `slash_providers_alias_*` function is gone from `behaviour_parity_rpc065.rs`.
2. `cargo test -p codelet-fspec-tui --tests behaviour_parity_rpc065 -- slash_provider`
   matches exactly one test (`slash_provider_activates_provider_settings_view`).
3. Any rpc065-* feature file with a scenario asserting the `/providers` alias is
   updated to remove that scenario.
4. The `slash_provider_activates_provider_settings_view` test continues to pass under
   the corrected `SlashCommandAction::Provider =>` dispatch arm.

## Why this is in RPC-065 (not RPC-020 / RPC-054)

RPC-020 owns the registry. RPC-054 owns the view. RPC-065 owns the **parity test
suite**. The test that locks in the bogus alias lives in RPC-065's deliverable, so
the surgical fix to the test file belongs in RPC-065's re-run.

## Out of scope

- Any other behaviour-parity test in `behaviour_parity_rpc065.rs`. Only the
  `/providers` alias test is removed.
- Adding new parity tests for the corrected `/provider` flow — that is RPC-054's job.
