import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'update-foundation',
  description:
    'Update a field in foundation.json (or foundation.json.draft during discovery)',
  usage: 'fspec update-foundation <section> <content>',
  whenToUse:
    'Use this command during draft-driven discovery workflow (fspec discover-foundation) to fill placeholder fields, OR to update an existing foundation.json. The command automatically detects which file to update (draft takes precedence over final foundation).',
  prerequisites: [
    'EITHER spec/foundation.json.draft (during discovery) OR spec/foundation.json (after discovery) exists',
  ],
  arguments: [
    {
      name: 'section',
      description:
        'Section name (one of: projectName, projectVision, projectType, problemTitle, problemDefinition, problemImpact, solutionOverview)',
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
      description:
        'Update the project name (updates draft if one exists, otherwise updates foundation.json)',
      output:
        '✓ Updated "projectName" in foundation.json.draft\n  Updated: spec/foundation.json.draft',
    },
    {
      command:
        'fspec update-foundation projectVision "CLI tool for AI agents to manage Gherkin specs using ACDD"',
      description: 'Update the project vision',
      output:
        '✓ Updated "projectVision" section in FOUNDATION.md\n  Updated: spec/foundation.json\n  Regenerated: spec/FOUNDATION.md',
    },
    {
      command: 'fspec update-foundation projectType "cli-tool"',
      description:
        'Update the project type (freeform short descriptor, 1-30 characters)',
      output:
        '✓ Updated "projectType" in foundation.json.draft\n  Updated: spec/foundation.json.draft',
    },
    {
      command:
        'fspec update-foundation problemTitle "Specification Management"',
      description: 'Update the primary problem title',
      output:
        '✓ Updated "problemTitle" in foundation.json.draft\n  Updated: spec/foundation.json.draft',
    },
    {
      command:
        'fspec update-foundation problemDefinition "AI agents lack structured workflow for spec management"',
      description: 'Update the primary problem description',
      output:
        '✓ Updated "problemDefinition" in foundation.json.draft\n  Updated: spec/foundation.json.draft',
    },
    {
      command: 'fspec update-foundation problemImpact "high"',
      description:
        'Update the problem impact (must be one of: high, medium, low)',
      output:
        '✓ Updated "problemImpact" in foundation.json.draft\n  Updated: spec/foundation.json.draft',
    },
    {
      command:
        'fspec update-foundation solutionOverview "Standardized CLI with Gherkin specs and ACDD workflow"',
      description: 'Update the solution overview',
      output:
        '✓ Updated "solutionOverview" section in FOUNDATION.md\n  Updated: spec/foundation.json\n  Regenerated: spec/FOUNDATION.md',
    },
  ],
  commonErrors: [
    {
      error:
        'Error: Invalid projectType: "" (must be 1-30 characters, got 0). Fix: fspec update-foundation projectType "<short-descriptor>"',
      fix: 'Provide a non-empty projectType (e.g. "cli-tool", "web-app", "saas-platform")',
    },
    {
      error:
        'Error: Invalid projectType: too long (must be 1-30 characters, got N). Fix: fspec update-foundation projectType "<short-descriptor>"',
      fix: 'Shorten the projectType value to 30 characters or fewer',
    },
    {
      error:
        'Error: Invalid value for problemImpact: "critical". Valid values: high, medium, low. Fix: fspec update-foundation problemImpact "<valid-value>"',
      fix: 'Use one of the exact values: high, medium, low',
    },
    {
      error:
        'Error: Unknown section: "invalidSection". Use field names like: projectName, projectVision, projectType, problemTitle, problemDefinition, problemImpact, solutionOverview',
      fix: 'Run `fspec list-foundation-sections` to see every valid section name with its JSON path and constraint',
    },
    {
      error: 'Error: Section name cannot be empty',
      fix: 'Provide a section name as the first argument',
    },
    {
      error: 'Error: Section content cannot be empty',
      fix: 'Provide content as the second argument (use quotes for multi-word content)',
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
        'fspec update-foundation problemTitle "Spec Management"\nfspec update-foundation problemDefinition "AI agents lack structured workflow"\nfspec update-foundation problemImpact "high"\nfspec update-foundation solutionOverview "CLI tool with Gherkin specs and ACDD workflow"',
    },
  ],
  relatedCommands: [
    'show-foundation',
    'list-foundation-sections',
    'validate-foundation-schema',
    'generate-foundation-md',
    'discover-foundation',
    'add-capability',
    'add-persona',
  ],
  notes: [
    'CRITICAL: Command automatically detects which file to update (draft vs final)',
    'If foundation.json.draft exists: Updates draft ONLY (no validation, no FOUNDATION.md regeneration)',
    'If foundation.json.draft does NOT exist: Updates foundation.json and regenerates FOUNDATION.md',
    'Draft workflow: Use during discovery to fill [QUESTION:] placeholders',
    'Final workflow: Use after discovery to update existing foundation',
    'Valid section names (exhaustive list): projectName, projectVision, projectType, problemTitle, problemDefinition, problemImpact, solutionOverview',
    'NOT supported by update-foundation: capabilities are managed via the `add-capability` command (with matching remove-capability)',
    'NOT supported by update-foundation: personas are managed via the `add-persona` command (with matching remove-persona)',
    'To see every valid section with its JSON path and constraints, run: fspec list-foundation-sections',
    'projectType is a freeform short string (1-30 characters); examples include cli-tool, web-app, saas-platform',
    'problemImpact is a strict enum with exactly three valid values: high, medium, low',
    'Use quotes around multi-word content: "This is the content"',
  ],
};

export default config;
