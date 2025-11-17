import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'add-hotspot',
  description:
    'Add hotspot to Event Storm section for capturing uncertainties, risks, or problems',
  usage: 'fspec add-hotspot <workUnitId> <text> [options]',
  whenToUse:
    'Use during Big Picture Event Storming when encountering areas of uncertainty, risk, or problems that need further investigation.',
  prerequisites: [
    'Work unit must exist',
    'Work unit must have eventStorm section initialized',
    'Hotspot should identify a PROBLEM or UNCERTAINTY',
  ],
  arguments: [
    {
      name: 'workUnitId',
      description: 'Work unit ID',
      required: true,
    },
    {
      name: 'text',
      description: 'Hotspot description (brief title)',
      required: true,
    },
  ],
  options: [
    {
      flag: '--concern <description>',
      description:
        'Risk, uncertainty, or problem description (detailed explanation)',
      required: false,
    },
    {
      flag: '--timestamp <ms>',
      description: 'Timeline position in milliseconds',
      required: false,
    },
    {
      flag: '--bounded-context <name>',
      description: 'Bounded context association',
      required: false,
    },
  ],
  examples: [
    {
      command:
        'fspec add-hotspot AUTH-001 "Password Reset Flow" --concern "Unclear timeout logic"',
      description: 'Add hotspot with concern description',
      output: '✓ Added hotspot "Password Reset Flow" to AUTH-001 (ID: 0)',
    },
    {
      command:
        'fspec add-hotspot PAYMENT-001 "Payment Gateway" --concern "Third-party API availability unclear"',
      description: 'Capture external dependency uncertainty',
      output: '✓ Added hotspot "Payment Gateway" to PAYMENT-001 (ID: 0)',
    },
  ],
  relatedCommands: [
    'add-domain-event',
    'add-command',
    'add-policy',
    'show-event-storm',
    'generate-example-mapping-from-event-storm',
  ],
  commonErrors: [
    {
      error: 'Work unit not found',
      fix: 'Ensure work unit exists: fspec show-work-unit <id>',
    },
  ],
  commonPatterns: [
    'Hotspots represent QUESTIONS or RISKS to investigate',
    'Use --concern to document detailed problem description',
    'Convert hotspots to questions via generate-example-mapping-from-event-storm',
    'Hotspots often become @human questions in Example Mapping',
  ],
  notes: [
    'Hotspots highlight areas needing more discovery',
    'Concerns can be converted to questions for stakeholders',
    'Use show-event-storm to view all hotspots',
    'Hotspots assigned stable IDs starting from 0',
  ],
};

export default config;
