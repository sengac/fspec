# AST Research: Go Extractor
## Current: extract_go(source, rel_path, _known_files) - Functions and Types only
## Missing: Imports edges (Go import statements), Calls edges (function calls in bodies)
## Go import resolution: string paths - local if starts with . or matches known project dirs
