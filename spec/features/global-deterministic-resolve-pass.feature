@done
@rust
@parser
@code-analysis
@KGRAPH-075
Feature: Global deterministic resolve pass with exact Rust module resolution
  """
  Resolve: ast_pipeline::resolve runs once per index over the sorted facts of every file and is pure and in-memory. Rust: Cargo.toml manifests found by the walk give crates and their dependency names (workspace = true follows [workspace.dependencies]); mod declarations and #[path] give the module tree; use trees give per-module name bindings; resolution follows bindings to definitions and stops at the first name that is not under the root. Other languages declare an import strategy in their table. Unresolved references are counted per kind and reason in the index result. Schema: HasMethod: Type -> Function is added, Module is removed, Calls keeps callCount and isConditional (now populated). KGRAPH-074 and KGRAPH-075 land as one compiling commit.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. After the facts of every file exist, one in-memory resolve pass builds every Imports, Calls, TypeRef, Implements, Extends and HasMethod edge from the facts of the whole tree. No edge is carried over from an earlier index, and the same tree always gives a byte-identical entity list
  #   2. A reference becomes an edge only when the exact language rules name exactly one definition. A reference that names none, more than one, or something outside the root is not an edge: it is counted as unresolved with a reason (external, not_found, ambiguous, receiver_type_unknown, macro_token_tree, not_in_module_tree). The index result reports unresolved counts per reference kind (calls, imports, type_refs, bases) and per reason
  #   3. Rust crates come from the Cargo.toml files under the root: a [package] manifest is a crate whose library root is [lib] path or src/lib.rs, whose binary roots are each [[bin]] path, src/main.rs and src/bin/*.rs, and whose name is [lib] name or the package name with - replaced by _. A [workspace] manifest only lists members
  #   4. The Rust module tree comes from exact facts: mod x; in a crate root or mod.rs names x.rs or x/mod.rs in the same directory, in any other file f.rs it names f/x.rs or f/x/mod.rs, #[path = "..."] names the file relative to the declaring file's directory, and an inline mod x { } is a module inside its file. A file that is no crate's module has no module path and its references are not_in_module_tree
  #   5. Rust paths resolve as the compiler does for names declared in source: crate::, self::, super::, the crate's own name, a dependency crate declared in its Cargo.toml (path or workspace dependency, with package renames), items declared in the module, and use declarations (grouped, nested, renamed with as, glob, and pub use re-exports followed to their definition). A dependency outside the root, std, core and alloc are external
  #   6. Rust call sites: a plain name or a scoped path resolves to the function it names; Type::f and Self::f resolve to the method f of an impl block of that type in the crate; self.f() resolves to the method f of the enclosing impl's self type. An inherent method wins over trait methods; when only trait impls of the type define f and more than one does, the call is ambiguous. Any other receiver is receiver_type_unknown. Every identifier or path followed by a parenthesized token tree inside a macro invocation is one macro_token_tree unresolved call
  #   7. Rust impl Trait for Type gives Implements Type -> Trait and trait A: B gives Extends A -> B; the bases of a class in other languages come from the declared base-type fields of its language table. A function defined in the body of an impl, trait, class or interface has a HasMethod edge from its type, so ast_hierarchy methods are exactly those functions
  #   8. Other languages resolve imports with the strategy declared in their language table (relative file with declared extensions and index files, dotted module path, package and type name, Go module path) and resolve calls to the definitions in scope: the enclosing containers, the same file, imported names and the same package, never by name across the whole tree
  #   9. A Calls edge carries callCount, the number of call sites from the caller to the callee, and isConditional, true when every one of those call sites is inside a declared decision node of the caller. The never-populated Module node is removed from the schema
  #   10. TypeRef edges come from the type names inside a function's declared parameter and return-type fields (refKind param or return_type), each resolved like any other path
  #
  # EXAMPLES:
  #   1. A Cargo workspace with crates app and util, where app depends on util by a workspace path dependency: app's fn main calls util::parse(), and ast_callers of parse lists main
  #   2. A crate with src/lib.rs (mod a;), src/a.rs (mod b;) and src/a/b.rs (pub fn deep()): src/lib.rs calls a::b::deep() and crate::a::b::deep(), giving one Calls edge with callCount 2
  #   3. use crate::a::{b::deep, other as renamed}; then deep() and renamed() resolve to src/a/b.rs deep and src/a.rs other
  #   4. src/a/b.rs calls super::sibling() and self::local(), resolving to src/a.rs sibling and src/a/b.rs local
  #   5. pub use inner::Thing; in src/lib.rs re-exports Thing, and crate::Thing::new() in another module resolves to the new in impl Thing inside src/inner.rs
  #   6. impl Engine { fn start(&self) { self.run() } fn run(&self) {} }: start calls run, and Engine has HasMethod edges to start and run only
  #   7. Two impls of traits P and Q for Engine both define fn go; self.go() inside another Engine method is unresolved with reason ambiguous and no Calls edge exists
  #   8. value.len() on a local variable is unresolved with reason receiver_type_unknown; std::mem::swap(a, b) is unresolved with reason external
  #   9. assert_eq!(compute(1), 2) inside a test counts one macro_token_tree unresolved call and creates no Calls edge
  #   10. A .rs file under the root that no crate's mod tree reaches has its calls counted as not_in_module_tree
  #   11. impl Display for Engine gives Implements Engine -> Display only when Display is a trait defined under the root; trait Child: Parent gives Extends Child -> Parent
  #   12. Python class Dog(Animal) with Animal imported by from .animals import Animal gives Extends Dog -> Animal; TypeScript class A extends B implements I gives Extends A -> B and Implements A -> I
  #   13. Python from .helpers import tidy then tidy() resolves to helpers.py tidy; TypeScript import { tidy } from './helpers' then tidy() resolves to helpers.ts tidy
  #   14. if ready { go() } as the only call to go from a caller gives isConditional true; a call outside any decision node gives false
  #   15. Indexing the same fixture twice, and once more after deleting and recreating its graph, gives byte-identical entity lists and identical unresolved counts
  #
  # ========================================
  Background: User Story
    As a agent using GraphSearch on a project
    I want to get Imports, Calls, Implements, Extends and type-method edges resolved from exact facts of the whole tree, with Rust paths resolved through the real module tree of each Cargo crate
    So that callers, callees, hierarchy and dead code are true for the current tree, never guessed, and every reference that cannot be resolved is counted with its reason

  Scenario: A call into another workspace crate resolves through its Cargo manifests
    Given a Cargo workspace with crates app and util, where app depends on util by a workspace path dependency
    And app's fn main calls util::parse()
    When ast_index runs for the project
    Then ast_callers of parse lists main

  Scenario: Nested modules resolve through mod declarations and count every call site
    Given a crate with src/lib.rs declaring mod a, src/a.rs declaring mod b and src/a/b.rs holding pub fn deep()
    And a function in src/lib.rs that calls a::b::deep() and crate::a::b::deep()
    When ast_index runs for the project
    Then the function has one Calls edge to deep with callCount 2

  Scenario: Grouped and renamed use declarations bind names exactly
    Given src/lib.rs with use crate::a::{b::deep, other as renamed}; and a function calling deep() and renamed()
    When ast_index runs for the project
    Then the function calls deep in src/a/b.rs and other in src/a.rs

  Scenario: super and self paths resolve relative to the module
    Given src/a/b.rs calling super::sibling() and self::local()
    When ast_index runs for the project
    Then the caller calls sibling in src/a.rs and local in src/a/b.rs

  Scenario: A pub use re-export is followed to its definition
    Given src/lib.rs with pub use inner::Thing; and impl Thing { pub fn new() } in src/inner.rs
    And a function in another module calling crate::Thing::new()
    When ast_index runs for the project
    Then the function calls new in src/inner.rs

  Scenario: self method calls resolve to the enclosing impl's type
    Given impl Engine { fn start(&self) { self.run() } fn run(&self) {} }
    When ast_index runs for the project
    Then start calls run
    And Engine has HasMethod edges to start and run only

  Scenario: An ambiguous trait method call is never guessed
    Given impls of traits P and Q for Engine that both define fn go, and an Engine method calling self.go()
    When ast_index runs for the project
    Then no Calls edge leads to either go
    And the index result counts one unresolved call with reason ambiguous

  Scenario: Calls on unknown receivers and external paths are unresolved with their reason
    Given a function calling value.len() on a local variable and std::mem::swap(a, b)
    When ast_index runs for the project
    Then the index result counts one unresolved call with reason receiver_type_unknown and one with reason external

  Scenario: Calls inside a macro token tree are unresolved
    Given a test calling assert_eq!(compute(1), 2)
    When ast_index runs for the project
    Then no Calls edge leads to compute
    And the index result counts one unresolved call with reason macro_token_tree

  Scenario: A file outside every crate's module tree is not resolved
    Given a .rs file under the root that no crate's mod tree reaches, calling fn helper
    When ast_index runs for the project
    Then its call is counted as unresolved with reason not_in_module_tree

  Scenario: Rust trait implementations and supertraits give Implements and Extends
    Given impl Display for Engine where Display is a trait under the root, impl std::fmt::Debug for Engine, and trait Child: Parent
    When ast_index runs for the project
    Then the graph holds Implements Engine -> Display and Extends Child -> Parent
    And no edge leads from Engine to Debug

  Scenario: Class bases in Python and TypeScript give Extends and Implements
    Given Python class Dog(Animal) importing Animal by from .animals import Animal
    And TypeScript class A extends B implements I with B and I imported from ./base
    When ast_index runs for the project
    Then the graph holds Extends Dog -> Animal, Extends A -> B and Implements A -> I

  Scenario: Imported names resolve calls in Python and TypeScript
    Given Python from .helpers import tidy followed by tidy()
    And TypeScript import { tidy } from './helpers' followed by tidy()
    When ast_index runs for the project
    Then each caller calls tidy in its own language's helpers file

  Scenario: A call inside a decision node is conditional
    Given a caller whose only call to go is inside if ready { go() } and a call to stay() outside any decision node
    When ast_index runs for the project
    Then the Calls edge to go has isConditional true and the Calls edge to stay has isConditional false

  Scenario: The same tree always gives the same graph
    Given a multi-language fixture with a Cargo workspace
    When it is indexed twice, and once more after its graph is deleted
    Then the three entity lists are byte-identical
    And the three unresolved counts are identical
