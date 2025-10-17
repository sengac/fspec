import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'discover-foundation',
  description:
    'Discover project foundation by analyzing codebase and creating draft file with placeholders',
  usage: 'fspec discover-foundation [options]',
  whenToUse:
    'Use when starting a new project with fspec to bootstrap foundation.json. Creates a draft file with [DETECTED:] values from code analysis and [QUESTION:] placeholders for AI/human to fill in.',
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
    '1. Run "fspec discover-foundation" to create draft with placeholders',
    '2. Analyze codebase to detect project type, personas, capabilities',
    '3. Generate spec/foundation.json.draft with [DETECTED:] and [QUESTION:] markers',
    '4. AI/human edits draft file to replace [QUESTION:] placeholders',
    '5. Run "fspec discover-foundation --finalize" to validate and create final foundation.json',
    '6. Draft file is deleted after successful finalization',
  ],
  examples: [
    {
      command: 'fspec discover-foundation',
      description: 'Create draft foundation with placeholders',
      output: `<system-reminder>
Detected 3 user personas from routes: End User, Admin, API Consumer.

Review in questionnaire. Focus on WHY/WHAT, not HOW.
See CLAUDE.md for boundary guidance.

Code analysis also detected:
- Project Type: web-app
- Key Capabilities: User Authentication, Data Management, API Access
</system-reminder>
✓ Generated spec/foundation.json.draft

Next steps:
1. Edit the draft file to replace [QUESTION: ...] placeholders
2. Run: fspec discover-foundation --finalize`,
    },
    {
      command: 'fspec discover-foundation --finalize',
      description: 'Validate draft and create final foundation.json',
      output: `✓ Generated spec/foundation.json
✓ Foundation discovered and validated successfully`,
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
    'Two-phase workflow: (1) Create draft with placeholders, (2) Finalize after editing',
    'Uses code analysis guidance from FOUND-002 (automated discovery)',
    'Draft file contains [DETECTED: value] for code-inferred values',
    'Draft file contains [QUESTION: text] placeholders for AI/human to fill',
    'Focuses on WHY (problems) and WHAT (capabilities), not HOW (implementation)',
    'Generated foundation.json uses v2.0.0 generic schema',
    'Finalization validates draft against schema before creating final file',
    'Draft file is deleted after successful finalization',
  ],
};

export default config;
