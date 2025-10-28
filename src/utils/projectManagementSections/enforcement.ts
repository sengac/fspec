export function getEnforcementSection(): string {
  return `## Enforcement Rules

### MANDATORY Requirements

1. **NO Markdown-Based Specifications**
   - DO NOT create user stories or acceptance criteria in \`.md\` files
   - ALL specifications MUST be in \`.feature\` files using Gherkin syntax
   - Exception: FOUNDATION.md, TAGS.md, and {{DOC_TEMPLATE}} are meta-documentation

2. **Tag Compliance**
   - Every \`.feature\` file MUST have at minimum: phase tag, component tag, and feature group tag
   - ALL tags MUST be documented in \`spec/TAGS.md\`
   - DO NOT create ad-hoc tags without updating the tag registry

3. **Background Section Required**
   - Every feature MUST have a \`Background\` section with the user story
   - Use the standard "As a... I want to... So that..." format
   - Multiple related scenarios can share the same background

4. **Proper Gherkin Syntax**
   - Use only valid Gherkin keywords: Feature, Background, Scenario, Scenario Outline, Given, When, Then, And, But, Examples
   - Follow indentation conventions (2 spaces for scenarios, 4 spaces for steps)
   - Use doc strings (""") for multi-line text blocks
   - Use data tables (|) for tabular data if needed
   - Use tags (@) at **both feature level and scenario level**
   - Feature-level tags have zero indentation
   - Scenario-level tags have 2-space indentation (same as scenario keyword)

5. **Formatting Before Commit**
   - Run \`fspec format\` before committing changes
   - Feature files that fail \`fspec validate\` will be rejected

### Validation Process

Before creating a pull request:

1. **Format Check**: \`fspec format\` should be run on all feature files
2. **Gherkin Syntax**: \`fspec validate\` must pass (validates Gherkin syntax)
3. **Tag Validation**: \`fspec validate-tags\` must pass (all tags exist in spec/TAGS.md or spec/tags.json)
4. **Test Coverage**: Each scenario must have corresponding test(s)
5. **Architecture Notes**: Complex features must include architecture documentation
6. **Build & Tests**: \`<quality-check-commands>\` and \`<test-command>\` must pass`;
}
