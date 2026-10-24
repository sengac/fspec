@BUG-213
@phase-1
@agent-core
@compaction
@context-management
Feature: Generated compaction reports activity without a synthetic work total
  """
  Architecture notes:
  The truthful-compaction-activity lifecycle contract applies. GenerateCompaction
  acquires activity ownership on the original target and publishes its reached
  phase with unknown current=0,total=0 counts, not a one-operation denominator.
  Exercise self and cross-target calls against an actual pending local provider.
  This specification maps only to truthful_compaction_generate_bug213.rs.
  """

  Scenario Outline: Generated compaction reports unknown work for its actual target
    Given a <target> compaction target without measured completed-work units
    When GenerateCompaction reaches its real pending builder
    Then its target activity reports the phase actually reached and unknown work counts
    And dropping the operation clears activity without a committed generation

    Examples:
      | target |
      | self   |
      | other  |
