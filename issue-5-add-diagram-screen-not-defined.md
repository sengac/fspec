# Issue #5: add-diagram fails with the error "screen is not defined" for a valid C4 diagram

**Author:** dzied-baradzied (Anton Bastynets)
**Created:** November 16, 2025
**Status:** Open
**Source:** https://github.com/sengac/fspec/issues/5

## Problem Description

The `add-diagram` command in fspec (version 0.8.6) throws an error when attempting to add a valid C4 diagram: "Invalid Mermaid syntax: screen is not defined"

## Expected Outcome

C4 diagram should be successfully added to the project foundation

## Actual Outcome

Command fails with:
```
Error: Invalid Mermaid syntax: screen is not defined
```

## Reproduction Steps

Execute this fspec command with a C4Context diagram containing system context elements (persons, systems, external systems, and relationships):

```bash
fspec add-diagram "Architecture" "C4 System Context" "C4Context
  title System Context diagram for My Awesome System
  Person(user1, \"User 1\", \"Description for User 1.\")
  System(my_system, \"My Awesome System\", \"Description for My Awesome System.\")
  System_Ext(ext_system1, \"External System 1\", \"Description for External System 1.\")
  System_Ext(ext_system2, \"External System 2\", \"Description for External System 2.\")
  Rel(user1, my_system, \"Uses\", \"to do something\")
  Rel(my_system, ext_system1, \"Uses\", \"to do something else\")
  Rel(my_system, ext_system2, \"Provides data to\", \"to do something else\")"
```

## Environment

- **fspec version:** 0.8.6
- **Node version:** v22.16.0
- **OS:** darwin (macOS)

## Analysis

The error "screen is not defined" suggests that:
1. The Mermaid validator is incorrectly configured or missing C4 diagram support
2. The C4 plugin for Mermaid may not be loaded
3. There may be an issue with how the diagram string is being passed to the Mermaid validator

C4 diagrams are a valid Mermaid diagram type and should be supported.
