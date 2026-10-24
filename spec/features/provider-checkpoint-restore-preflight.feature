@done
@CMPCT-078
@CMPCT-080
@providers
@provider-abstraction
@compaction
@phase-1
Feature: Validate provider checkpoint replay without changing live authority
  """
  Restore preflight checks complete retained checkpoint envelopes before callers
  install a durable boundary or alter runtime tokens. Shared extension handles
  must not be used as a mutable validation sandbox. Invalid replay remains an
  explicit error; preflight itself changes no checkpoint, writer or readiness.
  """

  Scenario: Invalid retained checkpoint preflight leaves live authority usable
    Given a usable installed Codex checkpoint with a persistence observer
    When retained replay is preflighted with malformed or incompatible checkpoint metadata
    Then each invalid replay returns an error without changing the checkpoint or invoking persistence
    And the original request remains preparable and an authentic retry validates

  Scenario: Prospective committed boundary preflight does not install the boundary
    Given an existing Codex checkpoint and a newer committed boundary
    When full retained replay is validated against the prospective boundary
    Then valid replay succeeds and incompatible retained identity fails
    And the original checkpoint and request authority remain unchanged
