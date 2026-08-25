@done
@cli
@update
@cross-platform
@high
@UPD-004
Feature: Updater does not set executable permissions on the replaced binary (unix)
  """
  The fix lives in codelet-fspec-core::update::replace (extract_targz/extract_zip + replace_binary). The executable bit is applied to the extracted temp binary via std::fs::set_permissions (unix-only, #[cfg(unix)] with PermissionsExt::from_mode(0o755)) BEFORE the atomic rename, because rename preserves the source file's permissions. Windows is unaffected: the self_replace path does not use Unix permission bits. The test points the engine at a local mock GitHub API (axum on 127.0.0.1:0) via the base_url override — redirect, don't intercept.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The executable permission MUST be applied to the extracted temp binary before the atomic rename, so the rename lands an already-executable file into place (rename preserves source permissions; chmod-ing the install path after rename would briefly leave a non-executable window)
  #   2. On Unix (macOS/Linux), after a successful in-place update the installed binary MUST be executable (owner/user/other execute bits set, i.e. mode 0o755) — the kernel refuses to exec a non-+x file, so a non-executable install bricked the CLI until a manual chmod
  #
  # EXAMPLES:
  #   1. Regression case (the reported bug): the extracted temp binary was created with default umask (0o644) and std::fs::rename preserved that mode, so the installed binary was non-executable and the next fspec invocation failed with 'permission denied'
  #   2. Engine on v0.9.3 with v0.10.0 latest: after perform_update the installed fspec binary has mode 0o755 (executable) — the next fspec invocation runs without 'permission denied'
  #
  # ========================================
  Background: User Story
    As a fspec user on macOS or Linux
    I want to self-update fspec in place
    So that the installed binary remains executable after the update

  @UPD-004
  @unix
  Scenario: Engine installs an executable binary on Unix
    Given the engine is configured at an older version
    And a newer release exists with an asset for the current platform
    When the engine performs an update
    Then it reports the new version
    And the installed binary is executable
