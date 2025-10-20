import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-diagram',
  description:
    'Add or update Mermaid diagram in foundation.json and regenerate FOUNDATION.md',
  usage: 'fspec add-diagram <section> <title> <code>',
  whenToUse:
    'Use this command to add architecture diagrams to foundation.json. The diagram code is validated against Mermaid syntax before being added. Useful for documenting system architecture, data flows, and component relationships.',
  prerequisites: [
    'spec/foundation.json exists (created by fspec init or discover-foundation)',
  ],
  arguments: [
    {
      name: 'section',
      description:
        'Section name (e.g., "Architecture", "Data Flow") - used for organization',
      required: true,
    },
    {
      name: 'title',
      description:
        'Diagram title (e.g., "Command Flow", "System Architecture")',
      required: true,
    },
    {
      name: 'code',
      description: 'Mermaid diagram code (syntax validated before adding)',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command:
        'fspec add-diagram "Architecture" "Command Flow" "graph TB\\n  CLI-->Parser\\n  Parser-->Validator"',
      description: 'Add flowchart diagram',
      output:
        '✓ Added diagram "Command Flow"\n  Updated: spec/foundation.json\n  Regenerated: spec/FOUNDATION.md',
    },
    {
      command:
        'fspec add-diagram "Architecture" "System Overview" "graph LR\\n  User-->API\\n  API-->Database"',
      description: 'Add system architecture diagram',
      output:
        '✓ Added diagram "System Overview"\n  Updated: spec/foundation.json\n  Regenerated: spec/FOUNDATION.md',
    },
    {
      command:
        'fspec add-diagram "Data Flow" "Authentication Flow" "sequenceDiagram\\n  User->>API: Login\\n  API->>DB: Verify"',
      description: 'Add sequence diagram',
      output:
        '✓ Added diagram "Authentication Flow"\n  Updated: spec/foundation.json\n  Regenerated: spec/FOUNDATION.md',
    },
  ],
  commonErrors: [
    {
      error: 'Error: Invalid Mermaid syntax: ...',
      fix: 'Validate your Mermaid code at https://mermaid.live or check Mermaid documentation',
    },
    {
      error: 'Error: Diagram code cannot be empty',
      fix: 'Provide Mermaid diagram code as the third argument',
    },
    {
      error: 'Error: Updated foundation.json failed schema validation: ...',
      fix: 'Ensure the diagram conforms to foundation schema requirements',
    },
  ],
  typicalWorkflow:
    '1. Design diagram in Mermaid Live Editor → 2. fspec add-diagram <section> <title> <code> → 3. Verify: fspec show-foundation → 4. View in FOUNDATION.md',
  commonPatterns: [
    {
      pattern: 'Add Architecture Diagrams',
      example:
        'fspec add-diagram "Architecture" "System Overview" "graph TB\\n  User-->API\\n  API-->Database"\nfspec add-diagram "Architecture" "Component Structure" "graph LR\\n  CLI-->Core\\n  Core-->Utils"',
    },
    {
      pattern: 'Update Existing Diagram',
      example:
        '# Adding a diagram with the same title replaces the existing one\nfspec add-diagram "Architecture" "System Overview" "graph TB\\n  User-->Gateway\\n  Gateway-->Services"',
    },
  ],
  relatedCommands: [
    'delete-diagram',
    'show-foundation',
    'generate-foundation-md',
  ],
  notes: [
    'Mermaid syntax is validated using mermaid.parse() before adding',
    'Invalid syntax will be rejected with detailed error messages',
    'If a diagram with the same title exists, it will be replaced',
    'Use \\n for line breaks in diagram code when passing as string',
    'Supports all Mermaid diagram types: flowchart, sequence, class, state, etc.',
    'Diagrams are stored in foundation.json and rendered in FOUNDATION.md',
  ],
};

export default config;
