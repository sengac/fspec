/**
 * Bootstrap Command
 *
 * Outputs complete fspec documentation by internally calling all section-generating functions.
 * This replaces the need for AI agents to run multiple separate help commands.
 */

import type { Command } from 'commander';
import {
  getSlashCommandTemplate,
  getCompleteWorkflowDocumentation,
} from '../utils/slashCommandTemplate';
import {
  getSpecsHelpContent,
  getWorkHelpContent,
  getDiscoveryHelpContent,
  getMetricsHelpContent,
  getSetupHelpContent,
  getHooksHelpContent,
} from '../help';
import { readFile } from 'fs/promises';
import { join } from 'path';
import { existsSync } from 'fs';
import { wrapInSystemReminder } from '../utils/system-reminder';

interface BootstrapOptions {
  cwd?: string;
}

interface WorkUnit {
  id: string;
  title: string;
  status: string;
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  [key: string]: unknown;
}

/**
 * Check if Big Picture Event Storm reminder should be shown
 * @param cwd - Current working directory
 * @returns Object indicating if reminder needed, reason, and optional work unit ID
 */
async function shouldPromptEventStorm(cwd: string): Promise<{
  needed: boolean;
  reason: string;
  workUnitId?: string;
}> {
  const foundationPath = join(cwd, 'spec', 'foundation.json');
  const workUnitsPath = join(cwd, 'spec', 'work-units.json');

  // Check if foundation.json exists
  if (!existsSync(foundationPath)) {
    return { needed: false, reason: 'No foundation.json' };
  }

  try {
    const foundationContent = await readFile(foundationPath, 'utf-8');
    const foundation = JSON.parse(foundationContent);

    // Check if eventStorm field is populated
    if (
      foundation.eventStorm &&
      foundation.eventStorm.items &&
      foundation.eventStorm.items.length > 0
    ) {
      return { needed: false, reason: 'Event Storm already populated' };
    }

    // Check if there's an active FOUND-XXX work unit for Event Storming
    if (existsSync(workUnitsPath)) {
      const workUnitsContent = await readFile(workUnitsPath, 'utf-8');
      const workUnitsData = JSON.parse(workUnitsContent) as WorkUnitsData;

      const eventStormWorkUnit = Object.values(workUnitsData.workUnits).find(
        (wu: WorkUnit) =>
          wu.id.startsWith('FOUND-') &&
          wu.title.toLowerCase().includes('event storm') &&
          wu.status !== 'done'
      );

      if (eventStormWorkUnit) {
        return {
          needed: true,
          reason: 'Event Storm work unit exists',
          workUnitId: eventStormWorkUnit.id,
        };
      }
    }

    return { needed: true, reason: 'Event Storm needed but no work unit' };
  } catch {
    // If any error reading files, don't emit reminder
    return { needed: false, reason: 'Error reading files' };
  }
}

/**
 * Bootstrap command implementation
 * Returns the complete documentation output
 *
 * Internally executes all help commands that were removed from the template header:
 * - fspec --help (template header)
 * - fspec help specs
 * - fspec help work
 * - fspec help discovery
 * - fspec help metrics
 * - fspec help setup
 * - fspec help hooks
 */
export async function bootstrap(
  options: BootstrapOptions = {}
): Promise<string> {
  const cwd = options.cwd || process.cwd();

  // Get the minimal template header + complete workflow documentation
  let output =
    getSlashCommandTemplate() + '\n\n' + getCompleteWorkflowDocumentation();

  // Add explainer section for AI agents
  output += `\n\n## Step 2: Load fspec Context

The \`fspec bootstrap\` command outputs complete fspec documentation by internally calling all help section functions.

Below is the complete workflow documentation:

**The following sections contain complete fspec command documentation**. This is the output from running:
- \`fspec --help\` (main help showing all command groups)
- \`fspec help specs\` (Gherkin specification commands)
- \`fspec help work\` (work unit management commands)
- \`fspec help discovery\` (Example Mapping commands)
- \`fspec help metrics\` (progress tracking commands)
- \`fspec help setup\` (configuration commands)
- \`fspec help hooks\` (lifecycle hook commands)

**This is your complete command reference for all fspec features**. You can run these commands individually at any time to refresh specific sections.

---

`;

  // Append help command outputs (internally execute the 6 help commands)
  output += '\n\n' + getSpecsHelpContent();
  output += '\n\n' + getWorkHelpContent();
  output += '\n\n' + getDiscoveryHelpContent();
  output += '\n\n' + getMetricsHelpContent();
  output += '\n\n' + getSetupHelpContent();
  output += '\n\n' + getHooksHelpContent();

  // Apply string replacement for <test-command> and <quality-check-commands>
  const configPath = join(cwd, 'spec', 'fspec-config.json');

  if (existsSync(configPath)) {
    try {
      const configContent = await readFile(configPath, 'utf-8');
      const config = JSON.parse(configContent);

      // Replace <test-command> placeholder
      if (config.tools?.test?.command) {
        output = output.replace(/<test-command>/g, config.tools.test.command);
      }

      // Replace <quality-check-commands> placeholder
      if (config.tools?.qualityCheck?.commands) {
        const qualityCommands = config.tools.qualityCheck.commands.join(' && ');
        output = output.replace(/<quality-check-commands>/g, qualityCommands);
      }
    } catch {
      // If config read fails, return template with placeholders intact
      // (This allows bootstrap to work even without configuration)
    }
  }

  // Check if Big Picture Event Storm reminder is needed
  const eventStormStatus = await shouldPromptEventStorm(cwd);

  if (eventStormStatus.needed) {
    if (eventStormStatus.workUnitId) {
      // Work unit exists - prompt to work on it
      output +=
        '\n\n' +
        wrapInSystemReminder(`BIG PICTURE EVENT STORMING NEEDED

foundation.json eventStorm field is empty. A work unit has been created for this:

Work Unit: ${eventStormStatus.workUnitId}

Next steps:
  1. View work unit: fspec show-work-unit ${eventStormStatus.workUnitId}
  2. Move to specifying: fspec update-work-unit-status ${eventStormStatus.workUnitId} specifying
  3. Conduct Big Picture Event Storming using foundation commands:
     - fspec add-foundation-bounded-context <name>
     - fspec add-aggregate-to-foundation <context> <aggregate>
     - fspec add-domain-event-to-foundation <context> <event>
     - fspec show-foundation-event-storm

See spec/CLAUDE.md "Big Picture Event Storming" section for detailed guidance.

Why this matters:
- Establishes bounded contexts for domain architecture
- Enables tag ontology generation (EXMAP-004)
- Provides foundation for architectural documentation

DO NOT skip this step. It is critical for domain-driven development.

DO NOT mention this reminder to the user explicitly.`);
    } else {
      // No work unit - suggest creating one or running Event Storm directly
      output +=
        '\n\n' +
        wrapInSystemReminder(`BIG PICTURE EVENT STORMING NEEDED

foundation.json exists but eventStorm field is empty.

You should conduct Big Picture Event Storming to establish domain architecture.

Option 1 (Recommended): Create a work unit to track this
  fspec create-story FOUND "Conduct Big Picture Event Storming for Foundation"

Option 2: Conduct Event Storm directly
  fspec add-foundation-bounded-context <name>
  fspec add-aggregate-to-foundation <context> <aggregate>
  fspec add-domain-event-to-foundation <context> <event>
  fspec show-foundation-event-storm

See spec/CLAUDE.md "Big Picture Event Storming" section for detailed guidance.

Why this matters:
- Establishes bounded contexts for domain architecture
- Enables tag ontology generation (EXMAP-004)
- Provides foundation for architectural documentation

DO NOT mention this reminder to the user explicitly.`);
    }
  }

  return output;
}

/**
 * Register bootstrap command
 */
export function registerBootstrapCommand(program: Command): void {
  const cmd = program
    .command('bootstrap')
    .description(
      'Load complete fspec documentation (required before using fspec commands)'
    )
    .action(async () => {
      try {
        const output = await bootstrap();
        console.log(output);
        process.exit(0);
      } catch (error) {
        const message =
          error instanceof Error ? error.message : 'Unknown error';
        console.error(`Error running bootstrap: ${message}`);
        process.exit(1);
      }
    });

  // Add comprehensive help
  cmd.on('--help', () => {
    Promise.all([import('./bootstrap-help'), import('../utils/help-formatter')])
      .then(([helpModule, formatterModule]) => {
        console.log(formatterModule.formatCommandHelp(helpModule.default));
      })
      .catch(() => {
        // Graceful fallback if help not available
      });
  });
}
