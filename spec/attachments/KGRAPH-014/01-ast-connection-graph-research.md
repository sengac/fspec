# AST Connection Graph — Detailed Research

## Concept

An AST Connection Graph maps the **static structure** of an entire codebase into a queryable property graph. Unlike the existing KGRAPH system which indexes conversation history, this graph indexes **code itself** — parsing every source file via AST analysis to extract structural relationships.

## Node Types

### 1. `File`
- **Key**: relative file path (e.g., `src/commands/validate.ts`)
- **Properties**: `language`, `lineCount`, `lastModified`, `size`, `isTest`, `isGenerated`
- **Volume**: ~hundreds to low thousands per project

### 2. `Module` / `Package`
- **Key**: module path (e.g., `codelet::napi::graph`)
- **Properties**: `moduleType` (namespace/package/crate/module), `exportCount`, `isPublic`
- **Volume**: ~tens to hundreds

### 3. `Function`
- **Key**: qualified name (e.g., `graph::dispatch::handle_search`)
- **Properties**: `name`, `isAsync`, `isPublic`, `paramCount`, `returnType`, `lineStart`, `lineEnd`, `complexity` (cyclomatic)
- **Volume**: ~hundreds to low thousands

### 4. `Type` (struct, class, interface, enum, trait)
- **Key**: qualified name (e.g., `graph::GraphEntity`)
- **Properties**: `typeKind` (struct/class/interface/enum/trait/type_alias), `fieldCount`, `isGeneric`, `isPublic`
- **Volume**: ~hundreds

### 5. `Import`
- **Key**: `{source_file}:{imported_path}`
- **Properties**: `importPath`, `isTypeOnly`, `isDefault`, `importedNames[]`
- **Volume**: ~thousands

### 6. `Dependency`
- **Key**: package name + version (e.g., `nanograph@1.0.0`)
- **Properties**: `version`, `isDev`, `source` (npm/crate/pip), `dependencyType` (direct/transitive)
- **Volume**: ~tens to hundreds

## Edge Types

### 1. `ContainsFunction` (File → Function)
### 2. `ContainsType` (File → Type)
### 3. `Imports` (File → File or File → Dependency)
### 4. `Calls` (Function → Function)
- **Properties**: `callCount`, `isConditional`, `isAsync`
### 5. `Implements` (Type → Type/Trait)
### 6. `Extends` (Type → Type)
### 7. `FieldOf` (Type → Type) — field type references
### 8. `Returns` (Function → Type)
### 9. `ParamOf` (Function → Type) — parameter type references
### 10. `ExportedBy` (Function/Type → Module)
### 11. `DependsOn` (Package → Dependency)

## Extraction Strategy

### For TypeScript/JavaScript (fspec codebase)
- Use **tree-sitter** (already in codebase) or **ast-grep** (already available as AstGrep tool)
- Parse patterns:
  - `import { X } from 'Y'` → File→Imports→File edges
  - `function X()` / `const X = () =>` → Function nodes + ContainsFunction edges
  - `interface X` / `class X` / `type X` → Type nodes
  - `X.Y()` / `await X()` → Calls edges
  - `implements X` / `extends X` → Implements/Extends edges

### For Rust (codelet codebase)
- Parse `use` declarations → Import nodes
- Parse `fn`, `struct`, `enum`, `trait`, `impl` → Function/Type nodes
- Parse method calls, function calls → Calls edges
- Parse `impl X for Y` → Implements edges

### For Dependencies
- Parse `package.json` / `Cargo.toml` / `Cargo.lock` → Dependency nodes
- Optionally parse dependency source (node_modules, .cargo/registry) for deeper analysis

## Build Strategy

Unlike the Learnings graph (which is incremental), the AST graph should be **rebuilt from scratch** when needed:

1. **Full rebuild**: Walk the file tree, parse each file, emit entities, bulk-load into graph
2. **Incremental update**: Track file modification times, re-parse only changed files
3. **On-demand**: Rebuild when user explicitly requests via `GraphSearch index` or a scheduled job

Full rebuild for a 100K LOC codebase should take <30 seconds (tree-sitter is extremely fast).

## Query Use Cases

1. **"What calls function X?"** → Traverse incoming `Calls` edges to Function node
2. **"What does module X depend on?"** → Traverse `Imports` edges from Files in module
3. **"Show the type hierarchy for X"** → Traverse `Extends`/`Implements` edges
4. **"What files would be affected if I change type X?"** → Reverse traverse `FieldOf`/`ParamOf`/`Returns` edges
5. **"What are the entry points to the system?"** → Functions with no incoming `Calls` edges that are exported
6. **"Find circular dependencies"** → Cycle detection on `Imports` graph
7. **"What's the complexity hotspot?"** → Functions sorted by cyclomatic complexity

## Storage Estimate

For the fspec codebase (~200 files, ~50K LOC):
- ~200 File nodes + ~500 Function nodes + ~200 Type nodes + ~100 Dependency nodes = ~1000 nodes
- ~2000 Import edges + ~1500 Calls edges + ~500 type edges = ~4000 edges
- Estimated storage: **<10MB** (vs 7.6GB for the old system)

## Integration with GraphSearch Tool

Extend the existing `GraphSearchAction` enum:
- `AstSearch { query, language, entity_type }` — search code entities by name/pattern
- `AstNeighbors { node_id, edge_types, depth }` — traverse code structure
- `AstCallChain { from, to }` — find call chains between functions
- `AstImpact { node_id }` — impact analysis for changes to a node
- `AstStats` — codebase statistics

## Existing Tools to Leverage

- **tree-sitter**: Already in the codebase (used by fspec research AST tool)
- **ast-grep**: Already available as AstGrep tool in codelet
- **nanograph**: Can be reused with a new schema for the AST graph
- **Lance**: Underlying storage engine, already integrated

## Key Implementation Notes

- Keep AST graph in a **separate nanograph database** from the Learnings graph
  - e.g., `~/.fspec/graph/ast-code.nano/` vs `~/.fspec/graph/learnings.nano/`
- Use batch loading (not per-entity) to avoid the Lance version amplification problem
- Consider in-memory graph for small codebases (< 10K nodes) with disk persistence for large ones
- The graph should be **project-scoped** (one per working directory), not global
