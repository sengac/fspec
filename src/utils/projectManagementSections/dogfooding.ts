export function getDogfoodingSection(): string {
  return `## Using fspec to Manage Its Own Specifications

fspec is designed to "eat its own dog food" - it should be used to manage its own specifications.

### Creating New Feature Files

\`\`\`bash
# Create a new feature file
fspec create-feature "Advanced Query Operations"

# This creates spec/features/advanced-query-operations.feature with template
\`\`\`

### Adding Scenarios

\`\`\`bash
# Add a scenario to an existing feature
fspec add-scenario advanced-query-operations "Filter features by multiple tags"

# Add steps to the scenario
fspec add-step advanced-query-operations "Filter features by multiple tags" given "I have feature files with various tags"
fspec add-step advanced-query-operations "Filter features by multiple tags" when "I run 'fspec list-features --tag=@critical --tag=@critical'"
fspec add-step advanced-query-operations "Filter features by multiple tags" then "only features with both tags should be listed"
\`\`\`

### Managing Architecture Documentation

\`\`\`bash
# Add architecture notes to a feature
fspec add-architecture gherkin-validation "Uses @cucumber/gherkin-parser for validation. Supports all Gherkin keywords."

# Add Mermaid diagram to foundation.json (with automatic syntax validation)
fspec add-diagram "Architecture Diagrams" "Command Flow" "graph TB\\n  CLI-->Parser\\n  Parser-->Validator"
\`\`\`

**Note**: All Mermaid diagrams are validated using mermaid.parse() before being added to foundation.json. Invalid syntax will be rejected with detailed error messages including line numbers.

### Managing Tags

\`\`\`bash
# Register a new tag (adds to spec/tags.json)
fspec register-tag @performance "Technical Tags" "Performance-critical features requiring optimization"

# Update tag description
fspec update-tag @performance --description "Updated description"

# Delete tag
fspec delete-tag @performance

# List all registered tags
fspec list-tags

# List tags by category
fspec list-tags --category "Phase Tags"

# Validate all tags in feature files are registered
fspec validate-tags

# Show tag statistics
fspec tag-stats
\`\`\`

**Note**: Tags are stored in spec/tags.json (single source of truth). The spec/TAGS.md file is for human-readable documentation and should be kept in sync with tags.json.`;
}
