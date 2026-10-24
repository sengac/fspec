@phase-1
@persistence
@session-management
@bug-fix
Feature: Recover original messages omitted from a saved index
  """
  The binary index is a derived cache, not evidence that an original is missing.
  A lookup miss or identity mismatch rebuilds it once per file generation under
  the shared message lock. Cleanup verifies required originals before rewriting
  history and refuses deletion when rebuilding cannot recover those originals.
  """

  Scenario: Lookup recovers an original omitted from an apparently current index
    Given an intact original omitted from an index recording the full history size
    When a live reader requests that original
    Then it rebuilds the index and returns the original identity and content
    And a cold reader can read the recovered original

  Scenario: Lookup repairs an index entry pointing at a different original
    Given a saved index mapping a required original to another message's bytes
    When a reader requests the required original
    Then it returns the required original rather than the unrelated message

  Scenario: Cleanup preserves referenced originals omitted from the index
    Given a referenced original omitted from an apparently current index
    And an indexed orphan exists
    When cleanup retains the referenced original
    Then cleanup removes only the orphan
    And the referenced original remains available after cold reopening

  Scenario: Cleanup refuses deletion when a required original is truly missing
    Given an indexed orphan and a required original absent from history
    When cleanup retains the missing original
    Then cleanup reports the missing original without rewriting history
    And the orphan remains available for later recovery
