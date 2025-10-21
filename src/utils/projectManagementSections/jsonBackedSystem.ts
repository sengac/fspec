export function getJsonBackedSystemSection(): string {
  return `## JSON-Backed Documentation System

fspec uses a **dual-format documentation system** combining human-readable Markdown with machine-readable JSON:

### Architecture Foundation
- **spec/FOUNDATION.md**: Human-readable project foundation, architecture, and phase documentation
- **spec/foundation.json**: Machine-readable data containing:
  - Mermaid diagrams with automatic syntax validation
  - Structured metadata for programmatic access
  - Single source of truth for tooling

### Tag Registry
- **spec/TAGS.md**: Human-readable tag documentation and guidelines
- **spec/tags.json**: Machine-readable tag registry containing:
  - Tag definitions with categories and descriptions
  - Single source of truth for tag validation
  - Automatically validated by \`fspec validate-tags\`

### Benefits of JSON-Backed System
1. **Dual Format**: Human-readable Markdown + machine-readable JSON
2. **Validation**: Automatic validation using JSON Schema (Ajv)
3. **Type Safety**: TypeScript interfaces map to JSON schemas
4. **Mermaid Validation**: Diagrams validated with mermaid.parse() before storage
5. **CRUD Operations**: Full create, read, update, delete via fspec commands
6. **Single Source of Truth**: JSON is authoritative, Markdown is documentation
7. **Version Control**: Both formats tracked in git for full history

### Bootstrapping Foundation for New Projects

For new projects without \`spec/foundation.json\`, use AI-driven discovery:

\`\`\`bash
fspec discover-foundation           # Create draft with placeholders
# AI fills fields one by one using fspec update-foundation
fspec discover-foundation --finalize # Validate and create foundation.json
\`\`\`

**Workflow**: Draft creation → Field-by-field prompting → AI analysis/update → Automatic chaining → Validation/finalization. See \`fspec discover-foundation --help\` for complete details.

## Benefits of This Approach

1. **Single Source of Truth**: Feature files + JSON data are the definitive specification
2. **Machine-Readable**: Can generate test skeletons, documentation, and reports
3. **Executable Documentation**: Scenarios become automated tests
4. **Traceability**: Tags link scenarios to phases, components, and priorities
5. **AI-Friendly**: Structured format guides AI agents to capture correct information
6. **Ecosystem Compatibility**: Works with all Cucumber tooling (parsers, formatters, reporters)
7. **Version Controlled**: Specifications evolve with code in git
8. **Quality Enforcement**: fspec validates syntax, tags, formatting, and data automatically
9. **Prevents Fragmentation**: Promotes Gherkin standard over proprietary formats
10. **Data Validation**: JSON Schema ensures data integrity across all documentation`;
}
