@done
@codelet
@infrastructure
@ci
@napi
@cross-platform
@NAPI-007
Feature: GitHub Actions CI/CD for codelet-napi cross-platform builds

  """
  Architecture Notes:

  FILES TO CREATE:
  - .github/workflows/build-codelet-napi.yml (main CI/CD workflow)

  FILES TO MODIFY:
  - codelet/napi/package.json:
    - Change "name" from "codelet-napi" to "@sengac/codelet-napi"
    - Set "private": false
    - Run "napi prepublish -t npm" to generate optionalDependencies
  - package.json (root fspec):
    - Change "codelet-napi": "file:codelet/napi" to "@sengac/codelet-napi": "^0.1.0"

  NPM PACKAGE STRUCTURE (7 packages total):
  - @sengac/codelet-napi (main package with JS wrapper + optionalDependencies)
  - @sengac/codelet-napi-darwin-arm64 (macOS Apple Silicon)
  - @sengac/codelet-napi-darwin-x64 (macOS Intel)
  - @sengac/codelet-napi-linux-x64-gnu (Linux x64)
  - @sengac/codelet-napi-linux-arm64-gnu (Linux ARM64)
  - @sengac/codelet-napi-win32-x64-msvc (Windows x64)
  - @sengac/codelet-napi-win32-arm64-msvc (Windows ARM64)

  WORKFLOW STRUCTURE:
  - Triggers: push/PR to codelet/**, tags codelet-napi-v*
  - Jobs:
    1. build (matrix: 6 platforms) → uploads .node artifacts
    2. test (matrix: macOS/Linux/Windows) → downloads artifacts, runs smoke test
    3. publish (on version tags only) → downloads all artifacts, runs napi prepublish, npm publish

  BUILD MATRIX:
  | Target                      | Runner          | Method              |
  |----------------------------|-----------------|---------------------|
  | aarch64-apple-darwin       | macos-14        | Native (M1/M2)      |
  | x86_64-apple-darwin        | macos-13        | Native (Intel)      |
  | x86_64-unknown-linux-gnu   | ubuntu-latest   | Native              |
  | aarch64-unknown-linux-gnu  | ubuntu-latest   | Docker + QEMU       |
  | x86_64-pc-windows-msvc     | windows-latest  | Native              |
  | aarch64-pc-windows-msvc    | windows-latest  | Cross-compile       |

  CRITICAL REQUIREMENTS:
  - Must include codelet/patches/rig-core in checkout (patched dependency)
  - Build from codelet/ directory (Cargo workspace root)
  - Cache Cargo registry/git/target for faster builds
  - Use NAPI-RS v3 (@napi-rs/cli ^3.5.0) for all operations

  SECRETS REQUIRED:
  - NPM_TOKEN: npm access token with publish rights to @sengac scope

  ARTIFACT FLOW:
  build job → upload codelet-napi.{platform}.node per target
  test job → download artifacts → verify import works
  publish job → download all artifacts → napi artifacts → napi prepublish -t npm → npm publish
  """

  Background: User Story
    As a maintainer of @sengac/fspec
    I want automated cross-platform builds and npm publishing
    So that users on any platform can install fspec without compiling Rust

  # ============================================================
  # WORKFLOW CONFIGURATION SCENARIOS
  # ============================================================

  @workflow @triggers
  Scenario: Workflow triggers on codelet directory changes
    Given the file .github/workflows/build-codelet-napi.yml exists
    When a push or PR modifies any file in "codelet/**"
    Then the workflow should trigger
    And build all 6 platform targets

  @workflow @triggers
  Scenario: Workflow triggers npm publish on version tags
    Given the file .github/workflows/build-codelet-napi.yml exists
    When a tag matching "codelet-napi-v*" is pushed
    Then the workflow should trigger build job
    And trigger test job after build succeeds
    And trigger publish job after test succeeds

  # ============================================================
  # BUILD JOB SCENARIOS
  # ============================================================

  @build @native
  Scenario: Native builds for x64 platforms
    Given the build job runs with matrix strategy
    When building for x86_64-apple-darwin
    Then it should run on macos-13 runner
    And use "napi build --platform --release --target x86_64-apple-darwin"
    And upload artifact "codelet-napi.darwin-x64.node"

  @build @native
  Scenario: Native build for Apple Silicon
    Given the build job runs with matrix strategy
    When building for aarch64-apple-darwin
    Then it should run on macos-14 runner (M1/M2)
    And use "napi build --platform --release --target aarch64-apple-darwin"
    And upload artifact "codelet-napi.darwin-arm64.node"

  @build @cross-compile
  Scenario: Cross-compilation for Linux ARM64
    Given the build job runs with matrix strategy
    When building for aarch64-unknown-linux-gnu
    Then it should run on ubuntu-latest runner
    And use Docker image "ghcr.io/napi-rs/napi-rs/nodejs-rust:lts-debian-aarch64"
    And upload artifact "codelet-napi.linux-arm64-gnu.node"

  @build @cross-compile
  Scenario: Cross-compilation for Windows ARM64
    Given the build job runs with matrix strategy
    When building for aarch64-pc-windows-msvc
    Then it should run on windows-latest runner
    And install aarch64-pc-windows-msvc Rust target
    And use "napi build --platform --release --target aarch64-pc-windows-msvc"
    And upload artifact "codelet-napi.win32-arm64-msvc.node"

  @build @workspace
  Scenario: Build includes patched rig-core dependency
    Given the checkout includes codelet/patches/rig-core
    And codelet/Cargo.toml has [patch.crates-io] for rig-core
    When the build runs for any platform target
    Then it should compile using the patched rig-core
    And the build should succeed

  # ============================================================
  # TEST JOB SCENARIOS
  # ============================================================

  @test @smoke
  Scenario: Smoke test verifies binary loads correctly
    Given the test job downloads build artifacts
    And Node.js 20 is installed
    When running a smoke test that imports codelet-napi
    Then the CodeletSession class should be accessible
    And no native module loading errors should occur

  # ============================================================
  # PUBLISH JOB SCENARIOS
  # ============================================================

  @publish @npm
  Scenario: Publish generates platform-specific packages
    Given all 6 build artifacts are downloaded
    And NPM_TOKEN secret is configured
    When the publish job runs "napi prepublish -t npm"
    Then it should generate 7 npm packages:
      | Package Name                          |
      | @sengac/codelet-napi                  |
      | @sengac/codelet-napi-darwin-arm64     |
      | @sengac/codelet-napi-darwin-x64       |
      | @sengac/codelet-napi-linux-x64-gnu    |
      | @sengac/codelet-napi-linux-arm64-gnu  |
      | @sengac/codelet-napi-win32-x64-msvc   |
      | @sengac/codelet-napi-win32-arm64-msvc |

  @publish @npm
  Scenario: Main package has correct optionalDependencies
    Given "napi prepublish -t npm" has been run
    When inspecting @sengac/codelet-napi package.json
    Then optionalDependencies should include all 6 platform packages
    And each platform package should have the same version

  # ============================================================
  # END-USER EXPERIENCE SCENARIOS (Acceptance Criteria)
  # ============================================================

  @acceptance @install
  Scenario Outline: Users on any platform get correct binary automatically
    Given @sengac/codelet-napi is published to npm with all platform packages
    And I am on a <platform> system
    When I run "npm install -g @sengac/fspec"
    Then npm should automatically install <platform_package>
    And the fspec CLI should work without Rust compilation

    Examples:
      | platform           | platform_package                       |
      | macOS ARM64        | @sengac/codelet-napi-darwin-arm64      |
      | macOS Intel        | @sengac/codelet-napi-darwin-x64        |
      | Linux x64          | @sengac/codelet-napi-linux-x64-gnu     |
      | Linux ARM64        | @sengac/codelet-napi-linux-arm64-gnu   |
      | Windows x64        | @sengac/codelet-napi-win32-x64-msvc    |
      | Windows ARM64      | @sengac/codelet-napi-win32-arm64-msvc  |