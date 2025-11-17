# Issue #6: Bug: show-foundation quality issues

**Author:** dzied-baradzied (Anton Bastynets)
**Created:** November 16, 2025
**Status:** Open
**Source:** https://github.com/sengac/fspec/issues/6

## Issue Description

The reporter identified multiple problems with the `fspec show-foundation` command:

1. **Parameter Ignoring:** Various command variations—with and without `--list-sections` or `--section` flags—produce identical output, suggesting parameters aren't being processed.

2. **Unintended File Creation:** The tool generates a new foundation.json file with template data when it doesn't exist, which may be unexpected behavior.

3. **Incomplete Documentation:** The help text references syntax like `fspec show-foundation "Architecture Diagrams"` that reportedly doesn't work without the `--section` flag.

4. **Inconsistent Parameter Names:** The update-foundation command rejects display names (e.g., "What We Are Building") and instead requires programmatic property names like `projectOverview`. The show-foundation command's section filtering likely has similar requirements but lacks clear documentation.

## Environment Details

- **fspec version:** 0.8.6
- **Node version:** v22.16.0
- **OS:** macOS (darwin)

## Key Error Message

```
Error: Unknown section: 'What We Are Building'. Use field names like: projectOverview, problemDefinition, etc.
```

This error message reveals the root issue: documentation uses user-friendly labels, but the CLI expects technical JSON property names.

## Root Cause Analysis

The command appears to have several quality issues:
- Parameters not being properly parsed or applied
- Mismatch between documentation examples and actual implementation
- Inconsistent naming conventions between display names and property names
- Side effects (file creation) that may be unexpected
