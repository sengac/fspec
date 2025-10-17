import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'discover-foundation',
  description:
    'Interactive AI-guided workflow to discover project foundation field-by-field with system-reminders',
  usage: 'fspec discover-foundation [options]',
  whenToUse:
    'Use when starting a new project with fspec to bootstrap foundation.json. Creates draft with placeholders, then guides AI field-by-field using system-reminders and chaining to next field after each fspec update-foundation command.',
  options: [
    {
      flag: '--output <path>',
      description:
        'Output path for final foundation.json (default: spec/foundation.json)',
    },
    {
      flag: '--finalize',
      description:
        'Finalize foundation.json from edited draft file',
    },
    {
      flag: '--draft-path <path>',
      description:
        'Path to draft file (default: spec/foundation.json.draft)',
    },
  ],
  workflow: [
    '1. AI runs "fspec discover-foundation" to create draft with placeholders',
    '2. Command creates spec/foundation.json.draft with [QUESTION:] and [DETECTED:] placeholders',
    '3. Command scans draft and emits system-reminder for FIRST unfilled field (Field 1/N)',
    '4. AI analyzes codebase (ULTRATHINK), asks human for confirmation, runs "fspec update-foundation --field <path> --value <value>"',
    '5. Command automatically re-scans draft and emits system-reminder for NEXT unfilled field',
    '6. Repeat steps 4-5 until all [QUESTION:] placeholders resolved',
    '7. When complete, AI runs "fspec discover-foundation --finalize" to validate and create foundation.json',
    '8. Draft file deleted, FOUNDATION.md auto-generated',
  ],
  examples: [
    {
      command: 'fspec discover-foundation',
      description: 'Create draft and receive guidance for first field',
      output: `✓ Generated spec/foundation.json.draft

Draft created. To complete foundation, you must ULTRATHINK the entire codebase.

analyze EVERYTHING: commands, routes, UI, tests, README, package.json.
Understand HOW it works, then determine WHY it exists and WHAT users can do.

I will guide you field-by-field.

<system-reminder>
Field 1/8: project.name

analyze package.json name field and confirm with human.

Run: fspec update-foundation --field project.name --value <name>
</system-reminder>`,
    },
    {
      command: 'fspec update-foundation --field project.name --value "fspec"',
      description: 'Fill first field, command chains to next field',
      output: `✓ Updated project.name to "fspec" in spec/foundation.json.draft

<system-reminder>
Field 2/8: project.vision (elevator pitch)

ULTRATHINK: Read ALL code, understand the system deeply. What is the core PURPOSE?
Focus on WHY this exists, not HOW it works.

Ask human to confirm vision.

Run: fspec update-foundation --field project.vision --value "your vision"
</system-reminder>`,
    },
    {
      command: 'fspec discover-foundation --finalize',
      description: 'Validate complete draft and create final foundation.json',
      output: `✓ Generated spec/foundation.json
✓ Foundation discovered and validated successfully

Discovery complete!

Created: spec/foundation.json, spec/FOUNDATION.md

Foundation is ready.`,
    },
  ],
  commonPatterns: [
    'CLI tools: Detects commander.js usage, bin field in package.json, command definitions',
    'Web apps: Detects Express routes, React components, identifies End User and API Consumer personas',
    'Libraries: Detects exports field in package.json, identifies Developer persona',
  ],
  relatedCommands: [
    'update-foundation',
    'show-foundation',
    'generate-foundation-md',
  ],
  notes: [
    'AI-driven feedback loop: command guides AI field-by-field with system-reminders',
    'After each fspec update-foundation, command automatically chains to next unfilled field',
    'Draft file contains [DETECTED: value] for code-inferred values requiring verification',
    'Draft file contains [QUESTION: text] placeholders for AI to fill via commands',
    'AI must NEVER manually edit draft - system detects manual edits and reverts changes',
    'Focuses on ULTRATHINK (deep analysis), WHY (problems), WHAT (capabilities), not HOW',
    'System-reminders are invisible to user but visible to AI for guidance',
    'Generated foundation.json uses v2.0.0 generic schema',
    'Finalization validates draft against schema and auto-generates FOUNDATION.md',
    'Draft file is automatically deleted after successful finalization',
  ],
};

export default config;
