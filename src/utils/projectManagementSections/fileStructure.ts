export function getFileStructureSection(): string {
  return `## File Structure and Organization

**CRITICAL**: All feature files MUST be in a **flat directory structure** (\`spec/features/*.feature\`). Organization is done via **@tags**, NOT subdirectories. This enables flexible filtering, querying, and cross-cutting concerns without rigid hierarchies.

### Directory Layout

\`\`\`
spec/
├── CLAUDE.md                    # This file - specification process guide
├── FOUNDATION.md                # Project foundation, architecture, and phases (human-readable)
├── foundation.json              # Machine-readable foundation data (diagrams, etc.)
├── TAGS.md                      # Tag registry documentation (human-readable)
├── tags.json                    # Machine-readable tag registry (single source of truth)
└── features/                    # Gherkin feature files (flat structure)
    ├── create-feature.feature
    ├── create-feature.feature.coverage      # Coverage tracking (auto-created)
    ├── add-scenario.feature
    ├── add-scenario.feature.coverage
    ├── add-step.feature
    ├── add-step.feature.coverage
    ├── gherkin-validation.feature
    ├── gherkin-validation.feature.coverage
    ├── tag-registry-management.feature
    ├── tag-registry-management.feature.coverage
    ├── add-diagram.feature
    ├── add-diagram.feature.coverage
    ├── format-feature-files.feature
    ├── format-feature-files.feature.coverage
    ├── list-features.feature
    ├── list-features.feature.coverage
    ├── show-feature.feature
    ├── show-feature.feature.coverage
    └── validate-tags.feature
    └── validate-tags.feature.coverage
\`\`\`

**Note**: \`.coverage\` files are JSON files automatically created when you run \`fspec create-feature\`. They track scenario-to-test-to-implementation mappings.

**Note**: Features are organized by tags (e.g., @critical, @high), NOT by directory structure. All feature files live in the flat \`spec/features/\` directory.

## CRITICAL: Feature File and Test File Naming

**ALWAYS name files using "WHAT IS" (the capability), NOT "what we're doing to build it"!**

Feature files are **living documentation** that must make sense AFTER implementation, not just during.

**✅ CORRECT**: \`user-authentication.feature\`, \`gherkin-validation.feature\` (describes the capability)
**❌ WRONG**: \`implement-authentication.feature\`, \`add-validation.feature\` (describes the task), \`AUTH-001.feature\` (work unit ID)

**Why**: Living documentation, timeless naming, clear intent, better discoverability.

**Process**: Identify capability → Name it (noun phrase) → Apply to all files (feature, test, source)

### Feature File Template

\`\`\`gherkin
@component @feature-group @technical-tags @priority
Feature: [Feature Name]

  """
  Architecture notes:
  - [Key architectural decisions]
  - [Dependencies and integrations]
  - [Critical implementation requirements]
  - [References to external docs if needed]
  """

  Background: User Story
    As a [role]
    I want to [action]
    So that [benefit]

  Scenario: [Scenario name describing a specific acceptance criterion]
    Given [precondition]
    And [additional precondition]
    When [action or trigger]
    And [additional action]
    Then [expected outcome]
    And [additional expected outcome]

  Scenario: [Another scenario]
    Given [precondition]
    When [action]
    Then [expected outcome]
\`\`\``;
}
