import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'remove-architecture-note',
  description: 'Remove architecture note from work unit by index',
  usage: 'fspec remove-architecture-note <workUnitId> <index>',
  whenToUse:
    'Use when you need to remove an incorrect or outdated architecture note from a work unit. View current notes with show-work-unit to see indices.',
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID (e.g., WORK-001)',
      required: true,
    },
    {
      name: 'index',
      description: 'Index of note to remove (0-based, see show-work-unit output)',
      required: true,
    },
  ],
  examples: [
    {
      command: 'fspec show-work-unit WORK-001',
      description: 'First, view architecture notes with their indices',
      output: `Architecture Notes:
  0. Uses @cucumber/gherkin parser
  1. Must complete validation within 2 seconds
  2. Share validation logic with formatter`,
    },
    {
      command: 'fspec remove-architecture-note WORK-001 1',
      description: 'Remove note at index 1 (the performance requirement)',
      output: '✓ Architecture note removed successfully',
    },
  ],
  commonErrors: [
    {
      error: 'Work unit has no architecture notes',
      fix: 'Verify work unit has notes using show-work-unit command',
    },
    {
      error: 'Invalid index N. Work unit has M architecture note(s)',
      fix: 'Use show-work-unit to see valid indices (0-based)',
    },
  ],
  relatedCommands: [
    'show-work-unit',
    'add-architecture-note',
    'generate-scenarios',
  ],
  notes: [
    'Indices are 0-based (first note is index 0)',
    'Use show-work-unit to view current notes and their indices',
    'Removing a note will shift subsequent indices down by 1',
  ],
};

export default config;
