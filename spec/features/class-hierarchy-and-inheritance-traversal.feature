@done
@KGRAPH-064
Feature: Class Hierarchy and Inheritance Traversal
  """
  Add AstHierarchy variant to GraphSearchAction enum. Implement in ast_hierarchy.rs using existing type_extends, type_implements, and type_container .gq queries with iterative multi-level BFS traversal. Uses type_extended_by and type_implemented_by reverse queries for children. Methods are approximated by finding all functions in the same file as the type via type_container and file_functions queries.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. New action_type 'ast_hierarchy' accepts a type slug and returns parents (via Extends edges), children (via reverse Extends), and implemented interfaces (via Implements edges)
  #   2. Methods are approximated as all functions in the same file as the type, found via type_container → file_functions queries (the schema lacks direct Type→Function containment edges)
  #   3. Traversal supports multi-level hierarchies (grandparent → parent → child) via iterative BFS over Extends edges with a configurable max depth (default 3)
  #   4. Already has Extends and Implements edges populated by extractors — this leverages existing type_extends and type_implements .gq queries plus type_extended_by reverse query
  #   5. The _include_methods parameter is accepted but ignored — methods are always included in the response
  #
  # EXAMPLES:
  #   1. Agent asks ast_hierarchy for 'Dog' — returns parent classes (Animal), child classes (GuideDog), interfaces (Trainable), and functions from the same file
  #   2. Agent asks ast_hierarchy for a type with no parents or children — returns the type itself with functions from its file, empty parents and children arrays
  #
  # ========================================
  Background: User Story
    As an AI agent
    I want to traverse class inheritance hierarchies to find parent classes, child classes, and method overrides
    So that I can understand type relationships and navigate OOP codebases effectively

  @happy-path
  Scenario: Find class hierarchy with parents and children
    Given I have a codebase indexed with class inheritance relationships
    When I request ast_hierarchy for a class with parent and child classes
    Then I should receive the parent classes via Extends edges
    And I should receive the child classes via reverse Extends edges
    And I should receive implemented interfaces via Implements edges
    And the response should include methods from the type's containing file

  @happy-path
  Scenario: Multi-level hierarchy traversal
    Given I have a codebase indexed with a 3-level class hierarchy
    When I request ast_hierarchy for the middle class
    Then I should receive grandparent classes in the parents array
    And I should receive grandchild classes in the children array

  @edge-case
  Scenario: Standalone type with no inheritance
    Given I have a codebase indexed with a class that has no parents or children
    When I request ast_hierarchy for that class
    Then I should receive the type itself with methods from its containing file
    And the parents array should be empty
    And the children array should be empty

  @error
  Scenario: Non-existent type returns error
    Given I have a codebase indexed in the AST graph
    When I request ast_hierarchy for a non-existent type slug
    Then I should receive an error indicating the type was not found
