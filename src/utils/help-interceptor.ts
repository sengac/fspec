import { displayCustomHelpWithNote } from '../help';
import { helpConfigs } from '../commands/help-registry';
import { displayHelpAndExit } from './help-formatter';
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { spawn } from 'child_process';
import chalk from 'chalk';
import { discoverResearchTools } from '../commands/research';

/**
 * Process-level help interceptor
 *
 * Intercepts --help flags before Commander.js parses arguments.
 * Uses pre-loaded help configs from help-registry (eagerly imported via import.meta.glob).
 * Falls back to Commander.js default help if no custom help exists.
 */

// Read version from package.json
function getVersion(): string | undefined {
  try {
    const __filename = fileURLToPath(import.meta.url);
    const __dirname = dirname(__filename);
    const packageJsonPath = join(__dirname, '..', 'package.json');
    const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));
    return packageJson.version;
  } catch (error) {
    // Silently fail if package.json cannot be read
    return undefined;
  }
}

/**
 * Handle custom help if --help flag is present OR no arguments provided
 * @returns true if custom help was displayed, false to let Commander.js handle it
 */
export async function handleCustomHelp(): Promise<boolean> {
  const args = process.argv;
  const hasHelp = args.includes('--help') || args.includes('-h');
  const hasNoArgs = args.length <= 2; // Just 'node' and 'script.js'

  // If no help flag and has arguments, let Commander.js handle it
  if (!hasHelp && !hasNoArgs) {
    return false;
  }

  // Find command name (first non-flag arg after 'node script.js')
  const commandName = args.slice(2).find(arg => !arg.startsWith('-'));

  if (!commandName) {
    // "fspec --help" or bare "fspec" without command -> show custom main help
    const version = getVersion();
    displayCustomHelpWithNote(version);
    return true; // Help was displayed, prevent Commander from showing help again
  }

  // RES-011: Special handling for research command with --tool flag
  if (commandName === 'research' && hasHelp) {
    const toolArgIndex = args.findIndex(arg => arg.startsWith('--tool'));
    let toolName: string | undefined;

    if (toolArgIndex !== -1) {
      const toolArg = args[toolArgIndex];
      if (toolArg.includes('=')) {
        toolName = toolArg.split('=')[1];
      } else if (
        toolArgIndex + 1 < args.length &&
        !args[toolArgIndex + 1].startsWith('-')
      ) {
        toolName = args[toolArgIndex + 1];
      }
    }

    // If tool is specified, forward help to research tool script
    if (toolName) {
      const cwd = process.cwd();
      const tools = await discoverResearchTools(cwd);
      const tool = tools.find(t => t.name === toolName);

      if (!tool) {
        console.error(
          chalk.red(`Error: Research tool '${toolName}' not found\n`)
        );
        console.log('Available tools:');
        for (const t of tools) {
          console.log(chalk.cyan(`  - ${t.name}`));
        }
        console.log(
          chalk.dim(`\nRun 'fspec research' for full list with status`)
        );
        process.exit(1);
      }

      // Forward --help to tool script
      return new Promise(resolve => {
        const child = spawn(tool.path, ['--help'], {
          stdio: ['inherit', 'pipe', 'pipe'],
        });

        let output = '';
        let errorOutput = '';

        child.stdout.on('data', (data: Buffer) => {
          output += data.toString();
        });

        child.stderr.on('data', (data: Buffer) => {
          errorOutput += data.toString();
        });

        child.on('close', (code: number) => {
          if (code === 0 && output) {
            console.log(output);
            process.exit(0);
          } else if (code !== 0 && errorOutput) {
            // Tool doesn't implement --help, show warning
            console.log(
              chalk.yellow(
                `Warning: Tool '${toolName}' does not implement --help flag\n`
              )
            );
            console.log('Showing generic usage:');
            console.log(
              chalk.dim(`  fspec research --tool=${toolName} [args...]\n`)
            );
            console.log(`For more information, check: ${tool.path}`);
            process.exit(0);
          } else {
            // Tool output to stderr or no output
            console.log(output || errorOutput);
            process.exit(code || 0);
          }
        });

        child.on('error', (error: Error) => {
          console.error(chalk.red(`Error executing tool: ${error.message}`));
          process.exit(1);
        });
      });
    }
    // No tool specified - fall through to show comprehensive research help
  }

  // Check if command has custom help registered (pre-loaded via import.meta.glob)
  const helpConfig = helpConfigs.get(commandName);

  if (!helpConfig) {
    // Command not in registry -> use Commander default help
    return false;
  }

  // Display custom help using pre-loaded config
  displayHelpAndExit(helpConfig);
  return true; // Will never reach here - displayHelpAndExit calls process.exit(0)
}
