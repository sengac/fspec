# AST Research: Swift Extractor
- Current: `extract_swift(source, rel_path, _known_files)` - ignores known_files
- Uses ast-grep patterns for func and type declarations
- Needs: extract_calls (no file-level imports in Swift)
