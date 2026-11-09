# BUG-413 root cause

Silent default on a serialization Result in generated code. Fix the generator/template to return Result (catalog construction fails loudly) and regenerate.

Duplicate note (2026-11-03): this looks like a duplicate of BUG-367 (every MCP dispatch schema() uses unwrap_or_default). When either card is picked up, merge them and close one as the duplicate with a note.
