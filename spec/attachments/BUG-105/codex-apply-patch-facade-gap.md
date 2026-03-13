# BUG-105 Codex apply_patch facade gap

## Summary

In Codex-backed sessions, patch-style editing appears to be missing as a first-class tool capability.
Instead of issuing a native patch/edit tool call, the agent falls back to shell execution such as:

```sh
apply_patch <<'PATCH'
...
PATCH
```

That fails in the session environment with:

```text
sh: apply_patch: command not found
```

This indicates the model/runtime path did not expose or select a dedicated patch-edit tool, even though the session provides structured editing tools such as `Edit` and `Write`.

## User-visible failure

Observed failure mode:

- agent decides to modify a file,
- agent emits a shell command that wraps `apply_patch`,
- shell tries to execute a binary/function named `apply_patch`,
- command fails because no such shell command exists in the environment,
- user must correct the agent and redirect it to the structured edit tools.

## Why this is a bug

This is not just a bad one-off choice by the model. The behavior suggests a capability/facade mismatch:

- the Codex provider path did not surface a first-class patch operation clearly enough,
- or the tool-selection guidance/facade biased the model toward shell fallback,
- or `apply_patch` is expected by Codex but is missing from the actual tool inventory for this integration.

The result is broken editing behavior and unnecessary shell misuse.

## Related external report

This appears related to upstream Codex issue:

- `https://github.com/openai/codex/issues/2005`
- Title: ``apply_patch` command generates requests with illegible diff`

That issue indicates `apply_patch` behavior is already problematic in Codex itself. In this integration, the symptom is even more basic: the runtime falls back to trying to execute `apply_patch` in the shell where it does not exist.

## Local evidence

Observed directly in this session:

```text
Command failed with exit code 127
sh: apply_patch: command not found
```

The session also exposes structured editing tools such as:

- `Edit`
- `Write`

So a correct implementation path should prefer those tools, or expose a real first-class `apply_patch` tool if Codex expects one.

## Suspected root cause

Likely causes to investigate in the Codex tool-calling integration:

1. The Codex facade/tool inventory omits a native `apply_patch` capability.
2. Tool descriptions or system guidance over-emphasize shell usage instead of structured edit tools.
3. The provider-specific tool facade maps editing operations inconsistently across agents.
4. The Codex integration assumes an `apply_patch` helper exists in the shell environment when it does not.

## Expected behavior

When the agent needs to patch files:

- it should use a first-class file-editing tool,
- it should not emit shell-wrapped `apply_patch` unless such a command is actually provided,
- and patch diffs should remain legible and executable through the supported tool path.

## Acceptance direction

A fix should ensure at least one of these is true:

- Codex sessions receive a real patch-edit tool call path, or
- Codex sessions are guided to use `Edit`/`Write` directly for file changes.

And in either case:

- the agent must stop attempting `apply_patch` as a shell command,
- editing requests must succeed through the supported tool interface,
- and the behavior should be consistent with other provider integrations.
