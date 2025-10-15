@done
@configuration
@hooks
@phase1
@HOOK-002
Feature: Hook configuration schema and validation

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # USER STORY:
  #   As a fspec developer
  #   I want to define hook configuration in spec/fspec-hooks.json
  #   So that the hooks system knows which scripts to execute at which events
  #
  # BUSINESS RULES:
  #   1. Configuration file is spec/fspec-hooks.json in JSON format
  #   2. Each hook has: name (string), command (path to executable), blocking (boolean, default false), timeout (number in seconds, default 60)
  #   3. Hooks can have optional condition object with: tags (array), prefix (array), epic (string), estimateMin (number), estimateMax (number)
  #   4. Configuration must be valid JSON and conform to hooks schema
  #   5. Hook command paths must point to executable files
  #   6. Global defaults can be set in 'global' object: timeout, shell
  #
  # EXAMPLES:
  #   1. Valid config with single hook: {hooks: {'post-implementing': [{name: 'lint', command: 'spec/hooks/lint.sh', blocking: true}]}}
  #   2. Hook with timeout override: {name: 'e2e-tests', command: 'test.sh', timeout: 300}
  #   3. Hook with conditions: {name: 'security', command: 'audit.sh', condition: {tags: ['@security'], prefix: ['AUTH']}}
  #   4. Invalid JSON causes validation error with helpful message
  #   5. Non-existent hook command path causes validation error
  #   6. Missing hook file returns clear error: 'Hook command not found: spec/hooks/missing.sh'
  #
  # ========================================
  Background: User Story
    As a fspec developer
    I want to define hook configuration in spec/fspec-hooks.json
    So that the hooks system knows which scripts to execute at which events

  Scenario: Load valid hook configuration with single hook
    Given I have a file "spec/fspec-hooks.json" with content:
      """
      {
        "hooks": {
          "post-implementing": [
            {
              "name": "lint",
              "command": "spec/hooks/lint.sh",
              "blocking": true
            }
          ]
        }
      }
      """
    When I load the hook configuration
    Then the configuration should be valid
    And the hook "lint" should be registered for event "post-implementing"
    And the hook should have blocking set to true
    And the hook should have timeout set to 60 (default)

  Scenario: Load hook configuration with timeout override
    Given I have a file "spec/fspec-hooks.json" with content:
      """
      {
        "hooks": {
          "post-testing": [
            {
              "name": "e2e-tests",
              "command": "test.sh",
              "timeout": 300
            }
          ]
        }
      }
      """
    When I load the hook configuration
    Then the hook "e2e-tests" should have timeout set to 300

  Scenario: Load hook configuration with conditions
    Given I have a file "spec/fspec-hooks.json" with content:
      """
      {
        "hooks": {
          "post-implementing": [
            {
              "name": "security",
              "command": "audit.sh",
              "condition": {
                "tags": ["@security"],
                "prefix": ["AUTH"]
              }
            }
          ]
        }
      }
      """
    When I load the hook configuration
    Then the hook "security" should have condition tags set to ["@security"]
    And the hook "security" should have condition prefix set to ["AUTH"]

  Scenario: Reject invalid JSON configuration
    Given I have a file "spec/fspec-hooks.json" with invalid JSON content
    When I try to load the hook configuration
    Then an error should be thrown
    And the error message should contain "Invalid JSON"
    And the error message should be helpful

  Scenario: Reject configuration with non-existent hook command
    Given I have a file "spec/fspec-hooks.json" with content:
      """
      {
        "hooks": {
          "post-implementing": [
            {
              "name": "missing",
              "command": "spec/hooks/missing.sh"
            }
          ]
        }
      }
      """
    And the file "spec/hooks/missing.sh" does not exist
    When I try to load the hook configuration
    Then an error should be thrown
    And the error message should contain "Hook command not found: spec/hooks/missing.sh"

  Scenario: Load configuration with global defaults
    Given I have a file "spec/fspec-hooks.json" with content:
      """
      {
        "global": {
          "timeout": 120,
          "shell": "/bin/bash"
        },
        "hooks": {
          "post-implementing": [
            {
              "name": "lint",
              "command": "spec/hooks/lint.sh"
            }
          ]
        }
      }
      """
    When I load the hook configuration
    Then the global timeout should be set to 120
    And the global shell should be set to "/bin/bash"
    And the hook "lint" should use the global timeout of 120
