@done
@rust
@parser
@code-analysis
@KGRAPH-074
Feature: Single-pass AST fact extraction, data-driven per language
  """
  Engine: ast_pipeline::facts parses with ast-grep-language's tree-sitter grammars (already in the lock) and walks Node::kind/field/children once per file. Language tables are data (a static table per language, no per-language code paths), and parallel parsing uses std::thread::scope, so no dependency changes. Facts are per-file and resolution-free, which KGRAPH-076 caches by content hash. The edges built from facts (Calls, Imports, TypeRef, Extends/Implements, Type->method) belong to KGRAPH-075's global resolve pass. Landing: KGRAPH-074 and KGRAPH-075 are built together in a private worktree and land as one compiling commit, so the shared tree never has a graph without call edges between the two cards.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. One engine extracts every supported language: each file is read once, parsed once and its syntax tree is walked once. What to collect comes from a declared table per language: file extensions, grammar, definition node kinds with their entity kind and name field, container node kinds, call node kinds with the callee field, import node kinds, base-type fields, test markers, decision node kinds, doc-comment and decorator node kinds, and variable node kinds. There are no ast-grep pattern strings, no scanning of body text and no letter-case or other heuristics; the 14 per-language extractors are removed (no legacy path)
  #   2. Every node of a declared definition kind becomes exactly one definition fact; for Rust that includes generic, where-clause, const, unsafe, async and extern fns, methods in impl and trait blocks, trait method signatures and nested fns
  #   3. A definition's identity is (normalized path, container chain, name, kind, start byte): same-named definitions in different containers, and overloads, are distinct nodes. qualifiedName is the path followed by the container chain and the name (path::Type::method). A File is keyed by its normalized path, so distinct paths never share a key
  #   4. Paths are relative to the root and use / on every platform
  #   5. A call-site fact records its innermost enclosing definition and the callee as written: a plain name, a scoped path with its segments, or a receiver with a method name. A call inside a nested definition belongs only to the nested definition. Text the grammar does not parse into calls (macro token trees) yields no call facts
  #   6. Test status comes from exact facts declared per language: test attributes and decorators on a definition or an enclosing container (Rust #[test], #[tokio::test], #[cfg(test)] mod) and per-language test path conventions for files (Rust tests/ directories, Go *_test.go, Python test_*.py and *_test.py, JS/TS *.test.* and *.spec.*); a path merely containing 'test' is not a test
  #   7. Cyclomatic complexity of a definition is 1 plus the number of declared decision nodes inside it, not counting nodes inside nested definitions
  #   8. Facts are sorted by path, then start byte; files are parsed in parallel and the output is byte-identical whatever the number of threads
  #   9. The walk does not follow symbolic links: a symlinked file is not parsed and is reported in the index result's files_skipped as {path, reason: symlink}
  #   10. Product owner, 2026-11-07: .h files are always parsed with the C++ grammar.
  #   11. Product owner, 2026-11-07: the file size cap is 1 MiB (1,048,576 bytes); a larger file is not read into the parser and is listed in the index result's files_skipped as {path, reason: too_large}.
  #
  # EXAMPLES:
  #   1. A Rust file holds fn generic<T>(), fn with_where<T>() where T: Clone, const fn c(), unsafe fn u(), async fn a() and extern "C" fn e(); ast_search lists all six
  #   2. A Rust file holds impl A { fn new() } and impl B { fn new() }; ast_search finds two functions named new, with qualifiedNames src/lib.rs::A::new and src/lib.rs::B::new, and B::new's callees are not attributed to A::new
  #   3. Files src/a-b.rs, src/a/b.rs and src/a.b.rs each hold fn f; the graph has three File nodes and three functions named f, one per path
  #   4. fn outer() { fn inner() { helper() } inner() }: the call to helper belongs to inner only, and outer's call site is inner
  #   5. In Rust, self.run(), Type::new() and module::f() each produce a call-site fact (receiver self with method run; scoped path Type::new; scoped path module::f)
  #   6. src/latest.rs is not a test file; a function marked #[test] inside #[cfg(test)] mod tests in src/lib.rs is a test
  #   7. For a multi-language fixture, the definition facts per declared node kind equal the count of nodes of that kind in each file's syntax tree
  #   8. Indexing the same fixture with 1 thread and with 8 threads gives byte-identical facts
  #   9. src/shape.h holds class Shape { public: virtual double area(); }; it is parsed with the C++ grammar, so ast_search lists class Shape and its method area
  #   10. A .rs file of exactly 1,048,576 bytes is parsed; one of 1,048,577 bytes is not parsed and files_skipped lists {path, reason: too_large}
  #   11. src/link.rs is a symbolic link to src/real.rs; only src/real.rs is parsed and files_skipped lists {path: src/link.rs, reason: symlink}
  #
  # QUESTIONS (ANSWERED):
  #   Q: .h files: which grammar declares them? Options: (a) always C, (b) always C++, (c) a per-root setting. Today the code guesses from the content, which the card's no-heuristics rule removes. My recommendation is (b), because the C++ grammar parses the C headers fspec sees.
  #   A: Product owner, 2026-11-07: .h files are always parsed with the C++ grammar.
  #
  #   Q: B22 asks for a file-size cap, with oversized files reported in files_skipped. What is the cap? My recommendation is 1 MiB, matching common minified-bundle sizes.
  #   A: Product owner, 2026-11-07: the file size cap is 1 MiB (1,048,576 bytes); a larger file is not read into the parser and is listed in the index result's files_skipped as {path, reason: too_large}.
  #
  # ========================================
  Background: User Story
    As a agent using GraphSearch on a project in any supported language
    I want to get a code graph whose definitions and call sites come from one exact pass over each file's syntax tree
    So that every function, method and type is in the graph under its own identity, indexing takes seconds instead of tens of minutes, and a new language is a declaration rather than new extractor code

  Scenario: Every kind of Rust fn is a definition
    Given a Rust file holding fn generic<T>(), fn with_where<T>() where T: Clone, const fn c(), unsafe fn u(), async fn a() and extern "C" fn e()
    When ast_index runs for the project
    And ast_search lists the project's functions
    Then the results hold generic, with_where, c, u, a and e

  Scenario: Same-named methods of different impl blocks are distinct functions
    Given src/lib.rs holding impl A { fn new() } and impl B { fn new() }, where B::new calls fn helper
    When ast_index runs for the project
    And ast_search lists the functions named new
    Then the results hold two functions with qualifiedNames src/lib.rs::A::new and src/lib.rs::B::new
    And ast_callees of B::new is helper and ast_callees of A::new is empty

  Scenario: Files whose paths differ only in separators and dots stay distinct
    Given files src/a-b.rs, src/a/b.rs and src/a.b.rs that each hold fn f
    When ast_index runs for the project
    Then the graph holds three File nodes with those three paths
    And three functions named f, one contained in each file

  Scenario: A call inside a nested function belongs only to the nested function
    Given fn outer() { fn inner() { helper() } inner() } and fn helper()
    When the facts of the file are extracted
    Then the call site helper belongs to inner only
    And the call site of outer is inner

  Scenario: Method calls and scoped calls are call-site facts
    Given a Rust function whose body calls self.run(), Type::new() and module::f()
    When the facts of the file are extracted
    Then its call sites are the method run on receiver self, the scoped path Type::new and the scoped path module::f

  Scenario: Test status comes from test attributes and test path conventions
    Given src/latest.rs holding fn latest() and src/lib.rs holding #[cfg(test)] mod tests { #[test] fn checks() {} }
    When the facts of the files are extracted
    Then src/latest.rs is not a test file and latest is not a test
    And checks is a test

  Scenario: Every node of a declared definition kind becomes one definition fact
    Given a multi-language fixture with Rust, Python, TypeScript, Go and Java files
    When the facts of the files are extracted
    Then for each file and each declared definition node kind, the number of definition facts equals the number of nodes of that kind in the file's syntax tree

  Scenario: Facts are the same whatever the number of threads
    Given a multi-language fixture
    When its facts are extracted with 1 thread and with 8 threads
    Then the two serialized fact lists are byte-identical
    And the facts are sorted by path, then start byte

  Scenario: A .h header is parsed with the C++ grammar
    Given src/shape.h holding class Shape { public: virtual double area(); };
    When ast_index runs for the project
    Then ast_search lists the class Shape and its method area

  Scenario: A file larger than 1 MiB is skipped and reported
    Given a .rs file of exactly 1,048,576 bytes and a .rs file of 1,048,577 bytes
    When ast_index runs for the project
    Then the first file is parsed
    And files_skipped lists the second file with reason too_large

  Scenario: A symbolic link is not followed and is reported
    Given src/real.rs and a symbolic link src/link.rs to it
    When ast_index runs for the project
    Then only src/real.rs is parsed
    And files_skipped lists src/link.rs with reason symlink
