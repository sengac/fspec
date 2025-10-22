export function getFormattingSection(): string {
  return `## Formatting and Linting

### Custom AST-Based Formatter

All \`.feature\` files MUST be formatted using fspec's built-in custom AST-based formatter.

**Important**: fspec uses a custom formatter powered by @cucumber/gherkin, NOT Prettier with prettier-plugin-gherkin. This ensures consistent, correct Gherkin formatting without the issues found in prettier-plugin-gherkin.

**Formatting Guarantees**:
- Consistent indentation (2 spaces)
- Proper spacing around keywords
- Preserves doc strings (""") and data tables (|)
- Maintains tag formatting
- Respects Gherkin AST structure

**Note**: Prettier is only used for TypeScript/JavaScript code formatting, not for .feature files.

### Automated Formatting

Run these commands regularly:

\`\`\`bash
# Format all feature files using fspec's custom formatter
fspec format

# Format specific feature file
fspec format spec/features/gherkin-validation.feature

# Validate Gherkin syntax
fspec validate

# Validate specific feature file
fspec validate spec/features/gherkin-validation.feature

# Run complete validation (syntax + tags)
fspec check
\`\`\``;
}
