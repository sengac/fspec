# AST Research: C# Extractor
- Current: `extract_csharp(source, rel_path, _known_files)` - ignores known_files
- Uses KindMatcher for method_declaration, constructor_declaration
- Needs: extract_imports (using statements), extract_calls, extract_type_refs
