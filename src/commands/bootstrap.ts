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

interface BootstrapOptions {
  cwd?: string;
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
