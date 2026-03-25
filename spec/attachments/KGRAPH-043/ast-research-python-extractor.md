# AST Research: Python Extractor Current State

## Current Functions in ast_python_extractor.rs
- `extract_python(source, rel_path, _known_files)` — main entry (known_files unused)
- `extract_functions(root, file_slug, entities)` — Function nodes via pattern `def $NAME($$$ARGS): $$$BODY`
- `extract_types(root, file_slug, entities)` — Type nodes via `class $NAME: $$$BODY`

## Missing Edge Types
1. **Imports edges**: `import X`, `from X import Y`, `from X.Y import Z`
2. **Calls edges**: function calls in def bodies
3. **TypeRef edges**: Python type annotations (optional, PEP 484+)

## Python Import Resolution
- `from click.core import BaseCommand` → `click/core.py`
- `import os.path` → external (skip)
- Relative: `from .utils import helper` → resolve relative to current package
- Dot-separated to path: replace `.` with `/` + `.py`
