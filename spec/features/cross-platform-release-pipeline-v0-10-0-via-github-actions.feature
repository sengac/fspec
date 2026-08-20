@cli
@UPD-001 @ci @cross-platform @release-management @high
Feature: Cross-platform release pipeline (v0.10.0) via GitHub Actions

  """
  Native matrix build (one runner per target) over cross-compilation: windows-11 (x86_64 MSVC), windows-11-arm (aarch64 MSVC), ubuntu-24.04 (x86_64 Linux), ubuntu-24.04-arm (aarch64 Linux), macos-latest (aarch64). Full plan in attachment release-pipeline-implementation.md
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The pipeline MUST build exactly 5 targets: x86_64-pc-windows-msvc, aarch64-pc-windows-msvc, x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, aarch64-apple-darwin
  #   2. The workspace version in rust/Cargo.toml [workspace.package] MUST be bumped from 0.1.0 to 0.10.0 and Cargo.lock MUST be regenerated so all member crates report 0.10.0
  #   3. Release assets MUST be named fspec-<target-triple>.zip (Windows) or fspec-<target-triple>.tar.gz (unix) containing only the fspec binary — this naming is a contract with the UPD-002 self-updater
  #   4. Each target MUST be built natively on a matching GitHub runner (no cross-compilation in CI), using the release-slim cargo profile and the pinned 1.95.0 toolchain from rust/rust-toolchain.toml
  #   5. The release job MUST fail (and publish nothing) if any of the 5 build jobs fails
  #   6. The release workflow MUST trigger on git tags matching v* and create/update a GitHub Release with all 5 assets attached using softprops/action-gh-release
  #
  # EXAMPLES:
  #   1. Running fspec --version on the mac aarch64 release binary prints fspec 0.10.0
  #   2. Pushing tag v0.10.0 runs 5 parallel build jobs (one per target) and a final release job that publishes fspec v0.10.0 with exactly 5 assets on the GitHub Releases page
  #   3. A user on Windows ARM64 downloads fspec-aarch64-pc-windows-msvc.zip, extracts fspec.exe, and it runs without installing a Rust toolchain
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

  Scenario: Workspace version is bumped to 0.10.0
    Given the workspace is at version 0.1.0
    When the version bump for v0.10.0 is applied
    Then rust/Cargo.toml [workspace.package] version is 0.10.0
    And Cargo.lock reports 0.10.0 for all member crates

  Scenario: Release workflow builds all five targets
    Given a git tag matching v* is pushed
    When the release workflow runs
    Then one build job runs natively per target (win x86_64, win arm64, linux x86_64, linux arm64, mac arm64)
    And every build uses the release-slim profile and the pinned 1.95.0 toolchain

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
    Then it prints fspec 0.10.0

  Scenario: Windows ARM64 user runs the prebuilt binary
    Given a user on Windows ARM64 downloads fspec-aarch64-pc-windows-msvc.zip
    When they extract and run fspec.exe
    Then it works without installing a Rust toolchain
