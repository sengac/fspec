import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'update-foundation',
  description: 'Update section content in foundation.json or foundation.json.draft during discovery',
  usage: 'fspec update-foundation <section> <content>',
  whenToUse:
    'Use this command during draft-driven discovery workflow (fspec discover-foundation) to fill placeholder fields, OR to update an existing foundation.json. The command automatically detects which file to update.',
  prerequisites: ['EITHER spec/foundation.json.draft (during discovery) OR spec/foundation.json (after discovery) exists'],
  arguments: [
    {
      name: 'section',
      description: 'Section name or field path (e.g., "projectVision", "problemDefinition", "solutionOverview")',
      required: true,
    },
    {
      name: 'content',
      description: 'New content for the section (can be multi-line text)',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command: 'fspec update-foundation projectName "My Project"',
      description: 'Update project name during discovery (updates draft if it exists)',
      output: '✓ Updated "projectName" in foundation.json.draft\n  Updated: spec/foundation.json.draft',
    },
    {
      command: 'fspec update-foundation projectVision "CLI tool for AI agents to manage Gherkin specs using ACDD"',
      description: 'Update project vision (after discovery completes)',
      output: '✓ Updated "projectVision" section in FOUNDATION.md\n  Updated: spec/foundation.json\n  Regenerated: spec/FOUNDATION.md',
    },
    {
      command: 'fspec update-foundation problemDefinition "AI agents lack structured workflow for spec management"',
      description: 'Update problem definition during discovery',
      output: '✓ Updated "problemDefinition" in foundation.json.draft\n  Updated: spec/foundation.json.draft',
    },
    {
      command: 'fspec update-foundation solutionOverview "Standardized CLI with Gherkin specs and ACDD workflow"',
      description: 'Update solution overview',
      output: '✓ Updated "solutionOverview" section in FOUNDATION.md\n  Updated: spec/foundation.json\n  Regenerated: spec/FOUNDATION.md',
    },
  ],
  commonErrors: [
    {
      error: 'Error: Unknown section: "invalidSection". Use field names like: projectOverview, problemDefinition, etc.',
      fix: 'Use valid section names. Common sections: projectName, projectVision, projectType, problemDefinition, problemImpact, solutionOverview',
    },
    {
      error: 'Error: Section name cannot be empty',
      fix: 'Provide a section name as the first argument',
    },
    {
      error: 'Error: Section content cannot be empty',
      fix: 'Provide content as the second argument (use quotes for multi-word content)',
    },
    {
      error: 'Error: Updated foundation.json failed schema validation: ...',
      fix: 'Content must comply with foundation schema. Check error message for specific validation issue.',
    },
  ],
  typicalWorkflow:
    'DURING DISCOVERY: 1. Create draft: fspec discover-foundation → 2. Fill fields: fspec update-foundation <section> <content> (updates draft) → 3. Repeat for all fields → 4. Finalize: fspec discover-foundation --finalize\n\nAFTER DISCOVERY: 1. Review: fspec show-foundation → 2. Update: fspec update-foundation <section> <content> (updates final) → 3. Verify: fspec show-foundation',
  commonPatterns: [
    {
      pattern: 'Update Project Metadata',
      example:
        'fspec update-foundation projectName "fspec"\nfspec update-foundation projectVision "Standardized CLI for AI agent spec management"\nfspec update-foundation projectType "cli-tool"',
    },
    {
      pattern: 'Update Problem/Solution Space',
      example:
        'fspec update-foundation problemDefinition "AI agents lack structured workflow"\nfspec update-foundation solutionOverview "CLI tool with Gherkin specs and ACDD workflow"',
    },
  ],
  relatedCommands: [
    'show-foundation',
    'validate-foundation-schema',
    'generate-foundation-md',
    'discover-foundation',
  ],
  notes: [
    'CRITICAL: Command automatically detects which file to update (draft vs final)',
    'If foundation.json.draft exists: Updates draft ONLY (no validation, no FOUNDATION.md regeneration)',
    'If foundation.json.draft does NOT exist: Updates foundation.json and regenerates FOUNDATION.md',
    'Draft workflow: Use during discovery to fill [QUESTION:] placeholders',
    'Final workflow: Use after discovery to update existing foundation',
    'Supported section names: projectName, projectVision, projectType, problemDefinition, problemImpact, solutionOverview',
    'Legacy section names (testingStrategy, developmentTools, etc.) map to solutionOverview for backward compatibility',
    'Use quotes around multi-word content: "This is the content"',
  ],
};

export default config;
