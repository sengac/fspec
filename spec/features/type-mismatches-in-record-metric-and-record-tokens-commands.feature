@cli
@phase-1
@bug-fix
@metrics
@refactoring
@BUG-047
Feature: Type mismatches in record-metric and record-tokens commands
  """
  Architecture notes:
  - Delete record-metric.ts entirely (duplicate of record-tokens)
  - Delete record-metric-help.ts help file
  - Remove all references from command registration (src/index.ts, src/help.ts)
  - Update all documentation and test files to remove record-metric
  - Add operation parameter to record-tokens function signature
  - Both functions use fileManager.transaction() for safe concurrent access
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. record-metric command accepts <metric> <value> arguments but function signature expects workUnitId and tokens
  #   2. record-tokens command passes 'operation' option but function signature doesn't accept it
  #   3. Function signatures must match their command action calls exactly
  #   4. TypeScript compilation passes due to index signature [key: string]: unknown allowing extra properties
  #   5. record-metric command accepts <metric> <value> arguments but function signature expects workUnitId and tokens
  #   6. record-tokens command passes 'operation' option but function signature doesn't accept it
  #   7. Function signatures must match their command action calls exactly
  #   8. TypeScript compilation passes due to index signature [key: string]: unknown allowing extra properties
  #
  # EXAMPLES:
  #   1. record-metric.ts: Function expects {workUnitId, tokens, cwd} but action calls with {metric, value, unit}
  #   2. record-metric.ts line 63-66: Passes 'metric', 'value', 'unit' but function doesn't accept these parameters
  #   3. record-tokens.ts: Function accepts {workUnitId, tokens, cwd} but action passes 'operation' which is not in signature
  #   4. record-tokens.ts line 76: Passes 'operation' option but function signature at line 18-22 doesn't have operation parameter
  #   5. record-metric.ts: Function expects {workUnitId, tokens, cwd} but action calls with {metric, value, unit}
  #   6. record-metric.ts line 63-66: Passes 'metric', 'value', 'unit' but function doesn't accept these parameters
  #   7. record-tokens.ts: Function accepts {workUnitId, tokens, cwd} but action passes 'operation' which is not in signature
  #   8. record-tokens.ts line 76: Passes 'operation' option but function signature at line 18-22 doesn't have operation parameter
  #
  # QUESTIONS (ANSWERED):
  #   Q: Looking at record-metric.ts, it seems there are TWO conflicting purposes: (1) The function implementation records token usage on work units, but (2) The command arguments suggest it should record arbitrary metrics (metric name, value, unit). Which behavior is intended? Should we align the function to match the command (generic metrics), or align the command to match the function (token tracking)?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. The function implementation in record-metric.ts actually records token usage on work units (identical to record-tokens). The command arguments suggesting generic metrics (metric name, value, unit) are incorrect. We should align the command to match the function: it should accept workUnitId and tokens, just like record-tokens does. The 'record-metric' command appears to be a duplicate/misnamed version of 'record-tokens' and should either be removed or fixed to match its actual implementation.
  #
  # ========================================
  Background: User Story
    As a fspec developer
    I want to fix type mismatches in record-metric and record-tokens commands
    So that commands work correctly with proper type safety

  Scenario: Delete duplicate record-metric command
    Given record-metric.ts exists with identical implementation to record-tokens
    And record-metric has incorrect command signature (<metric> <value>)
    When I delete src/commands/record-metric.ts
    And I delete src/commands/record-metric-help.ts
    Then the duplicate command should be removed

  Scenario: Remove record-metric from command registration
    Given record-metric is registered in src/index.ts
    When I remove the recordMetric import
    And I remove the registerRecordMetricCommand call
    Then record-metric command should no longer be available

  Scenario: Update documentation to remove record-metric references
    Given record-metric is referenced in feature files
    And record-metric is referenced in test files
    When I update work-unit-estimation-and-metrics.feature to use record-tokens
    And I update cli-command-registration.feature to remove record-metric
    And I update test files to remove record-metric tests
    Then all documentation should only reference record-tokens

  Scenario: Remove record-metric from coverage files
    Given record-metric is tracked in coverage files
    When I update work-unit-estimation-and-metrics.feature.coverage
    Then coverage should only track record-tokens

  Scenario: Add operation parameter to record-tokens function
    Given record-tokens command passes operation option
    But recordTokens function doesn't accept operation parameter
    When I add operation?: string to function signature
    Then command and function signatures should match
