import { writeFile } from 'fs/promises';
import type { Command } from 'commander';
import chalk from 'chalk';
import type { Foundation } from '../types/foundation';
import { ensureFoundationFile } from '../utils/ensure-files';

interface ShowFoundationOptions {
  field?: string;
  format?: 'text' | 'json';
  output?: string;
  cwd?: string;
}

interface ShowFoundationResult {
  success: boolean;
  output?: string;
  format?: string;
  error?: string;
}

// Map of common field names to JSON paths (generic schema v2.0.0)
const FIELD_MAP: Record<string, string> = {
  projectName: 'project.name',
  projectVision: 'project.vision',
  projectType: 'project.projectType',
  problemTitle: 'problemSpace.primaryProblem.title',
  problemDescription: 'problemSpace.primaryProblem.description',
  problemImpact: 'problemSpace.primaryProblem.impact',
  solutionOverview: 'solutionSpace.overview',

  // Legacy mappings for backward compatibility
  projectOverview: 'solutionSpace.overview',
  problemDefinition: 'problemSpace.primaryProblem.description',
};

export async function showFoundation(
  options: ShowFoundationOptions
): Promise<ShowFoundationResult> {
  const { field, format = 'text', output, cwd = process.cwd() } = options;

  try {
    // Load or create foundation.json using ensureFoundationFile (generic schema v2.0.0)
    const foundationData: any = await ensureFoundationFile(cwd);

    // Get specific field or entire foundation
    let displayData: any;

    if (field) {
      // Try to get field by direct property name or mapped path
      const fieldPath = FIELD_MAP[field] || field;
      displayData = getNestedProperty(foundationData, fieldPath);

      if (displayData === undefined) {
        return {
          success: false,
          error: `Field '${field}' not found`,
        };
      }
    } else {
      displayData = foundationData;
    }

    // Format output
    let formattedOutput: string;

    if (format === 'json') {
      formattedOutput = JSON.stringify(displayData, null, 2);
    } else {
      // text format - convert to readable text
      if (field) {
        // For specific field, display as plain text
        if (typeof displayData === 'string') {
          formattedOutput = displayData;
        } else {
          formattedOutput = JSON.stringify(displayData, null, 2);
        }
      } else {
        // For entire foundation, display as readable summary
        formattedOutput = formatFoundationAsText(foundationData);
      }
    }

    // Write to file if output specified
    if (output) {
      await writeFile(output, formattedOutput, 'utf-8');
    }

    return {
      success: true,
      output: formattedOutput,
      format,
    };
  } catch (error: any) {
    return {
      success: false,
      error: error.message,
    };
  }
}

// Helper function to get nested property by path
function getNestedProperty(obj: any, path: string): any {
  const parts = path.split('.');
  let current = obj;

  for (const part of parts) {
    if (current === undefined || current === null) {
      return undefined;
    }
    current = current[part];
  }

  return current;
}

// Helper function to format Foundation as readable text (generic schema v2.0.0)
function formatFoundationAsText(foundation: any): string {
  const lines: string[] = [];

  lines.push('=== PROJECT ===');
  if (foundation.project) {
    lines.push(`Name: ${foundation.project.name || 'N/A'}`);
    lines.push(`Vision: ${foundation.project.vision || 'N/A'}`);
    lines.push(`Type: ${foundation.project.projectType || 'N/A'}`);
    if (foundation.project.repository) {
      lines.push(`Repository: ${foundation.project.repository}`);
    }
    if (foundation.project.license) {
      lines.push(`License: ${foundation.project.license}`);
    }
  }
  lines.push('');

  if (foundation.problemSpace && foundation.problemSpace.primaryProblem) {
    lines.push('=== PROBLEM SPACE ===');
    const problem = foundation.problemSpace.primaryProblem;
    lines.push(`Title: ${problem.title || 'N/A'}`);
    lines.push(`Description: ${problem.description || 'N/A'}`);
    lines.push(`Impact: ${problem.impact || 'N/A'}`);
    lines.push('');
  }

  if (foundation.solutionSpace) {
    lines.push('=== SOLUTION SPACE ===');
    lines.push(foundation.solutionSpace.overview || 'N/A');
    lines.push('');

    if (
      foundation.solutionSpace.capabilities &&
      foundation.solutionSpace.capabilities.length > 0
    ) {
      lines.push('Capabilities:');
      foundation.solutionSpace.capabilities.forEach((cap: any) => {
        lines.push(`- ${cap.name}: ${cap.description}`);
      });
      lines.push('');
    }
  }

  if (foundation.personas && foundation.personas.length > 0) {
    lines.push('=== PERSONAS ===');
    foundation.personas.forEach((persona: any) => {
      lines.push(`- ${persona.name}: ${persona.description}`);
    });
    lines.push('');
  }

  if (foundation.architectureDiagrams && foundation.architectureDiagrams.length > 0) {
    lines.push('=== ARCHITECTURE DIAGRAMS ===');
    foundation.architectureDiagrams.forEach((diagram: any) => {
      lines.push(`- ${diagram.title}`);
    });
    lines.push('');
  }

  return lines.join('\n');
}

export async function showFoundationCommand(options: {
  field?: string;
  format?: string;
  output?: string;
}): Promise<void> {
  try {
    const result = await showFoundation({
      field: options.field,
      format: (options.format as 'text' | 'json') || 'text',
      output: options.output,
    });

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    if (!options.output) {
      console.log(result.output);
    } else {
      console.log(chalk.green('✓'), `Output written to ${options.output}`);
    }

    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}

export function registerShowFoundationCommand(program: Command): void {
  program
    .command('show-foundation')
    .description('Display FOUNDATION.md content')
    .option('--section <section>', 'Show specific section only')
    .option('--format <format>', 'Output format: text, markdown, or json', 'text')
    .option('--output <file>', 'Write output to file')
    .option('--list-sections', 'List section names only', false)
    .option('--line-numbers', 'Show line numbers', false)
    .action(showFoundationCommand);
}
