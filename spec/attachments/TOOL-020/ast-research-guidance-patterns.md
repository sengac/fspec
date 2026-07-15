# AST Research: `_` Positional Patterns in fspec_workflow_guidance.rs

## Scope
Analyzed `codelet/tools/src/fspec_workflow_guidance.rs` for all occurrences of positional `_` argument patterns.

## Findings

### Total `_` Occurrences
The file contains approximately 100+ occurrences of `"_"` in the guidance string literal.

### Pattern Types Found
1. **Work Unit Commands**: `{"_": ["AUTH-001"]}`, `{"_": ["AUTH-001", "specifying"]}`
2. **Feature Commands**: `{"_": ["user-auth"]}`, `{"_": ["user-auth", "Login"]}`
3. **Foundation Commands**: `{"_": ["Work Management"]}`, `{"_": ["context", "item"]}`
4. **Tag Commands**: `{"_": ["@security", "category", "desc"]}`
5. **Dependency Commands**: `{"_": ["AUTH-002", "AUTH-001"]}`

### Key Locations
- Lines 23-25: Initial CRITICAL section examples
- Lines 75: ACDD workflow examples
- Lines 90-97: Foundation discovery examples
- Lines 110-119: Foundation event storm examples
- Lines 157-178: Feature event storm examples
- Lines 186-213: Example mapping examples
- Lines 242-272: Specifying phase examples
- Lines 278-305: Feature query and tag examples
- Lines 364-377: Testing phase examples
- Lines 404-408: Done phase examples
- Lines 434-437: Estimation examples
- Lines 480-495: Coverage tracking examples
- Lines 522-577: Work unit management examples
- Lines 590-614: Stable indices examples
- Lines 623-637: Epic/prefix management examples
- Lines 643-670: Dependency management examples
- Lines 679-696: Tag management examples
- Lines 706-731: Architecture/diagrams examples
- Lines 737-731: Attachments examples
- Lines 768-773: Hook management examples
- Lines 779-791: Virtual hooks examples
- Lines 805-824: Git checkpoints examples
- Lines 858-873: Metrics examples
- Lines 963-975: Validation examples
- Lines 987-1044: Complete ACDD example

### Recommended Approach
Replace ALL `"_"` patterns with named keys. This is a documentation-only change to the raw string literal. No code logic changes needed.
