export function getLoadContextSection(): string {
  return `## Step 1: Load fspec Context

Load essential fspec documentation:

\`\`\`bash
fspec --help
fspec help specs       # Gherkin feature file commands
fspec help work        # Kanban workflow commands
fspec help discovery   # Example mapping commands
fspec help metrics     # Progress tracking
fspec help setup       # Tag registry and configuration
fspec help hooks       # Lifecycle hooks for workflow automation
\`\`\`

Then read \`spec/CLAUDE.md\` for fspec-specific workflow details.
`;
}
