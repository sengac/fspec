# AST Research: Rust Extractor Current State

## Current extract_rust function signature
`pub fn extract_rust(source: &str, rel_path: &str, _known_files: &HashSet<String>)`

## Current functions
- `extract_functions` - extracts function declarations, returns nothing (should return HashSet<String>)
- `extract_types` - extracts type declarations, returns nothing (should return HashSet<String>)

## Missing functions (needed for edge extraction)
- `extract_imports` - parse `use crate::`, `use super::`, `mod` statements
- `extract_calls` - scan function bodies for call expressions
- `extract_type_refs` - parse type annotations in signatures

## Reference: PHP extractor pattern (already working)
- `extract_functions()` → returns `HashSet<String>` of function names
- `extract_types()` → returns `HashSet<String>` of type names
- `extract_imports()` → returns `HashMap<String, (String, bool, String)>` import map
- `extract_calls()` → uses edge_helpers::extract_call_names_from_body + resolve_calls
- `extract_type_refs()` → uses edge_helpers::extract_type_names_from_signature + resolve_type_refs

## Rust-specific import patterns
- `use crate::module::submodule;` → resolve to module/submodule.rs or module/submodule/mod.rs
- `use super::sibling;` → resolve relative to parent
- `use self::child;` → resolve relative to current module
- `mod child_module;` → resolve to child_module.rs or child_module/mod.rs
- `use external_crate::Type;` → SKIP (not local)
