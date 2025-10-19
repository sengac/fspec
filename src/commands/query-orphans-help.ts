import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'query-orphans',
  description:
    'Detect orphaned work units with no epic assignment or dependency relationships',
  usage: 'fspec query-orphans [options]',
  whenToUse:
    'Use this command to identify work units lacking context and traceability. Essential for work unit hygiene, preventing unplanned scope creep, and ensuring all work aligns with epics or has clear relationships. Run after bulk work unit creation or periodically for maintenance.',
  prerequisites: ['spec/work-units.json exists with work units'],
  arguments: [],
  options: [
    {
      flag: '--output <format>',
      description: 'Output format: json or text',
      defaultValue: 'text',
    },
    {
      flag: '--exclude-done',
      description: 'Exclude work units in done status from orphan detection',
    },
  ],
  examples: [
    {
      command: 'fspec query-orphans',
      description: 'Find all orphaned work units (no epic, no relationships)',
      output:
        'Found 3 orphaned work unit(s):\n\n1. MISC-001 - Update documentation (backlog)\n   ⚠ No epic or dependency relationships\n   Suggested actions:\n     • Assign epic\n     • Add relationship\n     • Delete\n\n2. REFAC-003 - Refactor auth module (implementing)\n   ⚠ No epic or dependency relationships\n   Suggested actions:\n     • Assign epic\n     • Add relationship\n     • Delete\n\nTo fix orphaned work units:\n  fspec update-work-unit <id> --epic=<epic-name>\n  fspec add-dependency <id> --depends-on=<other-id>  (or --blocks, --relates-to)\n  fspec delete-work-unit <id>',
    },
    {
      command: 'fspec query-orphans --exclude-done',
      description: 'Find orphans excluding completed work',
      output:
        'Found 1 orphaned work unit(s):\n\n1. MISC-001 - Update documentation (backlog)\n   ⚠ No epic or dependency relationships\n   Suggested actions:\n     • Assign epic\n     • Add relationship\n     • Delete',
    },
    {
      command: 'fspec query-orphans --output json',
      description: 'Output orphans as JSON for automation',
      output:
        '{\n  "orphans": [\n    {\n      "id": "MISC-001",\n      "title": "Update documentation",\n      "status": "backlog",\n      "suggestedActions": ["Assign epic", "Add relationship", "Delete"]\n    }\n  ]\n}',
    },
    {
      command: 'fspec query-orphans | wc -l',
      description: 'Count orphaned work units',
      output: '15',
    },
  ],
  commonErrors: [
    {
      error: 'Error: No orphaned work units found',
      fix: 'All work units have epic or dependency relationships. No action needed.',
    },
    {
      error: 'Error: work-units.json not found',
      fix: 'Run: fspec init to create work-units.json file',
    },
    {
      error: 'Error: Invalid work-units.json format',
      fix: 'Check for JSON syntax errors in spec/work-units.json',
    },
  ],
  typicalWorkflow:
    '1. Run orphan detection: fspec query-orphans → 2. Review orphaned work units → 3. Assign epic OR add relationship: fspec update-work-unit <id> --epic=<epic> OR fspec add-dependency <id> --relates-to=<other-id> → 4. Delete if invalid: fspec delete-work-unit <id> → 5. Verify: fspec query-orphans',
  commonPatterns: [
    {
      pattern: 'Maintenance Workflow',
      example:
        '# Weekly cleanup: find orphans\nfspec query-orphans\n\n# Assign epics to valid work\nfspec update-work-unit MISC-001 --epic=documentation\nfspec update-work-unit REFAC-003 --epic=authentication\n\n# Add relationships for cross-cutting work\nfspec add-dependency MISC-001 --relates-to AUTH-001\n\n# Delete invalid work units\nfspec delete-work-unit INVALID-001\n\n# Verify cleanup\nfspec query-orphans',
    },
    {
      pattern: 'Bulk Work Unit Validation',
      example:
        '# After creating multiple work units\nfspec create-work-unit AUTH-001 "Setup auth"\nfspec create-work-unit AUTH-002 "Login flow"\nfspec create-work-unit AUTH-003 "Logout flow"\n\n# Check for orphans\nfspec query-orphans\n\n# Assign epic to all\nfspec update-work-unit AUTH-001 --epic=authentication\nfspec update-work-unit AUTH-002 --epic=authentication\nfspec update-work-unit AUTH-003 --epic=authentication',
    },
    {
      pattern: 'JSON Export for Reporting',
      example:
        "# Export orphans for dashboard\nfspec query-orphans --output json > orphans.json\n\n# Count orphans by status\njq '[.orphans[].status] | group_by(.) | map({status: .[0], count: length})' orphans.json",
    },
  ],
  relatedCommands: [
    'query-bottlenecks',
    'suggest-dependencies',
    'update-work-unit',
    'add-dependency',
    'delete-work-unit',
    'list-epics',
  ],
  notes: [
    'A work unit is orphaned if it has NEITHER epic NOR dependency relationships',
    'Orphaned work units lack context and traceability (unclear WHY they exist)',
    'By default, includes ALL statuses (even "done" work can be orphaned)',
    'Use --exclude-done to focus on active work units needing attention',
    'Orphans often indicate ad-hoc work added without proper planning',
    'Suggested actions: (1) Assign epic, (2) Add relationship, (3) Delete if invalid',
    'Regular orphan detection prevents unplanned scope creep',
  ],
};

export default config;
