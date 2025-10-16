import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-architecture-note',
  description: 'Add architecture note to work unit during Example Mapping',
  usage: 'fspec add-architecture-note <workUnitId> <note>',
  whenToUse:
    'Use during Example Mapping (specifying phase) to capture architecture decisions, non-functional requirements, dependencies, performance constraints, security considerations, or implementation patterns that should appear in the generated feature file docstring.',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID (e.g., WORK-001)',
      required: true,
    },
    {
      name: 'note',
      description:
        'Architecture note text. Optionally prefix with category (e.g., "Dependency:", "Performance:", "Refactoring:", "Security:", "UI/UX:", "Implementation:")',
      required: true,
    },
  ],
  examples: [
    {
      command:
        'fspec add-architecture-note WORK-001 "Uses @cucumber/gherkin parser"',
      description: 'Add general architecture note',
      output: '✓ Architecture note added successfully',
    },
    {
      command:
        'fspec add-architecture-note WORK-001 "Dependency: @cucumber/gherkin parser"',
      description: 'Add dependency note (will be categorized in docstring)',
      output: '✓ Architecture note added successfully',
    },
    {
      command:
        'fspec add-architecture-note WORK-001 "Performance: Must complete validation within 2 seconds"',
      description: 'Add performance requirement',
      output: '✓ Architecture note added successfully',
    },
    {
      command:
        'fspec add-architecture-note WORK-001 "Refactoring: Share validation logic with formatter"',
      description: 'Add refactoring note',
      output: '✓ Architecture note added successfully',
    },
  ],
  typicalWorkflow:
    'During Example Mapping: 1) Ask architecture questions, 2) Capture notes with add-architecture-note, 3) Run generate-scenarios to create feature file with populated docstring',
  commonPatterns: [
    'Prefix notes with "Dependency:" for libraries and integrations',
    'Prefix with "Performance:" for speed/throughput requirements',
    'Prefix with "Refactoring:" for code sharing opportunities',
    'Prefix with "Security:" for security considerations',
    'Prefix with "UI/UX:" for design references or examples',
    'Prefix with "Implementation:" for specific patterns to follow',
    'Notes without prefixes go into "General" category',
  ],
  relatedCommands: [
    'show-work-unit',
    'remove-architecture-note',
    'generate-scenarios',
    'add-assumption',
  ],
  notes: [
    'Architecture notes are stored in work unit and used to populate feature file docstrings',
    'Notes with recognized prefixes are automatically categorized in generated docstrings',
    'View captured notes with: fspec show-work-unit <workUnitId>',
    'Notes appear in generated feature files when running generate-scenarios',
  ],
};

export default config;
