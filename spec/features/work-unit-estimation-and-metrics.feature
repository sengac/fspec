@wip
@cli
@project-management
@estimation
@metrics
Feature: Work Unit Estimation and Metrics
  """
  Architecture notes:
  - Story points for AI-centric estimation (complexity, not time)
  - Track AI-specific metrics: tokens consumed, iterations
  - Compare estimated vs actual to improve future estimates
  - Cycle time tracking per workflow state
  - Historical metrics for pattern detection and learning

  Critical implementation requirements:
  - MUST support story point estimation (Fibonacci: 1,2,3,5,8,13,21)
  - MUST track actual tokens consumed during implementation
  - MUST record iteration count for each work unit
  - MUST compare estimated vs actual for learning
  - MUST track cycle time from stateHistory
  - MUST NOT implement velocity/burndown (sprint concepts, not ACDD)

  Metrics tracked:
  - estimate: Story points (complexity estimate)
  - actualTokens: Total tokens consumed during work
  - iterations: Number of AI iterations to complete
  - cycleTime: Time from backlog to done (calculated from stateHistory)

  Purpose:
  - Enable pattern detection (e.g., "5-point work = 110k tokens avg")
  - Improve estimation accuracy over time by prefix/epic
  - Identify bottlenecks in workflow states
  - NOT for velocity/burndown tracking (those are sprint-based, we use continuous flow)

  References:
  - Project Management Design: project-management.md (section 7: Estimation vs Actuals)
  """

  Background: User Story
    As an AI agent tracking work complexity
    I want to estimate and measure work units with AI-centric metrics
    So that I can improve future estimates and understand velocity

  @critical
  @happy-path
  Scenario: Assign story points to work unit
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    When I run "fspec update-work-unit AUTH-001 --estimate=5"
    Then the command should succeed
    And the work unit should have estimate of 5 story points

  @happy-path
  Scenario: Use Fibonacci sequence for estimates
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    When I run "fspec update-work-unit AUTH-001 --estimate=8"
    Then the command should succeed
    And the estimate should be valid Fibonacci number

  @validation
  @error-handling
  Scenario: Reject non-Fibonacci estimate
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    When I run "fspec update-work-unit AUTH-001 --estimate=7"
    Then the command should fail
    And the error should contain "Invalid estimate"
    And the error should suggest valid values: 1,2,3,5,8,13,21

  @metrics
  @tracking
  Scenario: Record tokens consumed during implementation
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "implementing"
    When I run "fspec record-metric AUTH-001 --tokens=45000"
    Then the command should succeed
    And the work unit should have actualTokens of 45000

  @metrics
  @query
  Scenario: Calculate cycle time from state history
    Given I have a project with spec directory
    And a work unit "AUTH-001" has stateHistory:
      | state        | timestamp            |
      | backlog      | 2025-01-15T10:00:00Z |
      | specifying   | 2025-01-15T11:00:00Z |
      | testing      | 2025-01-15T13:00:00Z |
      | implementing | 2025-01-15T14:00:00Z |
      | validating   | 2025-01-15T17:00:00Z |
      | done         | 2025-01-15T18:00:00Z |
    When I run "fspec query metrics AUTH-001"
    Then the output should show cycle time: "8 hours"
    And the output should show time per state

  @comparison
  @accuracy
  @critical
  Scenario: Compare estimate vs actual for completed work unit
    Given I have a project with spec directory
    And a completed work unit "AUTH-001" has:
      | estimate     | 5     |
      | actualTokens | 95000 |
      | iterations   | 2     |
      | status       | done  |
    When I run "fspec query estimate-accuracy AUTH-001"
    Then the output should show estimated: "5 points"
    And the output should show actual: "95000 tokens, 2 iterations"
    And the output should show comparison: "Within expected range"

  @pattern-detection
  @learning
  Scenario: Analyze estimate accuracy across all completed work
    Given I have a project with spec directory
    And completed work units:
      | id       | estimate | actualTokens | iterations |
      | AUTH-001 | 1        | 22000        | 1          |
      | AUTH-002 | 1        | 28000        | 2          |
      | AUTH-003 | 3        | 70000        | 2          |
      | AUTH-004 | 3        | 80000        | 3          |
      | AUTH-005 | 5        | 95000        | 2          |
    When I run "fspec query estimate-accuracy --output=json"
    Then the output should show average tokens per story point:
      | points | avgTokens | avgIterations | samples |
      | 1      | 25000     | 1.5           | 2       |
      | 3      | 75000     | 2.5           | 2       |
      | 5      | 95000     | 2.0           | 1       |

  @pattern-detection
  @by-prefix
  Scenario: Analyze estimate accuracy by prefix
    Given I have a project with spec directory
    And completed work units:
      | id       | prefix | estimate | actualTokens |
      | AUTH-001 | AUTH   | 5        | 95000        |
      | AUTH-002 | AUTH   | 3        | 70000        |
      | SEC-001  | SEC    | 5        | 140000       |
      | SEC-002  | SEC    | 3        | 95000        |
    When I run "fspec query estimate-accuracy --by-prefix --output=json"
    Then the output should show AUTH prefix:
      | avgAccuracy      | recommendation                |
      | estimates 5% low | estimates are well-calibrated |
    And the output should show SEC prefix:
      | avgAccuracy       | recommendation                   |
      | estimates 40% low | increase estimates by 2-3 points |

  @recommendations
  @learning
  Scenario: Get estimation recommendations for new work
    Given I have a project with spec directory
    And completed work units with established patterns
    When I run "fspec query estimation-guide"
    Then the output should show recommended patterns:
      | points | expectedTokens | expectedIterations | confidence |
      | 1      | 20k-30k        | 1-2                | high       |
      | 3      | 60k-90k        | 2-3                | high       |
      | 5      | 90k-130k       | 2-4                | medium     |

  Scenario: Update work unit with 5-point estimate
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    When I run "fspec update-work-unit AUTH-001 --estimate=5"
    Then the command should succeed
    And the work unit should have estimate of 5 story points
    And the estimate should be a valid Fibonacci number

  Scenario: Record 45k tokens consumed
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with status "implementing"
    When I run "fspec record-metric AUTH-001 --tokens=45000"
    Then the command should succeed
    And the work unit should have actualTokens of 45000
    And the metric should be tracked for future analysis

  @metrics
  @tracking
  Scenario: Increment iteration count
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with iterations 2
    When I run "fspec record-iteration AUTH-001"
    Then the command should succeed
    And the work unit should have iterations of 3
    And the iteration count should be incremented by 1

  Scenario: Query estimate accuracy for work unit
    Given I have a project with spec directory
    And a completed work unit "AUTH-001" with estimate 5
    And the work unit has actualTokens 95000
    And the work unit has iterations 2
    When I run "fspec query estimate-accuracy AUTH-001"
    Then the output should show estimated: "5 points"
    And the output should show actual: "95000 tokens, 2 iterations"
    And the output should show comparison and accuracy assessment

  Scenario: Get estimation guide with patterns
    Given I have a project with spec directory
    And completed work units with established patterns exist
    And historical data shows 1-point = 20k-30k tokens
    And historical data shows 3-point = 60k-90k tokens
    And historical data shows 5-point = 90k-130k tokens
    When I run "fspec query estimation-guide"
    Then the output should show recommended patterns by story points
    And the output should show expected token ranges
    And the output should show expected iteration counts
    And the output should show confidence levels for each pattern
