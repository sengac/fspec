import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-attachment',
  description:
    'Add an attachment to a work unit during Example Mapping. Attachments are stored in spec/attachments/<workUnitId>/ and tracked as relative paths in the work unit.',
  usage: 'fspec add-attachment <workUnitId> <filePath> [options]',
  whenToUse:
    'Use attachments to supplement architecture notes and non-functional requirements with visual context (architecture diagrams, UI mockups, API documentation, screenshots, reference documents, or any supporting materials).',
  arguments: [
    {
      name: 'workUnitId',
      description:
        'The ID of the work unit to attach the file to (e.g., AUTH-001, DASH-002)',
      required: true,
    },
    {
      name: 'filePath',
      description:
        'Path to the file to attach. Can be absolute or relative to current directory. The file will be copied to spec/attachments/<workUnitId>/',
      required: true,
    },
  ],
  options: [
    {
      flag: '-d, --description <text>',
      description:
        "Optional description of the attachment (what it represents, why it's relevant)",
    },
  ],
  examples: [
    {
      command: 'fspec add-attachment AUTH-001 diagrams/auth-flow.png',
      description: 'Add architecture diagram during discovery',
      output:
        '✓ Attachment added successfully\n  File: spec/attachments/AUTH-001/auth-flow.png',
    },
    {
      command:
        'fspec add-attachment UI-002 mockups/dashboard.png --description "Dashboard layout v2"',
      description: 'Add mockup with description',
      output:
        '✓ Attachment added successfully\n  File: spec/attachments/UI-002/dashboard.png\n  Description: Dashboard layout v2',
    },
    {
      command: 'fspec add-attachment API-003 docs/stripe-api-reference.pdf',
      description: 'Add API documentation',
      output:
        '✓ Attachment added successfully\n  File: spec/attachments/API-003/stripe-api-reference.pdf',
    },
    {
      command: 'fspec add-attachment BUG-005 screenshots/error-state.png',
      description: 'Add screenshot for bug reproduction',
      output:
        '✓ Attachment added successfully\n  File: spec/attachments/BUG-005/error-state.png',
    },
  ],
  prerequisites: [
    "Work unit must exist (created with 'fspec create-story', 'fspec create-bug', or 'fspec create-task')",
    'Source file must exist at the specified path',
    "Work unit should be in 'specifying' or earlier status",
  ],
  typicalWorkflow: [
    'Create work unit: fspec create-story AUTH "User Authentication"',
    'Move to specifying: fspec update-work-unit-status AUTH-001 specifying',
    'Start Example Mapping: Add rules, examples, questions',
    'Add architecture notes: fspec add-architecture-note AUTH-001 "Uses JWT tokens"',
    'Add attachments: fspec add-attachment AUTH-001 docs/auth-flow.png',
    'Continue discovery until all questions answered',
    'Generate scenarios: fspec generate-scenarios AUTH-001',
  ],
  commonErrors: [
    {
      error: "Work unit 'AUTH-001' does not exist",
      solution:
        "Create the work unit first with 'fspec create-story', 'fspec create-bug', or 'fspec create-task'",
    },
    {
      error: "Source file 'diagram.png' does not exist",
      solution:
        'Check the file path is correct (absolute or relative to current directory)',
    },
    {
      error: "Attachment 'diagram.png' already exists",
      solution:
        "The file has already been attached to this work unit. Use 'fspec list-attachments AUTH-001' to see all attachments. Remove the old attachment first with 'fspec remove-attachment'.",
    },
  ],
  relatedCommands: [
    'fspec list-attachments - List all attachments for a work unit',
    'fspec remove-attachment - Remove attachment',
    'fspec show-work-unit - View work unit details (includes attachments)',
    'fspec add-architecture-note - Add text-based architecture notes',
    'fspec add-rule - Add business rules',
    'fspec add-example - Add examples',
  ],
  notes: [
    'Attachments are copied to spec/attachments/<workUnitId>/ directory',
    'Original files are not modified',
    'Paths are stored as relative paths from project root',
    'All file types are supported (images, PDFs, documents, etc.)',
    'Attachments are version controlled (add to git)',
    'Use attachments to supplement, not replace, architecture notes',
  ],
};

export default config;
