@typescript
@prettier
@eslint
@code-quality
@cli
@critical
@CLEAN-001
Feature: Add ESLint and Prettier for code quality
  """

  Key architectural decisions:
  - Use mindstrike project (~/projects/mindstrike) ESLint and Prettier configuration as reference
  - Adapt mindstrike config for fspec CLI app (simpler than full-stack, no React/NestJS rules)
  - ESLint flat config (eslint.config.js) with TypeScript plugin
  - Prettier config (.prettierrc) matching mindstrike style (2-space tabs, single quotes, 80-char width)
  - eslint-plugin-prettier integration for unified linting/formatting

  Dependencies from mindstrike:
  - @eslint/js for base JavaScript rules
  - @typescript-eslint/eslint-plugin and @typescript-eslint/parser for TypeScript support
  - eslint-plugin-prettier for Prettier integration
  - Prettier v3+ for code formatting

  Critical implementation requirements:
  - MUST fix ALL existing violations before integration (zero errors/warnings)
  - MUST preserve existing code style (2-space indentation, single quotes)
  - MUST NOT break existing build or test scripts
  - MUST align with CLAUDE.md coding standards (no any, no require, etc.)
  - Config stored in project root (eslint.config.js, .prettierrc)
  - Add lint and lint:fix scripts to package.json

  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ESLint configuration MUST align with TypeScript strict mode and existing CLAUDE.md coding standards
  #   2. Prettier MUST be configured to match existing code formatting (2-space indentation, single quotes, etc.)
  #   3. All existing code violations MUST be fixed before integration (no warnings/errors in CI)
  #   4. Integration MUST NOT break existing npm scripts (build, test, dev)
  #   5. ESLint and Prettier MUST work together without conflicts (use eslint-config-prettier)
  #
  # EXAMPLES:
  #   1. Developer runs 'npm run lint' and all TypeScript files are checked with ESLint showing zero errors
  #   2. Developer runs 'npm run format' and all files are formatted consistently with Prettier
  #   3. Developer runs 'npm run build' and it completes successfully (lint doesn't break build)
  #   4. Developer commits code and pre-commit hook runs ESLint+Prettier automatically
  #   5. ESLint catches common TypeScript errors (unused vars, missing types, etc.) during development
  #   6. Prettier formats code on save in VSCode/IDE with consistent 2-space indentation and single quotes
  #
  # ========================================
  Background: User Story
    As a developer contributing to fspec
    I want to have ESLint and Prettier configured and running
    So that code quality is consistent and common errors are caught automatically

  Scenario: Run ESLint successfully with zero errors
    Given ESLint is configured in eslint.config.js
    And All existing code violations have been fixed
    Then All TypeScript files should be checked
    And Exit code should be 0
    And Output should show zero errors and zero warnings
    When Developer runs 'npm run lint'

  Scenario: Run Prettier successfully formatting all files
    Given Prettier is configured in .prettierrc
    And Code has mixed formatting styles
    Then All files should be formatted with 2-space indentation
    And Single quotes should be used consistently
    And Exit code should be 0
    When Developer runs 'npm run format'

  Scenario: Build completes successfully with linting integrated
    Given ESLint and Prettier are configured
    And All code passes quality checks
    Then TypeScript compilation should complete
    And No lint errors should be reported
    And Exit code should be 0
    When Developer runs 'npm run build'

  Scenario: ESLint catches common TypeScript errors during development
    Given ESLint is configured with TypeScript plugin
    And Code has unused variables and missing types
    Then ESLint should report unused variable errors
    And ESLint should report missing type errors
    And Exit code should be 1
    When Developer runs 'npm run lint'

  Scenario: Pre-commit hook runs ESLint and Prettier automatically
    And Developer has staged files
    When Developer runs git commit command
    Then ESLint should run on staged files
    And Prettier should format staged files
    And Commit should succeed if no errors
    Given Husky and lint-staged are configured
