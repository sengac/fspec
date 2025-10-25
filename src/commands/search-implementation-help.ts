import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'search-implementation',
  description: 'Search implementation code for specific function usage across work units',
  usage: 'fspec search-implementation --function=<name> [options]',
  options: [
    {
      flag: '--function <name>',
      description: 'Function name to search for',
      required: true,
    },
    {
      flag: '--show-work-units',
      description: 'Display which work units use each file',
    },
    {
      flag: '--json',
      description: 'Output results in JSON format',
    },
  ],
  whenToUse: [
    'Finding all places where a specific utility function is used',
    'Impact analysis before refactoring a function',
    'Identifying code reuse opportunities',
    'Detecting manual implementations that could use existing utilities',
  ],
  examples: [
    {
      command: 'fspec search-implementation --function=loadConfig',
      description: 'Find all uses of loadConfig function',
      output: `Found loadConfig in 8 files:

src/commands/init.ts (line 45)
src/commands/validate.ts (line 23)
src/utils/config-manager.ts (line 12)
...`,
    },
    {
      command: 'fspec search-implementation --function=loadConfig --show-work-units',
      description: 'Show which work units use the function',
      output: `loadConfig usage across work units:

src/commands/init.ts
  └─ INIT-001: Initialize fspec project

src/commands/validate.ts
  └─ CLI-003: Validate feature files

src/utils/config-manager.ts
  └─ CONFIG-001: Configuration utilities`,
    },
    {
      command: 'fspec search-implementation --function="readFile|writeFile" --json',
      description: 'Search with pattern matching',
      output: `{
  "function": "readFile|writeFile",
  "files": [
    {
      "path": "src/commands/create-feature.ts",
      "lines": [23, 45],
      "workUnits": ["CLI-001"]
    }
  ]
}`,
    },
  ],
  commonPatterns: [
    {
      title: 'Find manual file operations that could use utilities',
      commands: [
        'fspec search-implementation --function="readFile" --show-work-units',
        'fspec search-implementation --function="writeFile" --show-work-units',
      ],
    },
    {
      title: 'Impact analysis before refactoring',
      commands: [
        'fspec search-implementation --function=parseGherkin',
        'fspec search-implementation --function=formatFeature',
      ],
    },
  ],
  relatedCommands: ['compare-implementations', 'search-scenarios', 'show-coverage'],
  notes: [
    'Searches implementation files linked in coverage data',
    'Uses simple text search (not AST analysis)',
    'Supports regex patterns for advanced matching',
    'Helps identify opportunities to replace manual code with existing utilities',
  ],
};

export default config;
