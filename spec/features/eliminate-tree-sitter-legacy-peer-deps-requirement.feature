@done
@npm
@dependency-management
@dependencies
@high
@DEP-003
Feature: Eliminate tree-sitter legacy-peer-deps requirement

  """
  Uses npm overrides field to force tree-sitter@0.25.0 resolution for incompatible parsers. Overrides apply to 9 parsers: tree-sitter-c, tree-sitter-cpp, tree-sitter-java, tree-sitter-typescript, tree-sitter-c-sharp, tree-sitter-json, tree-sitter-kotlin, tree-sitter-php, tree-sitter-ruby. Runtime compatibility verified through actual parser testing (C, C++, Java, TypeScript parsers tested and confirmed working). No external dependencies required (uses native npm 8.3.0+ feature).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. npm install must succeed without --legacy-peer-deps flag
  #   2. All tree-sitter language parsers must be compatible with tree-sitter@0.25.0
  #   3. Solution must use npm native features (no external patching tools)
  #   4. No manual forking or republishing of tree-sitter packages allowed
  #   5. Runtime compatibility must be verified with actual parser tests before deployment
  #
  # EXAMPLES:
  #   1. Developer runs 'npm install' in fresh clone, installation succeeds without errors or warnings
  #   2. Developer runs 'npm install' after deleting node_modules and package-lock.json, all tree-sitter packages resolve to tree-sitter@0.25.0
  #   3. AST parser uses tree-sitter-c to parse C code, parsing succeeds with tree-sitter@0.25.0
  #   4. AST parser uses tree-sitter-typescript to parse TypeScript code, parsing succeeds with tree-sitter@0.25.0
  #   5. Developer runs 'npm list tree-sitter' and sees only version 0.25.0 (no duplicate versions)
  #
  # ========================================

  Background: User Story
    As a developer installing fspec
    I want to install npm dependencies without legacy-peer-deps flag
    So that I follow npm best practices and avoid peer dependency conflicts

  Scenario: Clean npm install succeeds without legacy-peer-deps flag
    Given I have a fresh clone of the fspec repository
    When I run npm install without any flags
    Then the installation should complete successfully
    Given the package.json contains npm overrides for tree-sitter parsers
    Then no ERESOLVE errors should be displayed
    Then no peer dependency warnings should be shown


  Scenario: All tree-sitter packages resolve to version 0.25.0
    Given I have successfully run npm install
    When I run npm list tree-sitter
    Then the output should show only tree-sitter@0.25.0
    Then no duplicate tree-sitter versions should exist in node_modules


  Scenario: Tree-sitter C parser works with tree-sitter 0.25.0
    Given tree-sitter-c is installed with tree-sitter@0.25.0
    When I use the AST parser to parse C code
    Then the parsing should succeed without errors
    Then the AST should be correctly generated


  Scenario: Tree-sitter TypeScript parser works with tree-sitter 0.25.0
    Given tree-sitter-typescript is installed with tree-sitter@0.25.0
    When I use the AST parser to parse TypeScript code
    Then the parsing should succeed without errors
    Then the AST should be correctly generated

