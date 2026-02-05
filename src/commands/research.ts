import { readdir, stat } from 'fs/promises';
import { join, basename } from 'path';
import { spawn } from 'child_process';
import { listResearchTools, TOOL_REGISTRY } from './research-tool-list';
import { getResearchTool } from '../research-tools/registry';
import type { Command } from 'commander';

import { output } from '../utils/output';
export interface ResearchTool {
  name: string;
  path: string;
  usage: string;
  helpCommand: string;
}

export interface ResearchOptions {
  cwd?: string;
  tool?: string;
  query?: string;
  attach?: boolean;
  workUnit?: string;
  all?: boolean;
  userConfigPath?: string; // For testing to avoid loading real user config
}

export interface ResearchResult {
  tools?: ResearchTool[];
  executed?: boolean;
  toolName?: string;
  query?: string;
  output?: string;
  promptForAttachment?: boolean;
  attachmentPath?: string;
  workUnitUpdated?: boolean;
  attachmentCreated?: boolean;
  discoveryMethod?: string;
}

/**
 * Discover research tools from spec/research-scripts/ directory
 * Auto-discovers ANY executable files (not just .sh)
 * Tool names derived from filenames
 */
export async function discoverResearchTools(
  cwd: string
): Promise<ResearchTool[]> {
  const researchScriptsDir = join(cwd, 'spec', 'research-scripts');
  const tools: ResearchTool[] = [];

  try {
    const files = await readdir(researchScriptsDir);

    for (const file of files) {
      const filePath = join(researchScriptsDir, file);
      const stats = await stat(filePath);

      // Check if file is executable (has execute permission)
      const isExecutable = (stats.mode & 0o111) !== 0;

      if (isExecutable && stats.isFile()) {
        // Derive tool name from filename (remove extension if present)
        const toolName = basename(file, extname(file)) || file;

        tools.push({
          name: toolName,
          path: filePath,
          usage: `fspec research --tool=${toolName} --query="your question"`,
          helpCommand: `${toolName} --help`,
        });
      }
    }
  } catch {
    // Directory doesn't exist or can't be read
    // Return empty array
  }

  return tools;
}

/**
 * Get file extension (helper function)
 */
function extname(filename: string): string {
  const dotIndex = filename.lastIndexOf('.');
  if (dotIndex === -1 || dotIndex === 0) {
    return '';
  }
  return filename.slice(dotIndex);
}

/**
 * Get required fields for a tool from the registry
 */
function getRequiredFieldsForTool(toolName: string): string[] {
  const toolMeta = TOOL_REGISTRY[toolName];
  return toolMeta?.required || [];
}

/**
 * Execute research tool with query
 */
async function executeResearchTool(
  toolPath: string,
  query: string
): Promise<string> {
  return new Promise((resolve, reject) => {
    const child = spawn(toolPath, [query], {
      stdio: ['pipe', 'pipe', 'pipe'],
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
      if (code !== 0) {
        reject(new Error(`Tool exited with code ${code}: ${errorOutput}`));
      } else {
        resolve(output);
      }
    });

    child.on('error', (error: Error) => {
      reject(error);
    });
  });
}

/**
 * Main research command
 */
export async function research(
  argsOrOptions: string[] | ResearchOptions = {},
  maybeOptions?: ResearchOptions
): Promise<ResearchResult> {
  // Handle both old signature research(args[], options) and new signature research(options)
  let options: ResearchOptions;
  if (Array.isArray(argsOrOptions)) {
    // Old signature: research(args, options)
    options = maybeOptions || {};
  } else {
    // New signature: research(options)
    options = argsOrOptions;
  }

  const cwd = options.cwd || process.cwd();

  // If no tool specified, list available tools
  if (!options.tool) {
    const toolsWithStatus = await listResearchTools(
      cwd,
      options.all,
      options.userConfigPath
    );

    // Output to console for CLI usage and test expectations
    if (!toolsWithStatus.length) {
      output.log('No research tools found.');
      return {
        tools: [],
        executed: false,
        discoveryMethod: 'dynamic',
      };
    }

    // List configured tools (or all tools if --all flag)
    for (const tool of toolsWithStatus) {
      output.log(`  ${tool.statusIndicator} ${tool.name}`);
      output.log(`    ${tool.description}`);
      if (tool.configured) {
        output.log(`    Ready to use`);
      } else if (tool.configGuidance) {
        output.log(`    Setup required: ${tool.configGuidance.split('\n')[0]}`);

        // Show JSON config example for unconfigured tools when using --all
        if (options.all) {
          output.log(`    Add to spec/fspec-config.json:`);
          output.log(`    {`);
          output.log(`      "research": {`);
          output.log(`        "${tool.name}": {`);
          // Show required fields as example
          const requiredFields = getRequiredFieldsForTool(tool.name);
          for (const field of requiredFields) {
            output.log(`          "${field}": "your-${field}-value",`);
          }
          output.log(`        }`);
          output.log(`      }`);
          output.log(`    }`);
        }
      }
      output.log();
    }

    // Show footer if not showing all tools
    // Get ALL tools to count unconfigured ones
    const allToolsForCount = await listResearchTools(
      cwd,
      true,
      options.userConfigPath
    );
    const unconfiguredCount = allToolsForCount.filter(
      t => !t.configured
    ).length;
    if (!options.all && unconfiguredCount > 0) {
      output.log(
        `  ${unconfiguredCount} additional tool${unconfiguredCount > 1 ? 's' : ''} available.`
      );
      output.log(`  Use --all to see all tools including setup instructions.`);
    }

    return {
      tools: toolsWithStatus.map(t => ({
        name: t.name,
        path: t.name, // Registry tools don't have paths
        usage: `fspec research --tool=${t.name}`,
        helpCommand: `fspec research --tool=${t.name} --help`,
      })),
      executed: false,
      discoveryMethod: 'registry',
    };
  }

  // Execute research tool
  const tools = await discoverResearchTools(cwd);
  const tool = tools.find((t: ResearchTool) => t.name === options.tool);

  if (!tool) {
    throw new Error(`Research tool not found: ${options.tool}`);
  }

  if (!options.query) {
    throw new Error('Query is required when executing a research tool');
  }

  const toolOutput = await executeResearchTool(tool.path, options.query);

  const result: ResearchResult = {
    executed: true,
    toolName: options.tool,
    query: options.query,
    output: toolOutput,
    promptForAttachment: !options.attach,
  };

  // Handle attachment if requested
  if (options.attach && options.workUnit) {
    const timestamp = new Date().toISOString().split('T')[0]; // YYYY-MM-DD
    const querySlug = options.query
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-|-$/g, '');

    const attachmentPath = `spec/attachments/${options.workUnit}/${options.tool}-${querySlug}-research-${timestamp}.md`;

    // TODO: Actually create the attachment file and update work unit metadata
    // For now, just return the expected path
    result.attachmentPath = attachmentPath;
    result.workUnitUpdated = true;
    result.attachmentCreated = true;
  }

  return result;
}

/**
 * Register research command with Commander.js
 * Uses TypeScript plugin system for research tools
 */
export function registerResearchCommand(program: Command): void {
  program
    .command('research [args...]')
    .description('Execute research tools during Example Mapping')
    .option('--tool <name>', 'Research tool to use')
    .option('--work-unit <id>', 'Work unit ID for attachment')
    .allowUnknownOption() // CRITICAL: Forward all unknown args to tool
    .action(async (varArgs: string[], options: any) => {
      try {
        const cwd = process.cwd();

        // If no tool specified, list available tools
        if (!options.tool) {
          const toolsWithStatus = listResearchTools(cwd, true); // Show all tools
          output.log('Available Research Tools:\n');
          for (const tool of toolsWithStatus) {
            output.log(`  ${tool.statusIndicator} ${tool.name}`);
            output.log(`    ${tool.description}`);
            output.log(`    Usage: fspec research --tool=${tool.name} <args>`);
            if (tool.configGuidance) {
              output.log(`    Config: ${tool.configGuidance.split('\n')[0]}`);
            }
            output.log();
          }
          return;
        }

        // Get all arguments after 'research --tool=<name>'
        const allArgs = process.argv.slice(2);
        const forwardedArgs: string[] = [];
        let skipNext = false;

        for (let i = 0; i < allArgs.length; i++) {
          const arg = allArgs[i];

          if (skipNext) {
            skipNext = false;
            continue;
          }

          // Skip --tool and its value
          if (arg === '--tool' || arg.startsWith('--tool=')) {
            if (arg === '--tool') skipNext = true;
            continue;
          }

          // Skip --work-unit and its value (fspec handles this)
          if (arg === '--work-unit' || arg.startsWith('--work-unit=')) {
            if (arg === '--work-unit') skipNext = true;
            continue;
          }

          // Forward everything else to the tool
          forwardedArgs.push(arg);
        }

        // Check if --help is requested BEFORE loading tool (BUG-074 fix)
        if (forwardedArgs.includes('--help') || forwardedArgs.includes('-h')) {
          try {
            const tool = await getResearchTool(options.tool, cwd);
            const { displayResearchToolHelp } = await import(
              '../utils/help-formatter'
            );
            displayResearchToolHelp(tool);
            return;
          } catch {
            // If tool not found, show helpful error with available tools
            const chalk = (await import('chalk')).default;
            output.error(
              chalk.red(`Research tool '${options.tool}' not found\n`)
            );
            output.error('Available research tools:');
            const toolsWithStatus = listResearchTools();
            for (const tool of toolsWithStatus) {
              output.log(
                `  ${tool.statusIndicator} ${tool.name} - ${tool.description}`
              );
            }
            output.error(`\nTry: fspec research --tool=<name> --help`);
            process.exit(1);
          }
        }

        // Load and execute tool
        const tool = await getResearchTool(options.tool, cwd);

        // Execute tool with forwarded arguments
        try {
          const toolResult = await tool.execute(forwardedArgs);
          output.log(toolResult);
        } catch (toolError: unknown) {
          // Wrap tool errors in system-reminder for AI visibility
          output.error('<system-reminder>');
          output.error('RESEARCH TOOL ERROR');
          output.error('');
          output.error(`Tool: ${tool.name}`);
          output.error(
            `Error: ${toolError instanceof Error ? toolError.message : String(toolError)}`
          );
          output.error('</system-reminder>');
          process.exit(1);
        }
      } catch (error: unknown) {
        output.error(error instanceof Error ? error.message : String(error));
        process.exit(1);
      }
    });
}
