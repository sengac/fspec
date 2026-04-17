@wip
@KGRAPH-027
Feature: Dead Code Detection via AST Graph — Calls/TypeRef Edge Population + Orphan Query
  """
  TS call extraction: Use ast-grep pattern `$CALLEE($$$ARGS)` on function bodies, filter to bare identifiers only (no dots/method calls), cross-reference with known function names from same file + imported names
  TS type reference extraction: Parse function signatures for type annotations (`param: TypeName`, `: ReturnType`), match against known Type nodes from same file + imported type names
  Dead code queries use nanograph `not { }` anti-join: `match { $f: File not { $other imports $f } }` for orphan files, similar for uncalled functions and unreferenced types. Client-side filtering removes test files and stubs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The TS extractor must detect function call expressions (e.g. `validateConfig()`, `parseArgs(x)`) within function bodies and emit Calls edges (Function→Function) for calls that resolve to functions in the same file
  #   2. The TS extractor must detect cross-file function calls by matching call names to imported identifiers — if file A imports `validateConfig` from './config' and calls `validateConfig()`, emit a Calls edge from A::callingFunction to config::validateConfig
  #   3. The TS extractor must detect type references in function signatures (parameter types, return types) and emit TypeRef edges (Function→Type) for types defined in the same file or imported
  #   4. Calls to non-function identifiers (e.g. constructors `new Foo()`, method chains `obj.method()`, builtins like `console.log()`) must NOT generate Calls edges — only direct named function calls that resolve to known Function nodes
  #   5. A new `ast_dead_code` GraphSearch action must use nanograph `not { }` anti-join queries to find: orphan files (no incoming Imports), uncalled functions (no incoming Calls), and unreferenced types (no incoming TypeRef/Extends/Implements)
  #   6. The ast_dead_code action must accept an optional `entity_type` filter (File, Function, Type) and exclude test files by default
  #   7. Scope: TypeScript extractor only. Rust extractor Calls/TypeRef population is deferred to KGRAPH-025
  #
  # EXAMPLES:
  #   1. File A has `function main() { validateConfig(); }` and `import { validateConfig } from './config'` → Calls edge from A::main to config::validateConfig
  #   2. File A has `function foo() { bar() }` and `function bar() {}` in same file → Calls edge from A::foo to A::bar (same-file call)
  #   3. Function `handler(req: Request): Response { ... }` referencing types Request and Response → TypeRef edges from handler to Request and Response
  #   4. File X is never imported by any other file → ast_dead_code returns X as orphan file
  #   5. Function `helperUnused()` exists but no other function calls it → ast_dead_code returns it as uncalled function
  #   6. Type `OldInterface` exists but no function references it in params or return type → ast_dead_code returns it as unreferenced type
  #   7. `console.log()`, `process.exit()`, `new Error()` do NOT generate Calls edges — these are builtins/constructors, not project function calls
  #   8. Test files should be excluded from dead code results by default — tests are leaf nodes, they call production code but nothing calls them
  #
  # ========================================
  Background: User Story
    As a AI agent
    I want to detect dead code (orphan files, uncalled functions, unreferenced types) via GraphSearch
    So that identify unused code and keep the codebase clean

  # ── Calls Edge Extraction ──────────────────────────────────────
  Scenario: Extract Calls edge for cross-file function call via import
    Given a TypeScript file "src/app.ts" with content:
      """
      import { validateConfig } from './config';
      function main() { validateConfig(); }
      """
    And a TypeScript file "src/config.ts" with content:
      """
      export function validateConfig() { return true; }
      """
    When the TS extractor processes both files
    Then a Calls edge should exist from "src-app-ts::main" to "src-config-ts::validateConfig"

  Scenario: Extract Calls edge for same-file function call
    Given a TypeScript file "src/utils.ts" with content:
      """
      function foo() { bar(); }
      function bar() { return 1; }
      """
    When the TS extractor processes the file
    Then a Calls edge should exist from "src-utils-ts::foo" to "src-utils-ts::bar"

  Scenario: Ignore method calls and builtins — no Calls edges for dotted expressions
    Given a TypeScript file "src/main.ts" with content:
      """
      function run() {
        console.log('hello');
        process.exit(0);
        obj.method();
      }
      """
    When the TS extractor processes the file
    Then no Calls edges should be emitted

  # ── TypeRef Edge Extraction ────────────────────────────────────
  Scenario: Extract TypeRef edges from function parameter and return types
    Given a TypeScript file "src/handler.ts" with content:
      """
      interface Request { url: string; }
      interface Response { status: number; }
      function handler(req: Request): Response { return { status: 200 }; }
      """
    When the TS extractor processes the file
    Then a TypeRef edge should exist from "src-handler-ts::handler" to "src-handler-ts::Request"
    And a TypeRef edge should exist from "src-handler-ts::handler" to "src-handler-ts::Response"

  # ── Dead Code Query: Orphan Files ──────────────────────────────
  Scenario: Detect orphan files with no incoming Imports edges
    Given a graph with File "src/used.ts" imported by "src/app.ts"
    And a graph with File "src/orphan.ts" imported by no other file
    When the ast_dead_code action runs with entity_type "File"
    Then the result should include "src/orphan.ts"
    And the result should not include "src/used.ts"

  # ── Dead Code Query: Uncalled Functions ────────────────────────
  Scenario: Detect uncalled functions with no incoming Calls edges
    Given a graph with Function "app::main" that calls "app::helper"
    And a graph with Function "app::unused" that is never called
    When the ast_dead_code action runs with entity_type "Function"
    Then the result should include "app::unused"
    And the result should not include "app::helper"

  # ── Dead Code Query: Unreferenced Types ────────────────────────
  Scenario: Detect unreferenced types with no incoming TypeRef edges
    Given a graph with Type "handler-ts::Request" referenced by "handler-ts::handler"
    And a graph with Type "handler-ts::OldInterface" referenced by no function
    When the ast_dead_code action runs with entity_type "Type"
    Then the result should include "handler-ts::OldInterface"
    And the result should not include "handler-ts::Request"

  # ── Filtering ──────────────────────────────────────────────────
  Scenario: Exclude test files from dead code results by default
    Given a graph with File "src/__tests__/app.test.ts" that is a test file
    And that test file is never imported by any other file
    When the ast_dead_code action runs with entity_type "File"
    Then the result should not include "src/__tests__/app.test.ts"
