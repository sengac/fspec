# Research: UPD-001 release-pipeline tests pinned to superseded decisions

Target: `cargo test --profile ci-test -p codelet-fspec --test upd001_release_pipeline`
Result on current tree **and on clean HEAD**: 4 passed, 4 failed. Pre-existing.

## Failures and causes

| Test | Assertion | Actual |
|---|---|---|
| `scenario_workspace_version_is_bumped_to_0_10_1` (l.83) | `[workspace.package] version == "0.10.1"` | `0.10.9` (`rust/Cargo.toml:49`) |
| `scenario_release_binary_reports_the_new_version` (l.313) | `fspec --version` prints `fspec 0.10.1` | prints `fspec 0.10.9` |
| `scenario_linux_binary_runs_on_old_glibc_distros` (l.335) | `release.yml` contains `ubuntu-20.04` | x86_64 Linux now builds a **static musl** binary on `ubuntu-24.04` (commit `5cd28d94` "fix(release): x86_64 linux static musl binary (UPD-001)"), superseding `89da2c4d` (ubuntu-20.04) |
| `scenario_release_workflow_builds_all_five_targets` (l.143) | per-target `os` matches `EXPECTED_TARGETS` | matrix changed: `x86_64-pc-windows-msvc` → `ubuntu-24.04` via cargo-xwin, `aarch64-pc-windows-msvc` → `windows-11-arm`, linux x86_64 `build_target: x86_64-unknown-linux-musl` |

Current matrix (`.github/workflows/release.yml:45-67`):
```
x86_64-pc-windows-msvc    os: ubuntu-24.04      (cargo-xwin, zip)
aarch64-pc-windows-msvc   os: windows-11-arm
x86_64-unknown-linux-gnu  os: ubuntu-24.04      build_target x86_64-unknown-linux-musl
aarch64-unknown-linux-gnu os: ubuntu-24.04-arm
aarch64-apple-darwin      os: macos-latest
```

## Suggested fix
The tests encode a one-off release (v0.10.1) as permanent behavior. Rewrite the scenarios as
invariants: the workspace version equals `fspec --version` output (no literal version);
exactly five targets with the current runner/build-target table; the Linux x86_64 asset is
statically linked (musl), which satisfies the old-glibc requirement without an
`ubuntu-20.04` runner. Update `spec/features` for UPD-001 accordingly.
