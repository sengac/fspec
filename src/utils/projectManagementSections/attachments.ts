export function getAttachmentsSection(): string {
  return `## Attachment Support for Discovery Process

During Example Mapping and discovery, you can attach supporting files (diagrams, mockups, documents) to work units.

### Attachment Commands

\`\`\`bash
# Add attachment to work unit
fspec add-attachment <work-unit-id> <file-path>
fspec add-attachment AUTH-001 diagrams/auth-flow.png

# Add attachment with description
fspec add-attachment UI-002 mockups/dashboard.png --description "Dashboard v2"

# List attachments for work unit
fspec list-attachments AUTH-001

# Remove attachment from work unit (deletes file)
fspec remove-attachment AUTH-001 diagram.png

# Remove attachment but keep file on disk
fspec remove-attachment AUTH-001 important-doc.pdf --keep-file
\`\`\`

### Attachment Storage

- **Location**: Files are copied to \`spec/attachments/<work-unit-id>/\`
- **Tracking**: Attachment paths stored in work unit metadata as relative paths from project root
- **Visibility**: Attachments displayed when running \`fspec show-work-unit <work-unit-id>\`

### When to Use Attachments

✅ **Use attachments for**:
- Diagrams explaining system architecture or flows
- Mockups showing UI designs
- Screenshots of existing behavior
- Documents with detailed requirements
- API contract files (OpenAPI, GraphQL schemas)

❌ **Don't use attachments for**:
- Source code (belongs in implementation)
- Test data (belongs in test files)
- Configuration files (belongs in project config)

### Example Discovery Workflow with Attachments

\`\`\`bash
# 1. Example Mapping with questions, rules, examples
fspec add-example AUTH-001 "User enters valid email and receives reset link"

# 2. Attach diagrams/mockups during discovery
fspec add-attachment AUTH-001 diagrams/auth-flow.png --description "Auth flow"

# 3. Generate scenarios from example map
fspec generate-scenarios AUTH-001
\`\`\`

### Attachment Validation

- Source file must exist before copying
- Work unit must exist before adding attachments
- Attachment paths are validated when listing or showing work units
- Missing files are reported with warnings`;
}
