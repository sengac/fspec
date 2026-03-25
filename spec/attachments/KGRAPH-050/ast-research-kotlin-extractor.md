# AST Research: Kotlin Extractor
- Current: `extract_kotlin(source, rel_path, _known_files)` - ignores known_files
- Uses KindMatcher for function_declaration and type kinds
- Needs: extract_imports, extract_calls, extract_type_refs
