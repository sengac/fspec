@KGRAPH-017
Feature: AST Extraction Pipeline — Tree-Sitter/AST-Grep Parser

  """
  Pipeline module in its own file (ast_pipeline.rs) with per-language extractors in separate files (ast_ts_extractor.rs, ast_rust_extractor.rs)
  Uses ast_grep_core and ast_grep_language crates directly. Uses ignore::WalkBuilder for gitignore-aware walking. Produces GraphEntity values loaded via registry::get_graph(AST_CODE_GRAPH).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The pipeline must use ast-grep (already in codelet) for parsing — NOT tree-sitter directly
  #   2. Must produce GraphEntity values (nodes and edges) compatible with database.rs load_entities()
  #   3. Must support TypeScript/JavaScript parsing: imports, function declarations, class/interface/type declarations, method calls
  #   4. Must support Rust parsing: fn/struct/enum/trait/impl declarations, use statements, function calls
  #   5. Must batch-collect all entities from a file before loading to avoid Lance version amplification
  #   6. Must respect .gitignore and skip node_modules, target, dist, .git directories
  #   7. Each extractor (TS extractor, Rust extractor) must be in its own file for separation of concerns
  #   8. Must use the AstGrep tool's pattern matching to find functions/types/imports — no raw tree-sitter API calls
  #
  # EXAMPLES:
  #   1. Parse a TypeScript file and extract Function nodes with name, qualified name, isAsync, isPublic, paramCount, lineStart, lineEnd
  #   2. Parse a Rust file and extract Function nodes and struct/enum/trait Type nodes with correct qualified names
  #   3. Parse import statements from TypeScript files and produce File→Imports→File edges
  #   4. Walk a project directory respecting .gitignore and extract all entities in batch, loading them into the AST graph in a single operation
  #
  # ========================================

  Background: User Story
    As an AI agent
    I want to have the codebase automatically parsed into the AST graph with function, type, and import relationships extracted
    So that I can query code structure without reading every file, and find call chains, impact analysis paths, and dependency trees

  Scenario: Extract Function nodes from a TypeScript file
    Given a TypeScript file "src/auth/login.ts" with async and sync function declarations
    When the TypeScript extractor parses the file
    Then Function nodes should be created with correct name, qualifiedName, isAsync, isPublic properties
    And each Function node should have lineStart and lineEnd positions
    And each Function node should have a paramCount matching the declaration
    And a File node should be created for "src/auth/login.ts"
    And Contains edges should link the File to each Function

  Scenario: Extract Type nodes from a Rust file
    Given a Rust file "src/graph/database.rs" with struct, enum, and trait declarations
    When the Rust extractor parses the file
    Then Type nodes should be created for each struct with typeKind "struct_kind"
    And Type nodes should be created for each enum with typeKind "enum_kind"
    And Type nodes should be created for each trait with typeKind "trait_kind"
    And Function nodes should be created for each fn declaration
    And a File node should be created for "src/graph/database.rs"

  Scenario: Extract import edges from TypeScript files
    Given a TypeScript file "src/auth/login.ts" that imports from "src/auth/utils.ts" and "src/config.ts"
    When the TypeScript extractor parses the file
    Then File nodes should be created for all three files
    And Imports edges should link "src/auth/login.ts" to "src/auth/utils.ts"
    And Imports edges should link "src/auth/login.ts" to "src/config.ts"
    And each Imports edge should have the importPath property set

  Scenario: Walk project directory with gitignore and batch load
    Given a project directory with TypeScript and Rust source files
    And a .gitignore file that excludes "node_modules" and "target"
    When the extraction pipeline walks the project directory
    Then files in node_modules should be skipped
    And files in target should be skipped
    And all extracted entities should be loaded in a single batch operation
    And the AST graph should contain File, Function, and Type nodes from the processed files
