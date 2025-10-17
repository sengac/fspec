import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'update-foundation',
  description: 'Update section content in foundation.json and regenerate FOUNDATION.md',
  usage: 'fspec update-foundation <section> <content>',
  whenToUse:
    'Use this command to programmatically update foundation.json sections (project vision, problem definition, solution overview, etc.) and automatically regenerate FOUNDATION.md from the updated JSON.',
  prerequisites: ['spec/foundation.json exists (created by fspec init or discover-foundation)'],
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
      command: 'fspec update-foundation projectVision "CLI tool for AI agents to manage Gherkin specs using ACDD"',
      description: 'Update project vision',
      output: '✓ Updated "projectVision" section in FOUNDATION.md\n  Updated: spec/foundation.json\n  Regenerated: spec/FOUNDATION.md',
    },
    {
      command: 'fspec update-foundation problemDefinition "AI agents lack structured workflow for spec management"',
      description: 'Update problem definition',
      output: '✓ Updated "problemDefinition" section in FOUNDATION.md\n  Updated: spec/foundation.json\n  Regenerated: spec/FOUNDATION.md',
    },
    {
      command: 'fspec update-foundation solutionOverview "Standardized CLI with Gherkin specs and ACDD workflow"',
      description: 'Update solution overview',
      output: '✓ Updated "solutionOverview" section in FOUNDATION.md\n  Updated: spec/foundation.json\n  Regenerated: spec/FOUNDATION.md',
    },
    {
      command: 'fspec update-foundation projectName "fspec"',
      description: 'Update project name',
      output: '✓ Updated "projectName" section in FOUNDATION.md\n  Updated: spec/foundation.json\n  Regenerated: spec/FOUNDATION.md',
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
    '1. Review current foundation: fspec show-foundation → 2. Update section: fspec update-foundation <section> <content> → 3. Verify: fspec show-foundation → 4. Validate: fspec validate-foundation-schema',
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
    'migrate-foundation',
  ],
  notes: [
    'Automatically regenerates FOUNDATION.md from foundation.json after update',
    'Validates updated JSON against foundation schema before writing',
    'Supported section names: projectName, projectVision, projectType, problemDefinition, problemImpact, solutionOverview',
    'Legacy section names (testingStrategy, developmentTools, etc.) map to solutionOverview for backward compatibility',
    'Use quotes around multi-word content: "This is the content"',
    'Changes are written to both spec/foundation.json and spec/FOUNDATION.md',
  ],
};

export default config;
