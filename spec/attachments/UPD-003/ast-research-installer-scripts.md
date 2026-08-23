# AST Research — UPD-003 (installer scripts)

## Scope
Shell scripts in `scripts/` (not indexable by the AST tool — it only covers
source languages). Research performed via direct file reads + GitHub API
inspection instead.

## Existing code analyzed

### scripts/install.sh (183 lines)
- Pure from-source installer: cargo/git prereq checks, protoc warning,
  `cargo build --profile release-slim -p codelet-fspec -j $BUILD_JOBS`,
  install to `$INSTALL_DIR` (default `~HOME/.local/bin`), PATH hint,
  `fspec --version` verify.
- Supports `--dir`, `--profile`, `INSTALL_DIR`, `BUILD_PROFILE`, `BUILD_JOBS`.
- Piped-mode handling: if `BASH_SOURCE` is absent and cwd lacks `rust/`,
  clones the repo (`git clone --depth 1`).
- **This entire behavior moves to `build-install.sh` unchanged.**

### scripts/install.ps1 (325 lines)
- Already a download installer for Windows: GitHub API `/releases/latest`,
  platform detection (x86_64/aarch64-pc-windows-msvc), download, checksum
  (checksums.txt → per-file .sha256 → skip with warning), Expand-Archive,
  install, PATH hint, version verify.
- **No changes needed** — its checksum path activates once the pipeline
  publishes `checksums.txt`.

### .github/workflows/release.yml (UPD-001)
- 5-target matrix; assets named `fspec-<target-triple>.zip|tar.gz`
  (UPD-002 self-updater filename contract).
- Release job flattens into `release/`, verifies archives, publishes via
  softprops/action-gh-release with `files: release/fspec-*`.
- **Gap:** no `checksums.txt` asset. Fix: generate
  `sha256sum release/fspec-* > release/checksums.txt` and widen the
  `files:` glob to `release/fspec-*, release/checksums.txt`.

### Reference: vinhnx/vtcode scripts/install.sh
- Pattern adopted: `detect_platform` (uname -s/-m → triple), candidate
  platform lists, GitHub API `releases?per_page=10` + HEAD-check walk
  newest→oldest, `releases/latest` redirect fallback, `curl -fSL -#`
  download to `mktemp -d`, sha256 verify (warn+skip if checksums missing),
  tar/unzip extract, `cp`+`chmod +x` to `$INSTALL_DIR`, PATH hint,
  `--version` verify. All logs to stderr.

## Platform → asset mapping (current release matrix)
| Host (uname -s / -m) | Asset target triple |
|---|---|
| Darwin / arm64 | aarch64-apple-darwin |
| Darwin / x86_64 | x86_64-apple-darwin (NOT published yet — will fail gracefully) |
| Linux / x86_64 | x86_64-unknown-linux-gnu (static-musl binary) |
| Linux / aarch64 | aarch64-unknown-linux-gnu |
| MINGW*/MSYS* | error → WSL or install.ps1 |

Asset URL: `https://github.com/sengac/fspec/releases/download/<tag>/fspec-<triple>.tar.gz`
(tag keeps `v` prefix; filename has no version segment).

## Test strategy
`rust/fspec/tests/inst003_installers.rs` shells out to the scripts with
stubbed `uname`/`curl` on a PATH prefix (no network, no toolchain):
- unsupported arch → fast fail before any curl call
- happy path (Darwin/arm64 + Linux/x86_64) → real tar.gz fixture, real
  sha256 verification, binary installed, version printed
- missing checksums.txt (curl exit 22) → warn + succeed
- checksum mismatch → abort, expected/actual printed, nothing installed
- `--dir` override
- static checks: build-install.sh preserves source-build behavior,
  install.sh has no cargo/git invocations, README + release.yml updated
