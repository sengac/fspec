import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'dependencies',
  description:
    'Comprehensive dependency management for work units (add, remove, list, validate, repair, analyze, visualize)',
  usage:
    'fspec dependencies <action> [work-unit-id] [options]\n\nActions:\n  add         - Add dependency relationship\n  remove      - Remove dependency relationship\n  list        - List dependencies for work unit\n  validate    - Validate dependency graph consistency\n  repair      - Auto-repair bidirectional relationships\n  graph       - Export dependency graph (JSON/Mermaid)\n  critical    - Calculate critical path\n  impact      - Analyze impact of work unit changes',
  whenToUse:
    'Use this command for comprehensive dependency management: creating relationships, validating graph consistency, visualizing dependencies, calculating critical paths, and analyzing impact. Central command for understanding work unit relationships and project structure.',
  prerequisites: ['spec/work-units.json exists with work units'],
  arguments: [
    {
      name: 'action',
      description:
        'Action to perform: add, remove, list, validate, repair, graph, critical, impact',
      required: true,
    },
    {
      name: 'work-unit-id',
      description: 'Work unit ID (required for add, remove, list, impact)',
      required: false,
    },
  ],
  options: [
    {
      flag: '--blocks <id>',
      description: 'Add blocks relationship (blocker blocks target)',
    },
    {
      flag: '--blocked-by <id>',
      description: 'Add blockedBy relationship (blocker must complete first)',
    },
    {
      flag: '--depends-on <id>',
      description: 'Add dependsOn relationship (soft dependency)',
    },
    {
      flag: '--relates-to <id>',
      description: 'Add relatesTo relationship (bidirectional, no blocking)',
    },
    {
      flag: '--type <type>',
      description: 'Relationship type filter: blocks, blockedBy, dependsOn, relatesTo, all',
    },
    {
      flag: '--format <format>',
      description: 'Output format for graph: json, mermaid',
    },
    {
      flag: '--graph',
      description: 'Display dependencies as graph visualization',
    },
  ],
  examples: [
    {
      command: 'fspec dependencies add AUTH-002 --depends-on AUTH-001',
      description: 'Add dependency: AUTH-002 depends on AUTH-001',
      output: '✓ Added dependency: AUTH-002 depends on AUTH-001',
    },
    {
      command: 'fspec dependencies add AUTH-001 --blocks AUTH-002',
      description: 'Add blocking relationship (bidirectional)',
      output:
        '✓ Added dependency: AUTH-001 blocks AUTH-002\n✓ Bidirectional: AUTH-002 blocked by AUTH-001',
    },
    {
      command: 'fspec dependencies list AUTH-001',
      description: 'List all dependencies for work unit',
      output:
        'Dependencies for AUTH-001:\n  Blocks: AUTH-002, AUTH-003\n  Blocked by: INFRA-001\n  Depends on: SCHEMA-001\n  Related to: DOC-001',
    },
    {
      command: 'fspec dependencies validate',
      description: 'Validate dependency graph consistency',
      output:
        '✓ Dependency graph is valid\n  Checked 45 work units\n  Verified 87 relationships\n  No errors found',
    },
    {
      command: 'fspec dependencies repair',
      description: 'Auto-repair bidirectional relationships',
      output: '✓ Repaired 3 relationship(s)\n  Fixed blocks/blockedBy bidirectional links',
    },
    {
      command: 'fspec dependencies graph --format mermaid',
      description: 'Export dependency graph as Mermaid diagram',
      output:
        'graph TD\n  AUTH-001[AUTH-001] -->|blocks| AUTH-002[AUTH-002]\n  AUTH-001[AUTH-001] -.->|depends on| SCHEMA-001[SCHEMA-001]\n  AUTH-002[AUTH-002] <-->|relates to| DOC-001[DOC-001]',
    },
    {
      command: 'fspec dependencies critical',
      description: 'Calculate critical path through work units',
      output:
        'Critical Path:\n  SCHEMA-001 → AUTH-001 → AUTH-002 → AUTH-003 → TEST-001\n\nPath length: 5 work units\nEstimated effort: 23 story points',
    },
    {
      command: 'fspec dependencies impact AUTH-001',
      description: 'Analyze impact of changes to work unit',
      output:
        'Impact Analysis for AUTH-001:\n\nDirectly Affected: 2 work units\n  - AUTH-002 (blocked by AUTH-001)\n  - AUTH-003 (blocked by AUTH-001)\n\nTransitively Affected: 5 work units\n  - TEST-001, TEST-002, DEPLOY-001, DOC-001, DOC-002\n\nTotal Affected: 7 work units',
    },
  ],
  commonErrors: [
    {
      error: 'Error: Circular dependency detected: AUTH-001 → AUTH-002 → AUTH-001',
      fix: 'Circular dependencies are not allowed. Restructure dependencies to break the cycle.',
    },
    {
      error: 'Error: Dependency already exists',
      fix: 'This dependency relationship already exists. Use remove first if you need to change it.',
    },
    {
      error: 'Error: Cannot create dependency to self',
      fix: 'Work units cannot depend on themselves. Specify a different target work unit.',
    },
    {
      error: 'Error: Work unit does not exist',
      fix: 'Verify work unit IDs exist. Run: fspec list-work-units',
    },
    {
      error: 'Error: 5 relationship(s) have inconsistent bidirectional links',
      fix: 'Run: fspec dependencies repair to auto-fix bidirectional consistency',
    },
  ],
  typicalWorkflow:
    '1. Add dependencies as you create work units → 2. Validate: fspec dependencies validate → 3. Repair if needed: fspec dependencies repair → 4. Visualize: fspec dependencies graph --format mermaid → 5. Analyze impact before changes: fspec dependencies impact <id>',
  commonPatterns: [
    {
      pattern: 'Sequential Dependency Setup',
      example:
        '# Create sequential work units\nfspec create-work-unit AUTH-001 "Setup auth infrastructure"\nfspec create-work-unit AUTH-002 "Add login endpoint"\nfspec create-work-unit AUTH-003 "Add logout endpoint"\n\n# Add dependencies\nfspec dependencies add AUTH-002 --depends-on AUTH-001\nfspec dependencies add AUTH-003 --depends-on AUTH-001\n\n# Validate\nfspec dependencies validate',
    },
    {
      pattern: 'Blocking Relationship',
      example:
        '# AUTH-001 must complete before AUTH-002 can start\nfspec dependencies add AUTH-001 --blocks AUTH-002\n\n# This automatically:\n# - Sets AUTH-002 blockedBy AUTH-001 (bidirectional)\n# - Sets AUTH-002 status to "blocked" if AUTH-001 not done',
    },
    {
      pattern: 'Cross-Cutting Relationships',
      example:
        '# Feature relates to documentation (no blocking)\nfspec dependencies add AUTH-001 --relates-to DOC-AUTH-001\n\n# This creates bidirectional relatesTo (both directions)',
    },
    {
      pattern: 'Dependency Maintenance',
      example:
        '# Weekly validation\nfspec dependencies validate\n\n# If errors found, repair\nfspec dependencies repair\n\n# Visualize current state\nfspec dependencies graph --format mermaid > dependencies.mmd',
    },
  ],
  relatedCommands: [
    'add-dependency',
    'remove-dependency',
    'query-bottlenecks',
    'suggest-dependencies',
    'update-work-unit-status',
    'show-work-unit',
  ],
  notes: [
    'Relationship types:',
    '  blocks: Hard blocking (blocker must complete first, sets status to "blocked")',
    '  blockedBy: Inverse of blocks (automatically bidirectional)',
    '  dependsOn: Soft dependency (no status change, for ordering)',
    '  relatesTo: Bidirectional relationship (no blocking, for context)',
    'Bidirectional enforcement:',
    '  - blocks ↔ blockedBy automatically synchronized',
    '  - relatesTo automatically bidirectional',
    'Circular dependency prevention:',
    '  - blocks/blockedBy relationships cannot form cycles',
    '  - dependsOn/relatesTo can have cycles (soft relationships)',
    'Auto-blocking:',
    '  - Adding blockedBy automatically sets status to "blocked" if blocker not done',
    '  - Completing blocker automatically unblocks dependent work',
    'Validation checks:',
    '  - Bidirectional consistency (blocks ↔ blockedBy)',
    '  - Non-existent work unit references',
    '  - Orphaned relationships',
  ],
};

export default config;
