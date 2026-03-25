# AST Research: Scala Extractor
- Current: `extract_scala(source, rel_path, _known_files)` - ignores known_files
- Uses ast-grep patterns for def and type declarations
- Needs: extract_imports, extract_calls, extract_type_refs
