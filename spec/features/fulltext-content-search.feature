@done
@KGRAPH-067
Feature: Full-Text and Content Search Within Graph

  """
  DRY approach: Reuse matches_fields() helper in dispatch_helpers.rs with different field lists per search_mode. Add decorator/parameter filter as post-match predicates in dispatch_ast_search. Extend dispatch_helpers.rs (field lists, matches_decorator, matches_parameter), ast_dispatch.rs (dispatch logic), and types.rs (new parameters).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ast_search searches configurable fields via search_mode — 'name' mode (default) searches name/slug/path/qualifiedName only, preventing noisy source code matches
  #   2. A new 'search_mode' parameter on ast_search controls which fields are searched: 'name' (name/slug/path/qualifiedName — default, backward compatible), 'content' (source/docstring only), 'all' (every field)
  #   3. A new 'decorator' filter parameter on ast_search does exact/contains matching against the decorators property — e.g. decorator='@app.route' returns only functions with that decorator (CGC: find_functions_by_decorator)
  #   4. A new 'parameter' filter parameter on ast_search does contains matching against the parameters property — e.g. parameter='request' returns functions that accept a parameter named 'request' (CGC: find_functions_by_argument)
  #   5. When search_mode is 'name' (default), source/docstring/parameters/decorators fields are NOT searched — prevents noisy matches when agents want to find a specific function by name
  #   6. decorator and parameter filters work as AND constraints combined with the query — query='User' with decorator='@Injectable' matches functions named User that also have the @Injectable decorator
  #   7. decorator matching is case-insensitive and strips leading @, #[, and trailing ] for matching — searching decorator='test' matches both '@test' and '@Test' and '#[test]'
  #
  # EXAMPLES:
  #   1. Agent searches query='dispatch' with search_mode='name' — returns functions named 'dispatch_ast_search', 'dispatch_ast_neighbors' but NOT functions that merely call dispatch in their body
  #   2. Agent searches query='authentication' with search_mode='content' — returns functions whose source code or docstrings contain 'authentication' even if their names don't
  #   3. Agent searches with decorator='@Test' — returns only Function nodes whose decorators property contains '@Test' or '@test' or '#[test]'
  #   4. Agent searches with parameter='request' — returns functions whose parameters string contains 'request' (e.g. 'self, request, response')
  #   5. Agent searches query='User' with decorator='@Injectable' and search_mode='name' — returns only functions named User that also have @Injectable decorator (AND logic)
  #   6. Agent uses ast_search with no search_mode parameter — defaults to 'name' mode for backward compatibility, only searches name/slug/path/qualifiedName
  #
  # ========================================

  Background: User Story
    As an AI agent
    I want to search graph nodes by source code content, docstrings, and names with controllable search scope
    So that I can find relevant code by what it does, not just by name, without noisy matches when I only want name-based results

  @search-mode
  Scenario: Name-only search excludes source code matches
    Given a project is indexed with functions that have source code stored
    When I search with query "dispatch" and search_mode "name"
    Then results include functions whose names contain "dispatch"
    And results do not include functions that only mention "dispatch" in their source code

  @search-mode
  Scenario: Content search finds matches in source code and docstrings
    Given a project is indexed with functions that have source code and docstrings stored
    When I search with query "authentication" and search_mode "content"
    Then results include functions whose source code contains "authentication"
    And results include functions whose docstrings contain "authentication"
    And results do not include functions that only match "authentication" in their name

  @search-mode
  Scenario: Default search mode is name-only for backward compatibility
    Given a project is indexed with functions that have metadata stored
    When I search with query "process" and no search_mode parameter
    Then results only include entities whose name, slug, path, or qualifiedName contains "process"

  @decorator-filter
  Scenario: Decorator filter returns functions with matching decorator
    Given a project is indexed with functions that have decorators stored
    When I search with decorator filter "Test"
    Then results include only functions whose decorators contain "Test"
    And decorator matching is case-insensitive

  @parameter-filter
  Scenario: Parameter filter returns functions with matching parameter name
    Given a project is indexed with functions that have parameters stored
    When I search with parameter filter "request"
    Then results include only functions whose parameters contain "request"

  @combined-filters
  Scenario: Query combined with decorator filter uses AND logic
    Given a project is indexed with functions that have names and decorators stored
    When I search with query "User" and decorator filter "Injectable" and search_mode "name"
    Then results include only functions named "User" that also have the "Injectable" decorator

  @search-mode
  Scenario: All search mode searches every field
    Given a project is indexed with functions that have full metadata
    When I search with query "validate" and search_mode "all"
    Then results include functions matching "validate" in name, source, docstring, parameters, or decorators

  @decorator-filter
  Scenario: Decorator matching strips leading symbols for cross-language matching
    Given a project is indexed with functions decorated with "@test" and "#[test]" and "@Test"
    When I search with decorator filter "test"
    Then results include all three functions regardless of decorator syntax prefix
