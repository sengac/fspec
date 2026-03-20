# Nanograph Query Language & Schema DSL

## Schema Language (`.pg` files)

Defines the graph structure with typed nodes and edges:

```
node Person {
    name: String
    age:  I32?              // nullable
    email: String @unique
    id: U64 @key
}

node Employee : Person {    // inheritance
    employee_id: String @unique
}

edge Knows: Person -> Person {
    since: Date?
}
```

### Scalar Types
`String`, `Bool`, `I32`, `I64`, `U32`, `U64`, `F32`, `F64`, `Date`, `DateTime`, `Vector(dim)`

### Composite Types
- `enum(val1, val2, ...)` — enumeration
- `[String]` — lists
- `?` suffix — nullable

### Annotations
- `@key` — merge key for upsert semantics
- `@unique` — unique constraint
- `@index` — create scalar index
- `@embed(source_prop)` — auto-generate embedding from property
- `@rename_from("old")` — schema migration helper
- `@description("...")` / `@instruction("...")` — metadata

## Query Language (`.gq` files)

Datalog-flavored, GraphQL-shaped syntax:

```
query friends_of($name: String) {
    match {
        $p: Person { name: $name }    // binding with property match
        $p knows $f                    // edge traversal
        $f.age > 25                    // filter
        not { $f worksAt $_ }         // stratified negation
    }
    return {
        $f.name
        count($f) as num_friends       // aggregation
    }
    order { $f.name asc }
    limit 10
}
```

### Key Features

| Feature | Syntax | Description |
|---------|--------|-------------|
| Bindings | `$p: Person { name: "Alice" }` | Variable binding with optional property matching |
| Traversals | `$p knows $f` | Edge traversal as Datalog predicates |
| Bounded expansion | `$a knows{1,3} $b` | Union of 1-hop, 2-hop, 3-hop |
| Negation | `not { ... }` | Compiles to AntiJoin |
| Optional (left join) | `maybe { $p worksAt $c }` | Unmatched → null |
| Disjunction | `or { { branch1 } { branch2 } }` | At least one shared variable |
| Aggregates | `count`, `sum`, `avg`, `min`, `max` | Standard aggregations |
| Text search | `search(prop, query)` | Token match |
| Fuzzy search | `fuzzy(prop, query, max_edits)` | Approximate match |
| Vector nearest | `nearest(vector_prop, query)` | Cosine distance |
| BM25 ranking | `bm25(prop, query)` | BM25 scoring |
| Hybrid ranking | `rrf(nearest(...), bm25(...))` | Reciprocal rank fusion |

### Mutations

```
// Insert
mutation add_person($name: String, $age: I32) {
    insert Person { name: $name, age: $age }
}

// Update (requires @key)
mutation update_person($slug: String, $name: String) {
    update Person { slug: $slug, name: $name }
}

// Delete (cascades edges)
mutation remove_person($name: String) {
    delete $p: Person { name: $name }
}
```

### Desugaring to Datalog

| Surface | Internal |
|---------|----------|
| `$p: Person` | `Person($p)` |
| `$p knows $f` | `knows($p, $f)` |
| `not { $p worksAt $_ }` | `¬∃c: worksAt($p, c)` |
| `$a knows{1,3} $b` | `knows¹($a,$b) ∨ knows²($a,$b) ∨ knows³($a,$b)` |

### Query Execution Pipeline

```
.gq text → parse_query() → QueryAST
         → typecheck_query() → TypeContext (validates against catalog)
         → lower_query() → QueryIR (pipeline of IROp operators)
         → build_physical_plan() → DataFusion ExecutionPlan
         → execute_query() → Vec<RecordBatch>
```
