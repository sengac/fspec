@done
@CMPCT-082
@agent-core
@context-management
@compaction
@phase-1
Feature: Core admission hooks reject inaccessible authority
  """
  This core integration target supports the existing fail-closed admission
  contract. It exercises the real hook before a request and at response finish
  with coherent and inconsistent usage. Physical-send proof stays in the
  provider-owned feature; this hook regression does not replace that acceptance.
  """

  Scenario: Inaccessible admission authority cancels every hook phase
    Given a core streaming hook whose admission state lock is inaccessible
    When request and response finish hooks check coherent and inconsistent usage
    Then every hook phase cancels instead of authorizing inference
