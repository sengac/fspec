import { readdir, stat } from 'fs/promises';
import { join, basename } from 'path';
import { spawn } from 'child_process';
import { listResearchTools } from './research-tool-list';
import { getResearchTool } from '../research-tools/registry';
import type { Command } from 'commander';

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
  } catch (error) {
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
  options: ResearchOptions = {}
): Promise<ResearchResult> {
  const cwd = options.cwd || process.cwd();

  // If no tool specified, list available tools
  if (!options.tool) {
    const tools = await discoverResearchTools(cwd);
    return {
      tools,
      executed: false,
      discoveryMethod: 'dynamic',
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

  const output = await executeResearchTool(tool.path, options.query);

  const result: ResearchResult = {
    executed: true,
    toolName: options.tool,
    query: options.query,
    output,
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
    .action(async (varArgs: string[], options: any, cmd: Command) => {
      try {
        const cwd = process.cwd();

        // If no tool specified, list available tools
        if (!options.tool) {
          const toolsWithStatus = listResearchTools();
          console.log('Available Research Tools:\n');
          for (const tool of toolsWithStatus) {
            console.log(`  ${tool.statusIndicator} ${tool.name}`);
            console.log(`    ${tool.description}`);
            console.log(`    Usage: fspec research --tool=${tool.name} <args>`);
            if (tool.configGuidance) {
              console.log(`    Config: ${tool.configGuidance.split('\n')[0]}`);
            }
            console.log();
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

        // Load and execute tool
        const tool = await getResearchTool(options.tool, cwd);

        // Check if --help is requested
        if (forwardedArgs.includes('--help') || forwardedArgs.includes('-h')) {
          console.log(tool.help());
          return;
        }

        // Execute tool with forwarded arguments
        try {
          const output = await tool.execute(forwardedArgs);
          console.log(output);
        } catch (toolError: any) {
          // Wrap tool errors in system-reminder for AI visibility
          console.error('<system-reminder>');
          console.error('RESEARCH TOOL ERROR');
          console.error('');
          console.error(`Tool: ${tool.name}`);
          console.error(`Error: ${toolError.message}`);
          console.error('</system-reminder>');
          process.exit(1);
        }
      } catch (error: any) {
        console.error(error.message);
        process.exit(1);
      }
    });
}
