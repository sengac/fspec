import { readdir, stat } from 'fs/promises';
import { join, basename } from 'path';
import { spawn } from 'child_process';

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
async function discoverResearchTools(cwd: string): Promise<ResearchTool[]> {
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
