# UPD-001: Cross-Platform Release Pipeline — Implementation Guide

> **Status**: Research complete (2026-08-20). This document is the authoritative
> implementation plan for UPD-001. Follow it section-by-section.

## 1. Goal

Bump the workspace version from `0.1.0` to `0.10.0` and add a GitHub Actions
CI/CD pipeline that builds the standalone `fspec` binary for five targets and
publishes them as a GitHub Release:

| # | Target triple            | Artifact name                        | Format   |
|---|--------------------------|--------------------------------------|----------|
| 1 | `x86_64-pc-windows-msvc` | `fspec-x86_64-pc-windows-msvc.zip`   | zip      |
| 2 | `aarch64-pc-windows-msvc`| `fspec-aarch64-pc-windows-msvc.zip`  | zip      |
| 3 | `x86_64-unknown-linux-gnu`| `fspec-x86_64-unknown-linux-gnu.tar.gz` | tar.gz |
| 4 | `aarch64-unknown-linux-gnu`| `fspec-aarch64-unknown-linux-gnu.tar.gz` | tar.gz |
| 5 | `aarch64-apple-darwin`   | `fspec-aarch64-apple-darwin.tar.gz`  | tar.gz   |

**Asset naming is a contract with UPD-002** (the self-updater). The
`self_update` crate's GitHub backend matches release assets by the
`bin_name` prefix — every asset name MUST start with `fspec-` and end with
`.zip` (Windows) or `.tar.gz` (unix). Do not rename these.

## 2. Version Bump (0.1.0 → 0.10.0)

The workspace uses a single source of truth:

```toml
# rust/Cargo.toml
[workspace.package]
version = "0.10.0"
```

Every member crate uses `version.workspace = true`, so editing this one line
bumps all 25 crates. Then:

```bash
cd rust
cargo update --workspace   # refresh Cargo.lock version entries
```

**Verification**: `cargo metadata --format-version 1 | python3 -c
"import json,sys; d=json.load(sys.stdin); print(sorted({p['version'] for p in
d['packages']}))"` must show only `0.10.0`.

**Tag**: after the pipeline is proven, tag the commit `v0.10.0` and push.
The release workflow triggers on `push: tags: ['v*']`.

## 3. Build Strategy: Native Matrix (NOT cross-compilation)

### Decision: one native runner per target

The repo already has `scripts/build-cross.sh` (cargo-xwin + cargo-zigbuild,
macOS-hosted). **Do not use it in CI.** Rationale:

1. **Windows ARM64** (`aarch64-pc-windows-msvc`) is the killer app:
   `cargo-xwin` has historically had poor/unreliable ARM64-MSVC support, and
   the zig path does not cover MSVC at all. A native `windows-11-arm`
   runner builds it correctly out of the box.
2. **Native builds are faster and more reliable** than cross (no toolchain
   bootstrap, no AVX-512 flag hacks — see the `zig_cflags` workaround in
   `build-cross.sh` L233-242, which exists precisely because cross is
   fragile).
3. **`release-slim` profile** (rust/Cargo.toml L332+) is designed for
   shipping the standalone binary: LTO on, DWARF off → ~150 MB → small
   artifacts. Use it for every target.
4. GitHub-hosted runners: `ubuntu-24.04` (x86_64 Linux), `ubuntu-24.04-arm`
   (aarch64 Linux), `windows-11` (x86_64 MSVC), `windows-11-arm` (aarch64
   MSVC), `macos-latest` (aarch64 Apple Silicon — GitHub's mac runners have
   been Apple Silicon since 2024).

**Caveat — `windows-11-arm` runner availability**: ARM Windows runners are
available on GitHub-hosted runners (beta, `windows-11-arm`). If they are
unavailable in your org, fallback: build `aarch64-pc-windows-msvc` on
`windows-11` via `cargo xwin` (works for x86_64→aarch64 MSVC with a full
LLVM + lld install) — but prefer the native runner.

### Toolchain pinning

`rust/rust-toolchain.toml` pins `1.95.0` (required by the `merman-core`
dependency). All runners must install it:

```yaml
- uses: dtolnay/rust-toolchain@stable
  with:
    toolchain: "1.95.0"
```

(Or `actions-rs/toolchain@v1` with `toolchain: 1.95.0`.)

## 4. The Workflow File

Create `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags:
      - "v*"
  workflow_dispatch:          # manual trigger for testing

permissions:
  contents: write             # required to create releases

env:
  CARGO_TERM_COLOR: always

jobs:
  build:
    name: Build ${{ matrix.target }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: x86_64-pc-windows-msvc
            os: windows-11
            archive: zip
          - target: aarch64-pc-windows-msvc
            os: windows-11-arm
            archive: zip
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-24.04
            archive: tar.gz
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-24.04-arm
            archive: tar.gz
          - target: aarch64-apple-darwin
            os: macos-latest
            archive: tar.gz
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain (pinned)
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: "1.95.0"

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: rust

      - name: Build (release-slim)
        working-directory: rust
        run: cargo build --profile release-slim --target ${{ matrix.target }} -p codelet-fspec

      - name: Package
        working-directory: rust/target/${{ matrix.target }}/release-slim
        run: |
          if [ "${{ matrix.archive }}" = "zip" ]; then
            zip -j ../../../../dist-fs.zip fspec.exe
            mv ../../../../dist-fs.zip ../../../../fspec-${{ matrix.target }}.zip
          else
            tar -czf ../../../../fspec-${{ matrix.target }}.tar.gz fspec
          fi
        shell: bash

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: fspec-${{ matrix.target }}
          path: |
            fspec-${{ matrix.target }}.zip
            fspec-${{ matrix.target }}.tar.gz

  release:
    name: Create GitHub Release
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Collect artifacts
        uses: actions/download-artifact@v4
        with:
          path: dist

      - name: Flatten into release dir
        run: |
          mkdir -p release
          find dist -name 'fspec-*.zip' -o -name 'fspec-*.tar.gz' | xargs -I{} cp {} release/
          ls -lh release/

      - name: Create release
        uses: softprops/action-gh-release@v2
        with:
          tag_name: ${{ github.ref_name }}
          name: fspec ${{ github.ref_name }}
          files: release/fspec-*
          generate_release_notes: true
```

### Key design points

- **`softprops/action-gh-release@v2`** is the de-facto standard for creating
  releases with assets (search-verified, used by most Rust CLI projects).
  It creates the release, attaches all matched files, and updates if the tag
  already exists (idempotent re-runs).
- **`Swatinem/rust-cache@v2`** caches the `rust/` workspace; a cold build of
  this 25-crate workspace with LTO takes 15-40 min, cached ~5-10 min.
- **`working-directory: rust`** — the workspace root is `rust/`, not the repo
  root. `cargo build` MUST run there so `rust/rust-toolchain.toml` is
  respected (same trap documented in `build-cross.sh` L212-215).
- **Artifact names** are produced by the packaging step and match the
  `self_update` contract exactly.
- **Manual trigger** (`workflow_dispatch`) lets you test the pipeline on a
  branch by creating a throwaway tag or dispatching (dispatch does not
  create a release without a tag — use a test tag like `v0.10.0-rc.1` for
  dry runs, then delete the release).

## 5. macOS Signing / Notarization

**Out of scope for v0.10.0** (recorded as a question in Example Mapping).
Unsigned binaries:
- macOS: users get Gatekeeper warnings; `xattr -d com.apple.quarantine
  fspec` or right-click → Open works.
- Windows: SmartScreen warnings on unsigned .exe.

If signing is later required: add `Apple ID` + `APPLE_CERTIFICATE` secrets,
use `softprops/dotenv` + `codesign` + `notarytool` on the macOS leg, and
`signtool` + a code-signing cert on the Windows leg. Track as a follow-up
story.

## 6. Testing the Pipeline

1. **Local dry run**: `cargo build --profile release-slim -p codelet-fspec`
   on the dev machine (macOS arm64) — must succeed and produce a binary that
   runs (`./target/aarch64-apple-darwin/release-slim/fspec --version`).
2. **CI dry run**: push tag `v0.10.0-rc.1`. Verify all 5 build jobs pass and
   the release job creates a release with 5 assets. Delete the release + tag
   when done.
3. **Final**: push `v0.10.0`. Verify the release page shows all 5 assets.
4. **Smoke test each binary** on real hardware where possible (at minimum:
   the mac arm64 binary on the dev machine, one Linux binary in a container
   via `docker run --rm -v $PWD:/w -w /w <target-image> ./fspec --version`).

## 7. Definition of Done

- [ ] `rust/Cargo.toml` `[workspace.package] version = "0.10.0"`
- [ ] `Cargo.lock` updated
- [ ] `.github/workflows/release.yml` exists and passes on tag `v0.10.0`
- [ ] GitHub Release `v0.10.0` exists with exactly 5 assets named per §1
- [ ] `fspec --version` prints `fspec 0.10.0` on at least 2 platforms
- [ ] UPD-002's asset-matching assumption (names in §1) is confirmed against
      the live release via `curl https://api.github.com/repos/sengac/fspec/releases/latest`

## 8. Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| `windows-11-arm` runner unavailable | Fallback to `cargo xwin` from `windows-11` (documented in §3) |
| LTO build OOM on 2-core runners | `release-slim` inherits `codegen-units` from release profile; if OOM, set `CARGO_BUILD_JOBS=1` env on the failing leg |
| `release-slim` profile missing on a runner | Profile is in `rust/Cargo.toml` (checked in) — no runner-side config needed |
| Release re-run duplicates assets | `action-gh-release` replaces assets with the same name on re-run |
| Tag pushed before pipeline proven | Always dry-run with `-rc` tags first (§6.2) |
