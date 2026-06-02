# RPC-020 — REVISION: remove non-existent `/providers` command

**Status:** RPC-020 re-opened on 2026-06-01 because the first pass invented a `/providers`
slash command that does not exist in the TypeScript Ink reference frontend. This file
documents the surgical fix that this re-run delivers.

---

## The defect

In the TypeScript Ink reference frontend (`src/tui/utils/slashCommands.ts`), the slash
command registry contains **exactly one** provider-related entry:

```typescript
{
  name: 'provider',
  description: 'Configure API providers',
  requiresSession: false,
},
```

There is NO `/providers` command. There is NO alias mapping `/providers` → `/provider`.

When RPC-020 was implemented, an extra `SlashCommandAction::Providers` variant was added
to the Rust enum, a second registry entry was added to `SLASH_COMMANDS`, and a
`SlashCommandAction::Provider | SlashCommandAction::Providers` arm was added to
`dispatch_rpc020.rs`. This invented a command that does not exist in the TS frontend
and breaks the 1:1 parity contract documented in RPC-020 rule [0]:

> "Slash command registry mirrors the TS SLASH_COMMANDS list (model, provider, debug,
> clear, compact, thinking, resume, detach, search, blocklist, role, merge-worktree,
> schedule, loop)…"

The original rule's list did not include `providers`. The `+ a few from the dossier
(help, quit, isolation, providers)` clause was a misread of the dossier — the dossier
references the *TypeScript registry* (which has `provider` singular) and the *help /
quit / isolation* additions that genuinely don't appear in the TS registry. `providers`
plural was never authorised.

## Affected files

```
codelet/fspec-tui/src/views/agent/slash_commands.rs
  - Remove `SlashCommandAction::Providers` variant
  - Remove the `SlashCommandAction::Providers => "providers"` arm in `name()`
  - Remove the `SLASH_COMMANDS` entry with action: Providers, description: "Open provider settings"

codelet/fspec-tui/src/app/dispatch_rpc020.rs
  - Change `SlashCommandAction::Provider | SlashCommandAction::Providers =>`
    to `SlashCommandAction::Provider =>`
  - Update the inline comment to drop the "legacy /providers alias" reference

codelet/fspec-tui/tests/behaviour_parity_rpc065.rs
  - Delete the `slash_providers_alias_activates_provider_settings_view` test
  - Delete the `/providers — alias for /provider` section header

spec/features/rpc020-slash-and-file-popups.feature
  - No scenarios for /providers exist in the feature file; nothing to remove here.
    (The bogus alias was added downstream in RPC-065's behaviour-parity suite.)
```

## Acceptance criteria (this revision)

1. `grep -nE "SlashCommandAction::Providers|/providers" codelet/fspec-tui/src/` returns
   only doc-comment paths like `~/.fspec/providers/` (filesystem references) and not
   any reference to a `/providers` slash command.
2. `SLASH_COMMANDS` registry length DECREASES by exactly one entry.
3. `cargo build -p codelet-fspec-tui` succeeds (no orphan match arms / no dangling
   variant references).
4. `cargo test -p codelet-fspec-tui --tests behaviour_parity_rpc065` passes with the
   `slash_providers_alias_*` test removed.
5. The remaining `slash_provider_activates_provider_settings_view` test continues to
   pass.
6. The TS Ink frontend reference at `src/tui/utils/slashCommands.ts` is unchanged
   (no `/providers` entry was ever there).

## Why this is in RPC-020 (not RPC-054)

RPC-020 owns the slash command registry. RPC-054 owns the ProviderSettingsView. The
`Providers` variant was added to the **registry** during RPC-020 implementation and
later wired by RPC-054's dispatch arm. The clean fix touches the registry first
(RPC-020 revision), then the dispatcher arm consumed by RPC-054 (handled in the
RPC-054 revision attachment).

## Out of scope

- Behaviour parity for the singular `/provider` command is intact and stays. No
  regression in the singular flow.
- Renaming or aliasing `/provider` to anything else.
