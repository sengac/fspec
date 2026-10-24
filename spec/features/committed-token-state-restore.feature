@done
@session
@compaction-resume
@phase-1
@CMPCT-062
Feature: Committed token state restore
  Restored context occupancy and cumulative spend remain distinct.

  """
  The persisted manifest is the authority for reasoning detail and saved
  counters. Legacy restore DTO counters must exactly agree with that manifest
  and remain nonnegative and representable in emitted counters. Context uses
  the saved inclusive input measurement; cumulative billing is not added to
  context occupancy. Restoration never increments billing or estimates missing
  values. Incompatible history generations or counters fail before publication.
  """

  Scenario: Saved context spend cache and reasoning become live state
    Given a committed manifest with distinct saved context spend cache and reasoning
    When a fresh session restores the matching saved counters
    Then live tracking cached fields and the emitted token update match the manifest

  Scenario: Invalid saved counters cannot publish restored success
    Given a committed manifest and incompatible or negative restore counters
    When token restoration is requested
    Then restoration fails without changing live counters or emitting a token update

  Scenario: Repeated token restoration does not accumulate billing
    Given a committed manifest with saved cumulative billing
    When matching token state is restored twice
    Then billing and context remain equal to one restore
