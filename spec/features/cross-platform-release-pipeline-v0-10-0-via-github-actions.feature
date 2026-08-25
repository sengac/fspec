@cli
@UPD-001
@ci
@cross-platform
@release-management
@high
Feature: Cross-platform release pipeline (v0.10.0) via GitHub Actions
  """
  Cross-compilation pipeline (no Windows runners): Windows targets built with cargo-xwin on ubuntu-24.04, Linux targets built with cargo-zigbuild (TARGET_GLIBC_VERSION=2.17) on ubuntu-24.04, macOS target built natively on macos-latest. All use the release-slim profile and pinned 1.95.0 toolchain.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The pipeline MUST build exactly 5 targets: x86_64-pc-windows-msvc, aarch64-pc-windows-msvc, x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, aarch64-apple-darwin
  #   2. The workspace version in rust/Cargo.toml [workspace.package] MUST be bumped from 0.1.0 to 0.10.0 and Cargo.lock MUST be regenerated so all member crates report 0.10.0
  #   3. Release assets MUST be named fspec-<target-triple>.zip (Windows) or fspec-<target-triple>.tar.gz (unix) containing only the fspec binary — this naming is a contract with the UPD-002 self-updater
  #   4. Windows targets MUST be cross-compiled with cargo-xwin on an ubuntu-24.04 runner (no Windows runners). Linux targets MUST be cross-compiled with cargo-zigbuild on an ubuntu-24.04 runner with TARGET_GLIBC_VERSION=2.17 so the binary runs on any x86_64/aarch64 Linux distro with glibc >= 2.17. The macOS target MUST be built natively on macos-latest. All builds use the release-slim cargo profile and the pinned 1.95.0 toolchain from rust/rust-toolchain.toml
  #   5. The release job MUST fail (and publish nothing) if any of the 5 build jobs fails
  #   6. The release workflow MUST trigger on git tags matching v* and create/update a GitHub Release with all 5 assets attached using softprops/action-gh-release
  #   7. Every build job MUST verify its archive is valid (unzip -t for .zip, tar -tzf for .tar.gz) before uploading, and the release job MUST re-verify all archives before publishing — a corrupt or mis-typed archive (e.g. a tar archive with a .zip extension) MUST fail the pipeline
  #
  # EXAMPLES:
  #   1. Running fspec --version on the mac aarch64 release binary prints fspec 0.10.1
  #   2. Pushing tag v0.10.1 runs 5 parallel build jobs (one per target) and a final release job that publishes fspec v0.10.1 with exactly 5 assets on the GitHub Releases page
  #   3. A user on Windows ARM64 downloads fspec-aarch64-pc-windows-msvc.zip, extracts fspec.exe, and it runs without installing a Rust toolchain
  #   4. A user on CentOS 7 (glibc 2.17) or any x86_64 Linux distro runs the fspec-x86_64-unknown-linux-gnu binary without "version GLIBC_2.39 not found" errors
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should v0.10.0 release binaries be code-signed (macOS notarization + Windows code-signing cert), or is unsigned acceptable for now (Gatekeeper/SmartScreen warnings)?
  #   A: Unsigned for now. Ship unsigned v0.10.0 binaries (Gatekeeper/SmartScreen warnings accepted); code signing (macOS notarization + Windows cert) is tracked as a follow-up story.
  #
  # ASSUMPTIONS:
  #   1. Unsigned for now. Ship unsigned v0.10.0 binaries (Gatekeeper/SmartScreen warnings accepted); code signing (macOS notarization + Windows cert) is tracked as a follow-up story.
  #
  # ========================================
  Background: User Story
    As a fspec user on any supported platform
    I want to get a prebuilt fspec binary from GitHub Releases
    So that install without building from source

  Scenario: Workspace version is bumped to 0.10.1
    Given the workspace is at version 0.1.0
    When the version bump for v0.10.1 is applied
    Then rust/Cargo.toml [workspace.package] version is 0.10.1
    And Cargo.lock reports 0.10.1 for all member crates

  Scenario: Release workflow builds all five targets
    Given a git tag matching v* is pushed
    When the release workflow runs
    Then one build job runs per target (win x86_64, win arm64, linux x86_64, linux arm64, mac arm64)
    And Windows targets are cross-compiled with cargo-xwin on ubuntu-24.04
    And Linux targets are cross-compiled with cargo-zigbuild on ubuntu-24.04 with TARGET_GLIBC_VERSION=2.17
    And the macOS target is built natively on macos-latest
    And every build uses the release-slim profile and the pinned 1.95.0 toolchain

  Scenario: Linux binary runs on old glibc distros
    Given the fspec-x86_64-unknown-linux-gnu release asset is downloaded
    When the binary is run on a system with glibc 2.17
    Then it runs without "version GLIBC_2.39 not found" or similar symbol version errors

  Scenario: Windows archive is a valid zip
    Given the fspec-x86_64-pc-windows-msvc release asset is downloaded
    When unzip -t is run on the archive
    Then it reports no errors and contains fspec.exe

  Scenario: Release is published with self-updater-compatible assets
    Given all five build jobs succeeded
    When the release job completes
    Then a GitHub Release exists with exactly five assets
    And Windows assets are named fspec-<target>.zip and unix assets fspec-<target>.tar.gz

  Scenario: Release job is blocked when any build fails
    Given one of the five build jobs fails
    When the workflow reaches the release stage
    Then no GitHub Release is created or updated

  Scenario: Release binary reports the new version
    Given the mac aarch64 release binary is extracted
    When fspec --version is run
    Then it prints fspec 0.10.1

  Scenario: Windows ARM64 user runs the prebuilt binary
    Given a user on Windows ARM64 downloads fspec-aarch64-pc-windows-msvc.zip
    When they extract and run fspec.exe
    Then it works without installing a Rust toolchain

  Scenario: Workspace version is bumped to 0.10.1
    Given the workspace is at version 0.1.0
    When the version bump for v0.10.1 is applied
    Then rust/Cargo.toml [workspace.package] version is 0.10.1
    And Cargo.lock reports 0.10.1 for all member crates

  Scenario: Release workflow builds all five targets
    Given a git tag matching v* is pushed
    When the release workflow runs
    Then one build job runs per target (win x86_64, win arm64, linux x86_64, linux arm64, mac arm64)
    And Windows targets are cross-compiled with cargo-xwin on ubuntu-24.04
    And Linux targets are cross-compiled with cargo-zigbuild on ubuntu-24.04 with TARGET_GLIBC_VERSION=2.17
    And the macOS target is built natively on macos-latest
    And every build uses the release-slim profile and the pinned 1.95.0 toolchain

  Scenario: Linux binary runs on old glibc distros
    Given the fspec-x86_64-unknown-linux-gnu release asset is downloaded
    When the binary is run on a system with glibc 2.17
    Then it runs without "version GLIBC_2.39 not found" or similar symbol version errors

  Scenario: Windows archive is a valid zip
    Given the fspec-x86_64-pc-windows-msvc release asset is downloaded
    When unzip -t is run on the archive
    Then it reports no errors and contains fspec.exe

  Scenario: Release is published with self-updater-compatible assets
    Given all five build jobs succeeded
    When the release job completes
    Then a GitHub Release exists with exactly five assets
    And Windows assets are named fspec-<target>.zip and unix assets fspec-<target>.tar.gz

  Scenario: Release job is blocked when any build fails
    Given one of the five build jobs fails
    When the workflow reaches the release stage
    Then no GitHub Release is created or updated

  Scenario: Release binary reports the new version
    Given the mac aarch64 release binary is extracted
    When fspec --version is run
    Then it prints fspec 0.10.1

  Scenario: Windows ARM64 user runs the prebuilt binary
    Given a user on Windows ARM64 downloads fspec-aarch64-pc-windows-msvc.zip
    When they extract and run fspec.exe
    Then it works without installing a Rust toolchain
