import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'dependencies',
  description:
    'Show all dependency relationships for a work unit (blocks, blockedBy, dependsOn, relatesTo)',
  usage: 'fspec dependencies <work-unit-id> [options]',
  whenToUse:
    'Use this command to quickly view all dependency relationships for a specific work unit. Shows which work units it blocks, is blocked by, depends on, and relates to. Essential for understanding work unit relationships and planning implementation order.',
  prerequisites: ['spec/work-units.json exists with work units'],
  arguments: [
    {
      name: 'work-unit-id',
      description: 'Work unit ID to show dependencies for',
      required: true,
    },
  ],
  options: [
    {
      flag: '--graph',
      description:
        'Display dependencies as graph visualization (shows dependency tree)',
    },
  ],
  examples: [
    {
      command: 'fspec dependencies MCP-001',
      description: 'Show dependencies for work unit with no relationships',
      output: 'Dependencies for MCP-001:',
    },
    {
      command: 'fspec dependencies MCP-004',
      description: 'Show dependencies for work unit with multiple relationships',
      output:
        'Dependencies for MCP-004:\n  Depends on: MCP-001, MCP-002\n  Blocks: MCP-005\n  Related to: DOC-MCP-004',
    },
    {
      command: 'fspec dependencies AUTH-001',
      description: 'Show all dependency types',
      output:
        'Dependencies for AUTH-001:\n  Blocks: AUTH-002, AUTH-003\n  Blocked by: INFRA-001\n  Depends on: SCHEMA-001\n  Related to: DOC-001',
    },
    {
      command: 'fspec dependencies AUTH-001 --graph',
      description: 'Show dependencies as graph visualization',
      output:
        'AUTH-001\n  blocks → AUTH-002\n    blocks → AUTH-004\n  blocks → AUTH-003',
    },
  ],
  commonErrors: [
    {
      error: "Error: Work unit 'INVALID-999' does not exist",
      fix: 'Verify work unit ID exists. Run: fspec list-work-units\n\nNote: This error includes AI-friendly system-reminder with suggestions.',
    },
  ],
  typicalWorkflow:
    '1. Create work units → 2. Add dependencies with add-dependency command → 3. View relationships: fspec dependencies <id> → 4. Plan implementation order based on dependencies',
  commonPatterns: [
    {
      pattern: 'Quick Dependency Check',
      example:
        '# Before starting work, check what blocks this work unit\nfspec dependencies UI-001\n\n# Output shows:\n#   Blocked by: AUTH-001, API-001\n#\n# Decision: Wait for AUTH-001 and API-001 to complete',
    },
    {
      pattern: 'Understanding Impact',
      example:
        '# Check what will be unblocked after completing this work\nfspec dependencies AUTH-001\n\n# Output shows:\n#   Blocks: AUTH-002, AUTH-003, UI-001\n#\n# Completing AUTH-001 will unblock 3 work units',
    },
    {
      pattern: 'Dependency Tree Visualization',
      example:
        '# View full dependency tree\nfspec dependencies AUTH-001 --graph\n\n# Shows cascading dependencies',
    },
    {
      pattern: 'Integration with add-dependency',
      example:
        '# Add dependency\nfspec add-dependency UI-001 AUTH-001\n\n# Verify it was added\nfspec dependencies UI-001\n# Output: Depends on: AUTH-001',
    },
  ],
  relatedCommands: [
    'add-dependency - Add dependency relationships between work units',
    'remove-dependency - Remove dependency relationships',
    'query-bottlenecks - Find critical path blockers',
    'suggest-dependencies - Auto-suggest dependencies based on patterns',
    'export-dependencies - Export dependency graph (JSON/Mermaid)',
    'show-work-unit - Show complete work unit details (includes dependencies)',
  ],
  notes: [
    'Simplified Command (BUG-019):',
    '  - This command was simplified from multi-action to single-purpose',
    '  - Old: fspec dependencies <action> <work-unit-id>',
    '  - New: fspec dependencies <work-unit-id>',
    '  - Other actions now use dedicated commands (add-dependency, remove-dependency, etc.)',
    '',
    'Relationship types displayed:',
    '  blocks: Work units blocked by this work unit',
    '  blockedBy: Work units blocking this work unit',
    '  dependsOn: Soft dependencies (ordering hints)',
    '  relatesTo: Related work units (no blocking)',
    '',
    'Error handling:',
    '  - Non-existent work units show AI-friendly system-reminder',
    '  - System-reminder includes troubleshooting steps',
    '  - Wrapped for visibility in Claude Code',
    '',
    'For comprehensive dependency management:',
    '  - add-dependency: Create relationships',
    '  - remove-dependency: Delete relationships',
    '  - clear-dependencies: Remove all relationships',
    '  - export-dependencies: Visualize full graph',
    '  - query-bottlenecks: Find critical path issues',
  ],
};

export default config;
