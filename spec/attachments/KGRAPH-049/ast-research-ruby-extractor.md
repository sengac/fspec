# AST Research: Ruby Extractor
- Current: `extract_ruby(source, rel_path, _known_files)` - ignores known_files
- Uses ast-grep patterns for methods and class/module types
- Needs: extract_imports (require_relative), extract_calls
