import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'list-attachments',
  description:
    'List all attachments for a work unit. Shows file paths, sizes, and modification dates. Verifies whether files exist on the filesystem and highlights missing files.',
  usage: 'fspec list-attachments <workUnitId>',
  whenToUse:
    'Use to see what files are attached to a work unit, to verify attachments exist on filesystem, to check file sizes and modification dates, before adding duplicate attachments, or when reviewing work unit context.',
  arguments: [
    {
      name: 'workUnitId',
      description:
        'The ID of the work unit to list attachments for (e.g., AUTH-001, DASH-002)',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command: 'fspec list-attachments AUTH-001',
      description: 'List attachments for work unit',
      output:
        'Attachments for AUTH-001 (3):\n\n  ✓ spec/attachments/AUTH-001/auth-flow.png\n    Size: 125.45 KB\n    Modified: 1/15/2025, 10:30:00 AM\n\n  ✓ spec/attachments/AUTH-001/token-refresh.png\n    Size: 89.23 KB\n    Modified: 1/15/2025, 11:00:00 AM',
    },
    {
      command: 'fspec list-attachments AUTH-002',
      description: 'No attachments',
      output: 'No attachments found for work unit AUTH-002',
    },
    {
      command: 'fspec list-attachments AUTH-003',
      description: 'Missing files detected',
      output:
        'Attachments for AUTH-003 (2):\n\n  ✓ spec/attachments/AUTH-003/diagram.png\n    Size: 45.12 KB\n    Modified: 1/14/2025, 3:00:00 PM\n\n  ✗ spec/attachments/AUTH-003/deleted-file.pdf\n    File not found on filesystem',
    },
  ],
  prerequisites: [
    "Work unit must exist (created with 'fspec create-story', 'fspec create-bug', or 'fspec create-task')",
  ],
  typicalWorkflow: [
    'View work unit details: fspec show-work-unit AUTH-001',
    'List attachments: fspec list-attachments AUTH-001',
    'Review attachment files in spec/attachments/AUTH-001/',
    'Add more attachments if needed: fspec add-attachment AUTH-001 <file>',
  ],
  commonErrors: [
    {
      error: "Work unit 'AUTH-001' does not exist",
      solution:
        "Create the work unit first with 'fspec create-story', 'fspec create-bug', or 'fspec create-task'",
    },
    {
      error: 'No attachments found for work unit AUTH-001',
      solution:
        "The work unit exists but has no attachments. Add attachments with 'fspec add-attachment AUTH-001 <file>'.",
    },
  ],
  relatedCommands: [
    'fspec add-attachment - Add attachment',
    'fspec remove-attachment - Remove attachment',
    'fspec show-work-unit - View work unit details (includes attachments)',
  ],
  notes: [
    'Attachments are stored in spec/attachments/<workUnitId>/',
    'File sizes are displayed in kilobytes (KB)',
    'Missing files are highlighted with ✗ marker',
    "Use 'fspec remove-attachment' to remove broken links",
  ],
};

export default config;
