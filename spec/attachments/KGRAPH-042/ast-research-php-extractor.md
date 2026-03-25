# AST Research: PHP Extractor Current State

## Current Functions in ast_php_extractor.rs
- `extract_php(source, rel_path, _known_files)` — main entry point (known_files unused, prefixed with _)
- `extract_functions(root, file_slug, lang, entities)` — extracts Function nodes via KindMatcher on `method_declaration` and `function_definition`
- `extract_types(root, file_slug, lang, entities)` — extracts Type nodes via KindMatcher on class/interface/trait/enum declarations
- `extract_php_func_name(text)` — extracts function name after `function ` keyword
- `is_php_public(text)` — checks if method is public (not private/protected)

## PHP AST Node Kinds Needed
- **Imports**: `namespace_use_declaration` — `use Namespace\Class;` statements
- **Calls**: `function_call_expression`, `member_call_expression`, `scoped_call_expression`
- **TypeRef**: Type annotations in parameter lists and return types — found in `simple_parameter` and function return types

## Missing Edge Types
1. **Imports edges** (File→File): Not extracted. Need to parse `use` statements, resolve namespace to path via PSR-4 mapping
2. **Calls edges** (Function→Function): Not extracted. Need to scan function bodies for call expressions
3. **TypeRef edges** (Function→Type): Not extracted. Need to parse type annotations in signatures

## Reference: TS Extractor Pattern
The TS extractor follows this pattern:
1. Extract functions → collect function names
2. Extract types → collect type names
3. Extract imports → collect import map (name→target slug)
4. Extract calls → scan function bodies, resolve against local functions + import map
5. Extract type refs → scan function signatures, resolve against local types + import map

## PHP-Specific Import Resolution
PSR-4: `use Slim\Routing\RouteResolver;` → resolve to `Slim/Routing/RouteResolver.php`
- Convert backslashes to forward slashes
- Append `.php`
- Check against known_files
- External packages (not in known_files) are skipped
