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

// Map of common field names to JSON paths
const FIELD_MAP: Record<string, string> = {
  projectOverview: 'whatWeAreBuilding.projectOverview',
  problemDefinition: 'whyWeAreBuildingIt.problemDefinition.primary.description',
  projectName: 'project.name',
  projectDescription: 'project.description',
};

export async function showFoundation(
  options: ShowFoundationOptions
): Promise<ShowFoundationResult> {
  const { field, format = 'text', output, cwd = process.cwd() } = options;

  try {
    // Load or create foundation.json using ensureFoundationFile
    const foundationData: Foundation = await ensureFoundationFile(cwd);

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

// Helper function to format Foundation as readable text
function formatFoundationAsText(foundation: Foundation): string {
  const lines: string[] = [];

  lines.push('=== PROJECT ===');
  lines.push(`Name: ${foundation.project.name}`);
  lines.push(`Description: ${foundation.project.description}`);
  lines.push(`Repository: ${foundation.project.repository}`);
  lines.push(`License: ${foundation.project.license}`);
  lines.push('');

  lines.push('=== WHAT WE ARE BUILDING ===');
  lines.push(foundation.whatWeAreBuilding.projectOverview);
  lines.push('');

  lines.push('=== WHY WE ARE BUILDING IT ===');
  lines.push(
    `Problem: ${foundation.whyWeAreBuildingIt.problemDefinition.primary.title}`
  );
  lines.push(
    foundation.whyWeAreBuildingIt.problemDefinition.primary.description
  );
  lines.push('');

  if (foundation.architectureDiagrams.length > 0) {
    lines.push('=== ARCHITECTURE DIAGRAMS ===');
    foundation.architectureDiagrams.forEach(diagram => {
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
