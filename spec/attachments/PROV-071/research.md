# PROV-071 — Shared Rust `fspec_home()` helper

## Problem

Four separate Rust modules re-implement `FSPEC_HOME` resolution with subtly different semantics. Any future surface that touches `~/.fspec` must either duplicate the logic again or pick one of the existing private helpers. This violates DRY and makes it impossible to change the layout (e.g., `XDG_CONFIG_HOME` support) without a shotgun edit.

## Current duplicates (exhaustive)

| Module | Helper | Visibility | Semantics |
|--------|--------|------------|-----------|
| `codelet/providers/src/claude_auth.rs:31` | `get_fspec_home()` | private | If `FSPEC_HOME` set → use as-is; else `$HOME/.fspec/credentials` |
| `codelet/providers/src/copilot/auth.rs:154` | `get_fspec_home()` | private | Same semantics as claude_auth |
| `codelet/providers/src/custom/discovery.rs:43` | `get_global_providers_dir()` | private | If `FSPEC_HOME` ends in `credentials` → parent + `providers`; else `FSPEC_HOME/providers`; fallback `$HOME/.fspec/providers` |
| `codelet/providers/src/custom/management.rs:300` | `find_provider_config_path()` (inlined) | private | Same logic as discovery.rs, duplicated for `show_provider_info` path lookup |
| `codelet/providers/src/custom/custom_provider.rs:144` | inline `std::env::var("FSPEC_HOME")` | inline | Used for credential lookups from Rhai scripts |
| `codelet/providers/src/custom/management.rs:310` | inline `std::env::var("FSPEC_HOME")` | inline | Used during `apply_custom_provider_env_vars` |

## Contract mismatch

- **credentials dir**: `claude_auth` / `copilot/auth` treat `FSPEC_HOME` as the credentials directory itself.
- **providers dir**: `discovery.rs` treats `FSPEC_HOME` as either `<base>/credentials` (when it ends in that literal) or as the base itself, then appends `providers`.

This means if a user sets `FSPEC_HOME=/tmp/test-home` (not ending in `credentials`), **credentials go to `/tmp/test-home/claude_auth.json`** but **providers go to `/tmp/test-home/providers/*.json`**. The two are siblings under the "home" but credentials are flat while providers are nested — an unintuitive split caused by each helper making independent guesses about `FSPEC_HOME` semantics.

## Target design

Introduce a single module `codelet/providers/src/fspec_home.rs` (or promote to `codelet/core` if consumed outside `providers`):

```rust
//! Single source of truth for resolving the `~/.fspec` base directory
//! and its well-known subdirectories.

use std::path::PathBuf;

/// Subdirectory names relative to the fspec base directory.
pub const CREDENTIALS_SUBDIR: &str = "credentials";
pub const PROVIDERS_SUBDIR: &str = "providers";
pub const SCRIPTS_SUBDIR: &str = "providers"; // Rhai scripts live next to JSON

/// Return the **base** `.fspec` directory.
///
/// Resolution order:
///   1. `FSPEC_HOME` env var, with backwards-compat for the legacy
///      convention where it pointed at `<base>/credentials`.
///   2. `$HOME/.fspec`
///   3. `/tmp/.fspec` as a last-resort fallback.
pub fn fspec_home() -> PathBuf { ... }

/// `<base>/credentials`
pub fn credentials_dir() -> PathBuf { fspec_home().join(CREDENTIALS_SUBDIR) }

/// `<base>/providers` — both JSON configs and `.rhai` scripts live here.
pub fn providers_dir() -> PathBuf { fspec_home().join(PROVIDERS_SUBDIR) }

/// Project-local providers directory relative to `cwd`, i.e.
/// `<cwd>/.fspec/providers`. Returns even when it does not exist.
pub fn project_providers_dir(cwd: &Path) -> PathBuf { cwd.join(".fspec").join(PROVIDERS_SUBDIR) }
```

### Backwards-compatible `FSPEC_HOME` handling

- If `FSPEC_HOME` ends in `credentials` (literal suffix) → treat parent as base (legacy).
- Otherwise → treat value as base directly.
- Emit a one-shot `tracing::warn!` when we detect the legacy form so users can migrate their env vars.

## Call-site updates

Replace all six duplicates:

1. `claude_auth.rs:31` → `credentials_dir().join("claude_auth.json")`
2. `copilot/auth.rs:154` → `credentials_dir().join(COPILOT_AUTH_FILENAME)`
3. `discovery.rs:43` → `providers_dir()` + `project_providers_dir(&std::env::current_dir()?)`
4. `management.rs:300` (`find_provider_config_path`) → iterate both `providers_dir()` and `project_providers_dir(..)` entries
5. `custom_provider.rs:144` → `credentials_dir()` for Rhai credential lookups
6. `management.rs:310` → `credentials_dir()` for `apply_custom_provider_env_vars`

## Test harness

A per-test `FspecHomeGuard` (like the one at `copilot/auth.rs:260`) must become a shared helper:

```rust
// codelet/providers/src/test_support/fspec_home_guard.rs
pub struct FspecHomeGuard { _tempdir: TempDir, original: Option<String> }
impl FspecHomeGuard { pub fn new() -> Self { ... } }
impl Drop for FspecHomeGuard { ... }
```

`copilot/auth.rs:260` and `manager.rs:1099` currently implement this twice — both should be replaced.

## Out of scope (deliberately)

- `XDG_CONFIG_HOME` support (Linux standards) — track as a separate LCP/CONFIG card.
- Per-workspace override (`$WORKSPACE/.fspec` outside CWD) — tracked separately.

## Acceptance summary

- Six existing call sites compile against the new API with identical observable behavior.
- New `FspecHomeGuard` replaces the two ad-hoc test guards.
- Legacy `FSPEC_HOME=<base>/credentials` emits a deprecation warning.
- 100% of Rust modules that touch `~/.fspec` go through this helper — verifiable via grep for `std::env::var("FSPEC_HOME")` returning only matches inside the helper module itself.

## References

- `codelet/providers/src/claude_auth.rs:30-42`
- `codelet/providers/src/copilot/auth.rs:150-167`
- `codelet/providers/src/custom/discovery.rs:38-63`
- `codelet/providers/src/custom/management.rs:295-315`
- `codelet/providers/src/custom/custom_provider.rs:140-150`
- `src/utils/config.ts:13` (`getFspecUserDir` — TS does NOT honour `FSPEC_HOME`, out of scope here but callers should read via NAPI post-PROV-072)
