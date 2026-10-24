@done
@agent-core
@session-management
@persistence
@BUG-246
Feature: Admit recovery originals only under verified session ownership
  """
  Shares the core RecoveryLease and BUG-246's bounded owner. Only an explicitly
  absent committed manifest permits retirement; unreadable evidence preserves
  exact originals for repair. Ownership order is recovery owner, recovery lease,
  then session ownership. Deleted-session admission never re-retains a record.
  """

  Scenario: Deleted sessions refuse renewed recovery admission
    Given two live sessions with independently retained recovery originals
    When one session is deleted and another original is submitted to its recovery owner
    Then admission is refused without recreating its pending lane or changing the other owner

  Scenario: Unreadable manifests preserve exact unacknowledged originals
    Given a recovery original retained in memory and an unreadable committed manifest
    When retry and a new recovery admission inspect that manifest
    Then neither operation acknowledges or discards the retained original
    And repairing the manifest recovers only that authentic original once
