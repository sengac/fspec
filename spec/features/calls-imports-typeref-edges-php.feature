@KGRAPH-042
Feature: Calls/Imports/TypeRef edges — PHP
  """
  Uses KindMatcher for `namespace_use_declaration` to find imports, `function_call_expression`/`member_call_expression` for calls.
  PHP import resolution uses PSR-4 namespace-to-path mapping (backslash to forward-slash + .php).
  The extractor accepts known_files for import resolution and filters external namespaces.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. PHP `use Namespace\Class;` statements must produce Imports edges from source file to the resolved target file, using PSR-4 namespace-to-path mapping
  #   2. Same-file method calls via `$this->method()` and bare function calls must produce Calls edges between functions in the same file
  #   3. Type annotations in PHP function signatures (parameter types and return types) must produce TypeRef edges from the function to the type, but only for project-local types
  #   4. External `use Psr\Http\Message\*` and similar third-party namespace imports must NOT produce edges
  #
  # EXAMPLES:
  #   1. `use Slim\Routing\RouteResolver;` in App.php with RouteResolver.php existing → Imports edge from App-php to Routing-RouteResolver-php
  #   2. `$this->getRouteResolver()` inside `addRoutingMiddleware()` → Calls edge from addRoutingMiddleware to getRouteResolver (same file)
  #   3. `function handle(AppRequest $req): AppResponse` where AppRequest/AppResponse are local types → TypeRef edges from handle to both types
  #
  # ========================================
  Background: User Story
    As a developer
    I want to get Imports, Calls, and TypeRef edges extracted from PHP source files
    So that dead code detection works for PHP projects via ast_dead_code

  Scenario: Extract Imports edges from PHP use statements
    Given a PHP file with `use Slim\Routing\RouteResolver;` namespace import
    And the target file `Slim/Routing/RouteResolver.php` exists in the project
    When the PHP extractor processes the source file
    Then an Imports edge should be emitted from the source file to the target file
    And external `use Psr\Http\Message\*` imports should NOT produce edges

  Scenario: Extract Calls edges from PHP same-file method calls
    Given a PHP file with methods `addRoutingMiddleware()` and `getRouteResolver()`
    And `addRoutingMiddleware()` contains `$this->getRouteResolver()`
    When the PHP extractor processes the source file
    Then a Calls edge should be emitted from `addRoutingMiddleware` to `getRouteResolver`

  Scenario: Extract TypeRef edges from PHP type-annotated signatures
    Given a PHP file with `public function handle(AppRequest $request): AppResponse`
    And types `AppRequest` and `AppResponse` are defined in local project files
    When the PHP extractor processes the source file
    Then TypeRef edges should be emitted from `handle` to `AppRequest` and `AppResponse`
    And external types not in the project should NOT produce TypeRef edges
