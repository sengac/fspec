# fspec Feature File Tag Registry

This document defines all tags used in Gherkin feature files. All tags MUST be documented here before use.

## Tag Categories

### Phase Tags (Required)

Tags that identify which development phase a feature belongs to (from FOUNDATION.md).

| Tag | Description | Usage |
|-----|-------------|-------|
| `@phase1` | Phase 1: Core Validation & Feature Management | Features: Gherkin syntax validation, feature file creation, formatting, basic querying (5 files) |
| `@phase2` | Phase 2: Tag Registry & Management | Features: TAGS.md operations, tag validation, architecture/background documentation (7 files) |
| `@phase3` | Phase 3: Advanced Feature Editing | Features: Add scenarios and steps to existing features (2 files) |
| `@phase4` | Phase 4: CRUD Operations & Tag-Based Queries | Features: Query/display scenarios by tag, update/delete operations (4 files) |
| `@phase5` | Phase 5: Advanced CRUD & Bulk Operations | Features: Update scenarios/steps, bulk delete, retag operations (7 files) |
| `@phase6` | Phase 6: Architecture Documentation | Features: FOUNDATION.md management, Mermaid diagrams (3 files) |

**Rule**: Every feature file MUST have exactly ONE phase tag.

### Component Tags (Required)

Tags that identify which architectural component a feature belongs to.

| Tag | Description | Scope |
|-----|-------------|-------|
| `@cli` | Command-Line Interface | All CLI commands, argument parsing, command handlers, user-facing terminal interactions (28 files) |
| `@parser` | Gherkin Parser Integration | @cucumber/gherkin-parser usage, syntax validation, AST processing (3 files) |
| `@generator` | Template Generation | Feature file templates, scaffolding, boilerplate generation (1 file) |
| `@validator` | Validation Logic | Syntax validation, tag validation, consistency checks (1 file) |
| `@formatter` | Formatting & Prettification | Prettier integration, Gherkin formatting, code style enforcement (1 file) |
| `@file-ops` | File Operations | Reading/writing feature files, FOUNDATION.md, TAGS.md management (1 file) |
| `@integration` | Cross-Component Integration | Features spanning multiple components, end-to-end flows (0 files) |

**Rule**: Every feature file MUST have at least ONE component tag (may have multiple if cross-component).

### Feature Group Tags (Required)

Tags that categorize features by functional area.

| Tag | Description | Examples |
|-----|-------------|----------|
| `@feature-management` | Feature File Operations | Create feature, add scenario, add step, add architecture notes (11 files) |
| `@foundation-management` | FOUNDATION.md Operations | Add Mermaid diagram, update foundation sections, show foundation (3 files) |
| `@tag-management` | TAGS.md Operations | Register tag, validate tags, list tags, tag statistics (7 files) |
| `@validation` | Syntax & Structure Validation | Gherkin syntax check, tag compliance, formatting validation (3 files) |
| `@querying` | Query & Search Operations | List features, show feature, find by tag, search scenarios (6 files) |
| `@formatting` | Auto-Formatting | Prettier execution, Gherkin formatting, consistency enforcement (1 file) |
| `@modification` | Feature Modification Operations | Update/delete scenarios, steps, tags, retag operations (11 files) |
| `@bulk-operations` | Bulk Multi-File Operations | Bulk delete scenarios/features by tag (3 files) |
| `@documentation` | Documentation Display | Show acceptance criteria, show feature, show foundation (2 files) |
| `@read-only` | Read-Only Operations | Display operations that don't modify files (2 files) |
| `@utility` | Utility & Helper Commands | Multi-validation commands, aggregate operations (1 file) |
| `@scaffolding` | Project Setup & Templates | Initialize spec directory, create templates, setup structure (0 files) |

**Rule**: Every feature file MUST have at least ONE feature group tag.

### Technical Tags (Optional)

Tags for specific technical concerns or architectural patterns.

| Tag | Description | Use Cases |
|-----|-------------|-----------|
| `@gherkin` | Gherkin Specification Compliance | Features ensuring Gherkin spec adherence |
| `@cucumber-parser` | Cucumber Parser Integration | Direct usage of @cucumber/gherkin-parser |
| `@ast` | Abstract Syntax Tree | Working with Gherkin AST from parser (custom formatter) |
| `@mermaid` | Mermaid Diagram Support | Inserting/validating Mermaid diagrams in FOUNDATION.md |
| `@error-handling` | Error Handling | Error scenarios, validation failures, recovery |
| `@file-system` | File System Operations | Reading/writing files, directory operations, path handling |
| `@template` | Template System | Template generation, variable substitution, scaffolding |

**Rule**: Use technical tags to highlight specific architectural concerns.

### Platform Tags (Optional)

Tags for platform-specific scenarios or cross-platform requirements.

| Tag | Description | When to Use |
|-----|-------------|-------------|
| `@windows` | Windows-Specific | Windows paths, PowerShell compatibility, CMD behaviors |
| `@macos` | macOS-Specific | Unix paths, macOS-specific behaviors, Darwin platform |
| `@linux` | Linux-Specific | Linux distributions, Unix behaviors, shell compatibility |
| `@cross-platform` | Cross-Platform Requirement | Features that MUST work on all platforms, path normalization |

**Rule**: Use platform tags when a scenario has platform-specific behavior or requirements.

### Priority Tags (Optional)

Tags indicating implementation priority or criticality.

| Tag | Description | Criteria |
|-----|-------------|----------|
| `@critical` | Critical Priority - Must Have | Core functionality, blocking features, foundation requirements |
| `@high` | High Priority - Should Have | Important features, significant user value, CAGE integration needs |
| `@medium` | Medium Priority - Nice to Have | Enhancements, convenience features, improved UX |
| `@low` | Low Priority - Future Enhancement | Optional features, advanced querying, cosmetic improvements |

**Rule**: Use priority tags to guide implementation order within a phase.

### Status Tags (Optional)

Tags tracking development status of features.

| Tag | Description | Meaning |
|-----|-------------|---------|
| `@wip` | Work In Progress | Feature is currently being implemented, tests may be incomplete |
| `@todo` | To Do | Feature is planned but not started, scenarios defined but no code |
| `@done` | Complete | Feature is fully implemented, all tests passing, documented |
| `@deprecated` | Deprecated | Feature is being phased out, replaced by newer implementation |
| `@blocked` | Blocked | Feature cannot proceed due to dependencies or external factors |

**Rule**: Update status tags as features progress through development lifecycle.

### Testing Tags (Optional)

Tags for test-related scenarios and requirements.

| Tag | Description | Test Type |
|-----|-------------|-----------|
| `@unit-test` | Unit Test Coverage Required | Tests for individual functions, command handlers, utilities |
| `@integration-test` | Integration Test Required | Tests across components, parser integration, file operations |
| `@e2e-test` | End-to-End Test Required | Complete command flows, CAGE integration, full scenarios |
| `@manual-test` | Requires Manual Testing | Visual verification, platform-specific checks, edge cases |

**Rule**: Use testing tags to indicate what level of testing is required for a scenario.

### CAGE Integration Tags (Optional)

Tags specific to CAGE integration and agentic coding workflows.

| Tag | Description | Use Cases |
|-----|-------------|-----------|
| `@cage-hook` | CAGE Hook Integration | Features called from CAGE hooks (PreToolUse, PostToolUse, etc.) |
| `@execa` | Execa Child Process | Features invoked via Node.js execa from CAGE |
| `@acdd` | Acceptance Criteria Driven Development | Features following ACDD methodology |
| `@spec-alignment` | Specification Alignment | Features ensuring code-spec synchronization |

**Rule**: Use CAGE tags to indicate integration points with CAGE system.

## Tag Combination Examples

### Example 1: Phase 1 Feature File Creation

```gherkin
@phase1 @cli @generator @feature-management @gherkin @cross-platform @critical @unit-test @integration-test
Feature: Create Feature File with Template
```

**Interpretation**:
- Phase 1 feature
- Belongs to CLI and generator components
- Part of feature management functionality
- Ensures Gherkin compliance
- Must work on all platforms
- Critical priority (core functionality)
- Requires both unit and integration tests

### Example 2: Phase 2 Tag Registry Validation

```gherkin
@phase2 @validator @tag-management @validation @error-handling @high @integration-test @cage-hook
Feature: Validate Feature File Tags Against Registry
```

**Interpretation**:
- Phase 2 feature
- Belongs to validator component
- Part of tag management functionality
- Includes validation and error handling
- High priority
- Requires integration tests
- Called from CAGE hooks

### Example 3: Phase 3 Advanced Querying

```gherkin
@phase3 @cli @querying @parser @ast @medium @unit-test @e2e-test
Feature: Advanced Scenario Search and Filtering
```

**Interpretation**:
- Phase 3 feature
- Belongs to CLI and querying components
- Uses parser and AST processing
- Medium priority
- Requires unit and end-to-end tests

## Tag Usage Guidelines

### Required Tag Combinations

Every feature file MUST have:
1. **One Phase Tag**: `@phase1`, `@phase2`, or `@phase3`
2. **At least One Component Tag**: `@cli`, `@parser`, `@generator`, `@validator`, etc.
3. **At least One Feature Group Tag**: `@feature-management`, `@tag-management`, etc.

**Minimum Valid Example**:
```gherkin
@phase1 @cli @feature-management
Feature: Minimal Valid Feature
```

### Recommended Tag Combinations

For complete feature documentation, include:
1. Required tags (phase, component, feature group)
2. Technical tags for key architectural concerns
3. Platform tags if platform-specific
4. Priority tag for implementation planning
5. Testing tags for test strategy
6. CAGE integration tags if applicable

**Recommended Example**:
```gherkin
@phase1 @cli @parser @validation @gherkin @cucumber-parser @error-handling @cross-platform @critical @integration-test @cage-hook
Feature: Gherkin Syntax Validation
```

### Tag Ordering Convention

When multiple tags are present, use this order:
1. Phase tag
2. Component tag(s)
3. Feature group tag(s)
4. Technical tags
5. Platform tags
6. Priority tag
7. Status tag
8. Testing tags
9. CAGE integration tags

**Example**:
```gherkin
@phase2 @cli @validator @tag-management @validation @mermaid @cross-platform @high @wip @integration-test @cage-hook
Feature: FOUNDATION.md Mermaid Diagram Validation
```

## Adding New Tags

### Process for Adding Tags

1. **Identify Need**: Determine if existing tags are insufficient
2. **Check Registry**: Verify tag doesn't already exist in this document
3. **Define Tag**: Create clear description and usage guidelines
4. **Update TAGS.md**: Add to appropriate category table
5. **Document Examples**: Show how tag should be used
6. **Use fspec**: Register tag using `fspec register-tag` command
7. **Apply**: Use tag in feature files

### Tag Naming Conventions

- Use lowercase
- Use hyphens for multi-word tags (`@cross-platform`, not `@crossplatform`)
- Be specific and descriptive (`@cucumber-parser` not `@parser-integration`)
- Avoid redundancy (`@cli-commands` → just `@cli`)
- Keep concise (prefer `@wip` over `@work-in-progress`)

### Tag Anti-Patterns

❌ **DON'T**:
- Create overlapping tags (`@validate` and `@validation` → use `@validation`)
- Use vague tags (`@important` → use `@critical`, `@high`, etc.)
- Create single-use tags (`@login-feature` → use combination of existing tags)
- Mix concerns (`@cli-validation` → use `@cli @validation`)

✅ **DO**:
- Reuse existing tags through combinations
- Create specific technical tags (`@gherkin`, `@mermaid`)
- Document new tags thoroughly
- Use tags to enable filtering and reporting

## Tag-Based Queries

### Common Tag Queries

**All Phase 1 features**:
```bash
fspec list-features --tag=@phase1
```

**Critical validation features**:
```bash
fspec list-features --tag=@validation --tag=@critical
```

**Features requiring integration tests**:
```bash
fspec list-features --tag=@integration-test
```

**CAGE hook integration points**:
```bash
fspec list-features --tag=@cage-hook
```

**Cross-platform CLI features**:
```bash
fspec list-features --tag=@cli --tag=@cross-platform
```

## Tag Statistics

### Current Tag Usage (Update Regularly)

| Phase | Total Features | Complete | In Progress | Planned |
|-------|----------------|----------|-------------|---------|
| Phase 1 | 5 | 5 | 0 | 0 |
| Phase 2 | 7 | 7 | 0 | 0 |
| Phase 3 | 2 | 2 | 0 | 0 |
| Phase 4 | 4 | 4 | 0 | 0 |
| Phase 5 | 7 | 7 | 0 | 0 |
| Phase 6 | 3 | 3 | 0 | 0 |
| **Total** | **28** | **28** | **0** | **0** |

| Component | Feature Count | Percentage |
|-----------|---------------|------------|
| @cli | 28 | 100% |
| @parser | 3 | 11% |
| @generator | 1 | 4% |
| @validator | 1 | 4% |
| @formatter | 1 | 4% |
| @file-ops | 1 | 4% |

| Feature Group | Feature Count | Percentage |
|---------------|---------------|------------|
| @feature-management | 11 | 39% |
| @modification | 11 | 39% |
| @tag-management | 7 | 25% |
| @querying | 6 | 21% |
| @foundation-management | 3 | 11% |
| @bulk-operations | 3 | 11% |
| @validation | 3 | 11% |
| @documentation | 2 | 7% |
| @read-only | 2 | 7% |
| @formatting | 1 | 4% |
| @utility | 1 | 4% |

**Update Command**: `fspec tag-stats` (updates this section automatically)

## Validation

### Tag Validation Rules

1. **Registry Compliance**: All tags in feature files MUST exist in this TAGS.md
2. **Required Tags**: Every feature MUST have phase + component + feature group tags
3. **No Orphans**: Tags in TAGS.md should be used in at least one feature file
4. **Consistent Naming**: Follow tag naming conventions strictly

### Validation Commands

**Validate all feature files**:
```bash
fspec validate-tags
```

**Check for orphaned tags**:
```bash
fspec tag-stats --show-orphans
```

**Validate specific feature**:
```bash
fspec validate login.feature
```

## References

- **Gherkin Reference**: https://cucumber.io/docs/gherkin/reference/#tags
- **Cucumber Tag Expressions**: https://cucumber.io/docs/cucumber/api/#tag-expressions
- **fspec Foundation**: [spec/FOUNDATION.md](./FOUNDATION.md)
- **Gherkin Specification Process**: [spec/CLAUDE.md](./CLAUDE.md) (when created)
