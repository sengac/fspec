@done
@cli
@installer
@cross-platform
@release-management
@high
@UPD-003
Feature: GitHub-release binary installer (install.sh) + source build installer (build-install.sh)
  """
  scripts/install.sh is a pure download installer (vtcode-style): platform detection via uname, release resolution via GitHub API (releases?per_page=10, walk newest→oldest with HEAD checks, fallback to releases/latest redirect), download to mktemp -d, sha256 verification against the release's checksums.txt asset (warn+skip if absent), tar extraction, install to $INSTALL_DIR (default ~/.local/bin), PATH hint, and fspec --version verification. It requires only curl, tar, and a sha256 tool. scripts/build-install.sh preserves the existing from-source build flow (cargo/git prereq checks, cargo build --profile release-slim -p codelet-fspec, install). The release pipeline gains a checksums.txt asset (sha256sum format) published alongside the 5 binary archives. Asset filenames follow the UPD-002 self-updater contract: fspec-<target-triple>.tar.gz / fspec-<target-triple>.zip, downloaded from /releases/download/<tag>/ (tag keeps the v prefix; no version segment in the filename).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. scripts/install.sh MUST be a pure download installer: it detects the host platform, finds the latest GitHub release with a matching asset, downloads it, verifies its checksum, extracts the fspec binary, and installs it to the install directory. It MUST NOT require a Rust toolchain, cargo, or git, and MUST NOT build anything from source.
  #   2. scripts/build-install.sh MUST contain the existing from-source build behavior currently in scripts/install.sh: prerequisite checks (cargo, git, protoc warning), cargo build of codelet-fspec with the release-slim profile, and installation of the built binary. It MUST support the same --dir, --profile, and INSTALL_DIR/BUILD_PROFILE/BUILD_JOBS options.
  #   3. install.sh MUST detect the host platform via uname (Darwin/arm64, Darwin/x86_64, Linux/x86_64, Linux/aarch64) and map it to the release asset target triple: aarch64-apple-darwin, x86_64-apple-darwin, x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu. Unsupported OS or architecture MUST fail with a clear error message.
  #   4. install.sh MUST resolve the release to install by listing recent releases (GitHub API, up to 10) and picking the most recent one that has an asset matching the detected platform. Asset filenames MUST follow the UPD-002 contract: fspec-<target-triple>.tar.gz for unix, downloaded from the tag URL https://github.com/sengac/fspec/releases/download/<tag>/fspec-<target-triple>.tar.gz. If the API is unavailable, it MUST fall back to following the releases/latest redirect to get the tag.
  #   5. install.sh MUST verify the downloaded archive against the release's checksums.txt asset (sha256sum format: hash, two spaces, filename). If checksums.txt is not present on the release, it MUST print a warning and continue without verification rather than failing. A checksum mismatch MUST abort the installation with an error showing expected and actual hashes.
  #   6. install.sh MUST install the extracted binary to $INSTALL_DIR (default ~/.local/bin), making it executable, and MUST verify the installation by running fspec --version. If the install directory is not on PATH, it MUST print instructions for adding it to the user's shell config.
  #   7. The release pipeline (.github/workflows/release.yml) MUST publish a checksums.txt asset alongside the 5 binary archives. checksums.txt MUST contain one line per archive in sha256sum format (64-hex-char hash, two spaces, filename) so install.sh and install.ps1 can verify integrity.
  #   8. install.sh MUST work when piped (curl ... | bash) as well as when run from a file, and MUST support a --dir <path> option (and INSTALL_DIR environment variable) to override the default install directory, plus --help for usage.
  #   9. The README quick-start MUST document the one-line curl install command for macOS/Linux and the PowerShell command for Windows, and MUST reference build-install.sh for the from-source build path.
  #
  # EXAMPLES:
  #   1. A user on Apple Silicon Mac runs: curl -fsSL https://raw.githubusercontent.com/sengac/fspec/main/scripts/install.sh | bash — the script detects aarch64-apple-darwin, downloads the matching tar.gz from the latest release, verifies the checksum, installs fspec to ~/.local/bin/fspec, and prints the version. No Rust toolchain is needed.
  #   2. A user on an x86_64 Linux distro runs the installer — it detects x86_64-unknown-linux-gnu, downloads fspec-x86_64-unknown-linux-gnu.tar.gz (the static-musl asset), and the installed binary runs without glibc version errors.
  #   3. A developer who wants a from-source build runs scripts/build-install.sh — it checks for cargo and git, builds codelet-fspec with the release-slim profile, and installs the built binary to ~/.local/bin/fspec exactly as the old install.sh did.
  #   4. A user on a 32-bit or otherwise unsupported Linux architecture runs the installer — it fails immediately with a clear error naming the unsupported architecture and does not download anything.
  #   5. A release without a checksums.txt asset is installed — the script prints a warning that checksum verification was skipped and completes the installation successfully.
  #
  # ========================================
  Background: User Story
    As a fspec user on macOS or Linux
    I want to install fspec with a one-line curl command
    So that get a verified prebuilt binary in seconds without a Rust toolchain, while developers can still build from source via build-install.sh

  Scenario: One-line curl install on Apple Silicon Mac
    Given a user on an aarch64 macOS host
    When they run curl -fsSL https://raw.githubusercontent.com/sengac/fspec/main/scripts/install.sh | bash
    Then the script detects the platform aarch64-apple-darwin
    And it downloads fspec-aarch64-apple-darwin.tar.gz from the latest release with that asset
    And it verifies the archive against the release checksums.txt
    And it installs the fspec binary to ~/.local/bin/fspec
    And it prints the installed version from fspec --version
    And no Rust toolchain, cargo, or git is required

  Scenario: Install on x86_64 Linux
    Given a user on an x86_64 Linux host
    When they run scripts/install.sh
    Then the script detects the platform x86_64-unknown-linux-gnu
    And it downloads fspec-x86_64-unknown-linux-gnu.tar.gz from the latest release
    And the installed binary runs without glibc version errors

  Scenario: From-source build via build-install.sh
    Given a developer with cargo and git installed
    When they run scripts/build-install.sh
    Then it checks the cargo and git prerequisites
    And it builds codelet-fspec with the release-slim profile
    And it installs the built binary to ~/.local/bin/fspec

  Scenario: Unsupported architecture fails fast
    Given a user on an unsupported Linux architecture
    When they run scripts/install.sh
    Then it exits with a non-zero status
    And it prints an error naming the unsupported architecture
    And it downloads nothing

  Scenario: Missing checksums.txt warns but succeeds
    Given a release that has no checksums.txt asset
    When the installer downloads and installs the archive
    Then it prints a warning that checksum verification was skipped
    And the installation completes successfully

  Scenario: Checksum mismatch aborts installation
    Given a release whose checksums.txt does not match the downloaded archive
    When the installer verifies the archive
    Then it aborts with a non-zero exit
    And it prints the expected and actual hashes
    And no binary is installed

  Scenario: Install directory override via --dir
    Given the user wants to install to /usr/local/bin
    When they run scripts/install.sh --dir /usr/local/bin
    Then the fspec binary is installed to /usr/local/bin/fspec

  Scenario: Install directory not on PATH prints instructions
    Given the install directory is not on the user PATH
    When the installer completes
    Then it prints instructions for adding the install directory to the shell config

  Scenario: Release pipeline publishes checksums.txt
    Given a git tag matching v* is pushed
    When the release workflow completes
    Then the GitHub Release contains the 5 binary archives
    And it contains a checksums.txt asset
    And checksums.txt has one sha256sum-format line per archive (hash, two spaces, filename)

  Scenario: README documents the one-line install
    Given the README quick-start section
    When it is read
    Then it shows the curl one-line install command for macOS and Linux
    And it shows the PowerShell install command for Windows
    And it references build-install.sh for the from-source build path
