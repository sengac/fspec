# AST Research: C/C++ Extractor Current State

## C Extractor
- `extract_c(source, rel_path, _known_files)` - ignores known_files
- Uses pattern matching for functions and types
- No imports/calls/typeref extraction

## C++ Extractor
- `extract_cpp(source, rel_path, _known_files)` - ignores known_files
- Uses line-based scanning for functions (ast-grep patterns fail for C++)
- No imports/calls extraction

## Strategy
- Both use `#include "file.h"` for local includes → parse line-by-line
- Both use bare function calls → use edge_helpers::extract_call_names_from_body
- No TypeRef edges (C/C++ type annotations aren't at function signature level the same way)
