import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'create-bug',
  description:
    'Create a new bug with research guidance for understanding existing code before fixing',
  usage: 'fspec create-bug <prefix> <title> [options]',
  whenToUse:
    'Use when tracking bugs that require researching existing scenarios, implementation, and test coverage before fixing to prevent regression.',
  arguments: [
    {
      name: 'prefix',
      description:
        'Bug prefix (e.g., BUG, FIX, HOTFIX). Must be registered with create-prefix first.',
      required: true,
    },
    {
      name: 'title',
      description: 'Brief description of the bug',
      required: true,
    },
  ],
  options: [
    {
      flag: '-d, --description <description>',
      description: 'Detailed description of the bug',
    },
    {
      flag: '-e, --epic <epic>',
      description: 'Epic ID to associate with this bug',
    },
    {
      flag: '-p, --parent <parent>',
      description: 'Parent bug ID for hierarchical relationships',
    },
  ],
  examples: [
    {
      command: 'fspec create-bug BUG "Login validation broken"',
      description: 'Create simple bug with research guidance',
      output:
        '✓ Created bug BUG-001\n  Title: Login validation broken\n\n<system-reminder>\nBug BUG-001 created successfully.\n\nCRITICAL: Research existing code FIRST before fixing bugs.\n  fspec search-scenarios --query="login"\n  fspec search-implementation --function="validateLogin"\n  fspec show-coverage\n</system-reminder>',
    },
    {
      command:
        'fspec create-bug BUG "Memory leak in dashboard" --epic=performance',
      description: 'Create bug with epic',
      output:
        '✓ Created bug BUG-002\n  Title: Memory leak in dashboard\n  Epic: performance',
    },
    {
      command:
        'fspec create-bug HOTFIX "Critical auth bypass" --description="CVE-2024-1234"',
      description: 'Create critical bug with description',
      output:
        '✓ Created bug HOTFIX-001\n  Title: Critical auth bypass\n  Description: CVE-2024-1234',
    },
  ],
  prerequisites: [
    'Prefix must be registered: fspec create-prefix PREFIX "Description"',
    'Epic must exist if using --epic: fspec create-epic EPIC "Title"',
    'Parent bug must exist if using --parent',
  ],
  typicalWorkflow: [
    'Create bug: fspec create-bug PREFIX "Title"',
    'Follow research guidance from system-reminder',
    'Search scenarios: fspec search-scenarios --query="keyword"',
    'Search implementation: fspec search-implementation --function="functionName"',
    'Check coverage: fspec show-coverage feature-name',
    'Add reproduction steps: fspec add-example BUG-001 "Reproduction steps"',
    'Add fix scenarios: fspec add-rule BUG-001 "Expected behavior"',
    'Move to specifying: fspec update-work-unit-status BUG-001 specifying',
  ],
  commonErrors: [
    {
      error: "Prefix 'PREFIX' is not registered",
      solution: 'Run: fspec create-prefix PREFIX "Description"',
    },
    {
      error: "Parent bug 'PARENT-001' does not exist",
      solution: 'Create parent first or remove --parent option',
    },
    {
      error: "Epic 'epic-name' does not exist",
      solution: 'Run: fspec create-epic epic-name "Title"',
    },
  ],
  relatedCommands: [
    'fspec search-scenarios - Search existing scenarios by keyword',
    'fspec search-implementation - Search implementation for function usage',
    'fspec show-coverage - Check test coverage for features',
    'fspec list-features - List all feature files',
    'fspec add-rule - Add expected behavior rule',
    'fspec add-example - Add reproduction steps',
    'fspec update-work-unit-status - Move bug through ACDD workflow',
  ],
  notes: [
    'Bugs require research BEFORE fixing to prevent regressions',
    'System-reminder guides AI agents to use research commands',
    'Bugs should link to existing features when fixing behavior',
    'Bugs may or may not require new tests (depends on coverage gaps)',
  ],
};

export default config;
