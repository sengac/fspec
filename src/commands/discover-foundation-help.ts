import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'discover-foundation',
  description:
    'Discover project foundation by analyzing codebase and running interactive questionnaire',
  usage: 'fspec discover-foundation [options]',
  whenToUse:
    'Use when starting a new project with fspec to automatically generate foundation.json. Analyzes existing code to detect project type, personas, and capabilities, then runs interactive questionnaire to gather WHY/WHAT information.',
  options: [
    {
      flag: '--output <path>',
      description:
        'Output path for generated foundation.json (default: spec/foundation.json)',
    },
  ],
  workflow: [
    '1. Analyze codebase using automated discovery patterns',
    '2. Detect project type (CLI tool, web app, library, etc.)',
    '3. Identify user personas from routes/commands/exports',
    '4. Infer capabilities focusing on WHAT not HOW',
    '5. Run interactive questionnaire with prefilled answers',
    '6. Generate foundation.json with v2.0.0 schema',
    '7. Validate generated foundation against schema',
  ],
  examples: [
    {
      command: 'fspec discover-foundation',
      description: 'Run full discovery workflow',
      output: `Analyzing codebase...
✓ Detected project type: cli-tool
✓ Found 1 persona: Developer using CLI
✓ Identified 12 capabilities

Running questionnaire...
? What is the project vision? [DETECTED: CLI tool for spec management]
...

✓ Generated spec/foundation.json
✓ Validation passed`,
    },
    {
      command: 'fspec discover-foundation --output foundation.json',
      description: 'Generate foundation in project root',
      output: '✓ Generated foundation.json',
    },
  ],
  commonPatterns: [
    {
      title: 'Discovery for CLI tools',
      description:
        'Detects commander.js usage, bin field in package.json, command definitions',
    },
    {
      title: 'Discovery for web apps',
      description:
        'Detects Express routes, React components, identifies End User and API Consumer personas',
    },
    {
      title: 'Discovery for libraries',
      description:
        'Detects exports field in package.json, identifies Developer persona',
    },
  ],
  relatedCommands: [
    'update-foundation',
    'show-foundation',
    'migrate-foundation',
    'generate-foundation-md',
  ],
  notes: [
    'Uses code analysis guidance from FOUND-002 (automated discovery)',
    'Questionnaire prefills answers detected from code analysis',
    'Focuses on WHY (problems) and WHAT (capabilities), not HOW (implementation)',
    'Generated foundation.json uses v2.0.0 generic schema',
    'Validation ensures schema compliance before writing file',
    'Creates backup of existing foundation.json if present',
  ],
};

export default config;
