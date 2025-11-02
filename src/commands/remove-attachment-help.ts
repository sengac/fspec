import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'remove-attachment',
  description:
    'Remove an attachment from a work unit. By default, this removes both the tracking entry and deletes the file from disk. Use --keep-file to preserve the file.',
  usage: 'fspec remove-attachment <workUnitId> <fileName> [options]',
  whenToUse:
    'Use to remove outdated or incorrect attachments, to clean up broken attachment links, to remove duplicate attachments, or when attachment is no longer relevant to the work unit.',
  arguments: [
    {
      name: 'workUnitId',
      description:
        'The ID of the work unit to remove the attachment from (e.g., AUTH-001)',
      required: true,
    },
    {
      name: 'fileName',
      description:
        'The name of the file to remove (e.g., diagram.png). Must match the basename of the attachment path.',
      required: true,
    },
  ],
  options: [
    {
      flag: '--keep-file',
      description:
        'Keep the file on disk (only remove from work unit tracking)',
    },
  ],
  examples: [
    {
      command: 'fspec remove-attachment AUTH-001 auth-flow.png',
      description: 'Remove attachment and delete file',
      output:
        '✓ Attachment removed from work unit and file deleted\n  File: spec/attachments/AUTH-001/auth-flow.png',
    },
    {
      command: 'fspec remove-attachment AUTH-001 important-doc.pdf --keep-file',
      description: 'Remove attachment but keep file on disk',
      output:
        '✓ Attachment removed from work unit (file kept)\n  File: spec/attachments/AUTH-001/important-doc.pdf',
    },
    {
      command: 'fspec remove-attachment AUTH-001 missing-file.png',
      description: 'Remove attachment when file is already missing',
      output:
        '⚠ Attachment removed from work unit (file was already missing)\n  File: spec/attachments/AUTH-001/missing-file.png',
    },
  ],
  prerequisites: [
    'Work unit must exist',
    'Attachment must be tracked in the work unit',
  ],
  typicalWorkflow: [
    'List attachments: fspec list-attachments AUTH-001',
    'Identify attachment to remove',
    'Remove attachment: fspec remove-attachment AUTH-001 old-diagram.png',
    'Verify removal: fspec list-attachments AUTH-001',
  ],
  commonErrors: [
    {
      error: "Work unit 'AUTH-001' does not exist",
      solution: 'Create the work unit first or check the work unit ID',
    },
    {
      error: "Work unit 'AUTH-001' has no attachments to remove",
      solution:
        "The work unit has no attachments. Use 'fspec list-attachments AUTH-001' to verify.",
    },
    {
      error: "Attachment 'diagram.png' not found for work unit 'AUTH-001'",
      solution:
        "The file name doesn't match any tracked attachments. Use 'fspec list-attachments AUTH-001' to see available attachments. Ensure the file name matches exactly (case-sensitive).",
    },
  ],
  relatedCommands: [
    'fspec add-attachment - Add attachment',
    'fspec list-attachments - List all attachments',
    'fspec show-work-unit - View work unit details',
  ],
  notes: [
    'By default, both tracking entry and file are removed',
    'Use --keep-file to preserve the file on disk',
    'Removing attachment updates work unit timestamps',
    'File name is matched against basename of attachment paths',
    "If file doesn't exist, only tracking entry is removed (warning shown)",
  ],
};

export default config;
