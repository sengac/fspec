/**
 * list-foundation-sections Command
 *
 * Exposes every valid foundation section name with its JSON path and
 * constraint info. Weaker LLMs (and humans) use this as a discovery
 * mechanism when filling out the foundation draft.
 *
 * Part of FOUND-044 (Fail-Fast Foundation Workflow for Weaker LLMs).
 */

import type { Command } from 'commander';
import chalk from 'chalk';

import { output } from '../utils/output';

export interface ListFoundationSectionsOptions {
  /** Current working directory (defaults to process.cwd()) */
  cwd?: string;
  /** Format: 'text' (default) or 'json' */
  format?: 'text' | 'json';
}

export interface ListFoundationSectionsResult {
  /** Whether the command succeeded */
  success: boolean;
  /** Rendered output for 'text' format or JSON string for 'json' format */
  output?: string;
  /** Error message when success is false */
  error?: string;
}

/**
 * Single row in the sections table. Each row describes one valid
 * update-foundation section name.
 */
interface FoundationSectionSpec {
  /** Section name as passed to `fspec update-foundation <section>` */
  name: string;
  /** Dotted JSON path inside foundation.json */
  jsonPath: string;
  /** Human-readable constraint description */
  constraint: string;
  /** Optional non-exhaustive examples of valid values */
  examples?: string[];
  /** Short explanation of what the field represents */
  description: string;
}

/**
 * The canonical list of foundation sections exposed by update-foundation.
 *
 * This is the single source of truth: when new fields are added to
 * update-foundation, add them here too.
 */
const FOUNDATION_SECTIONS: FoundationSectionSpec[] = [
  {
    name: 'projectName',
    jsonPath: 'project.name',
    constraint: 'freeform string',
    description: 'Project name',
  },
  {
    name: 'projectVision',
    jsonPath: 'project.vision',
    constraint: 'freeform string',
    description: 'One-sentence elevator pitch',
  },
  {
    name: 'projectType',
    jsonPath: 'project.projectType',
    constraint: 'freeform string (1-30 characters)',
    examples: ['cli-tool', 'web-app', 'saas-platform'],
    description: 'Short descriptor of what kind of software this is',
  },
  {
    name: 'problemTitle',
    jsonPath: 'problemSpace.primaryProblem.title',
    constraint: 'freeform string',
    description: 'Short title of the primary problem the project solves',
  },
  {
    name: 'problemDefinition',
    jsonPath: 'problemSpace.primaryProblem.description',
    constraint: 'freeform string',
    description: 'Detailed description of the primary problem',
  },
  {
    name: 'problemImpact',
    jsonPath: 'problemSpace.primaryProblem.impact',
    constraint: 'enum: high, medium, low',
    description: 'How critical the problem is',
  },
  {
    name: 'solutionOverview',
    jsonPath: 'solutionSpace.overview',
    constraint: 'freeform string',
    description: 'High-level solution approach',
  },
];

/**
 * Render the sections list as human-readable text. The output deliberately
 * includes the section name, JSON path, constraint, description, and any
 * examples on separate lines so weaker LLMs can parse each row reliably.
 */
function renderSectionsAsText(sections: FoundationSectionSpec[]): string {
  const lines: string[] = [];
  lines.push('Foundation Sections (update-foundation field reference)');
  lines.push('=========================================================');
  lines.push('');

  for (const section of sections) {
    lines.push(`• ${section.name}`);
    lines.push(`    path:       ${section.jsonPath}`);
    lines.push(`    constraint: ${section.constraint}`);
    if (section.examples && section.examples.length > 0) {
      lines.push(`    examples:   ${section.examples.join(', ')}`);
    }
    lines.push(`    about:      ${section.description}`);
    lines.push('');
  }

  lines.push(
    'Note: capabilities and personas are managed via dedicated commands'
  );
  lines.push(
    '      (add-capability, add-persona) and cannot be updated via update-foundation.'
  );

  return lines.join('\n');
}

/**
 * List every valid foundation section name with its JSON path and
 * constraint info.
 */
export async function listFoundationSections(
  options: ListFoundationSectionsOptions = {}
): Promise<ListFoundationSectionsResult> {
  const { format = 'text' } = options;

  try {
    let rendered: string;
    if (format === 'json') {
      rendered = JSON.stringify(FOUNDATION_SECTIONS, null, 2);
    } else {
      rendered = renderSectionsAsText(FOUNDATION_SECTIONS);
    }

    return {
      success: true,
      output: rendered,
    };
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    return {
      success: false,
      error: message,
    };
  }
}

/**
 * CLI entry point for list-foundation-sections.
 */
export async function listFoundationSectionsCommand(options?: {
  format?: string;
}): Promise<void> {
  try {
    const result = await listFoundationSections({
      format: (options?.format as 'text' | 'json') || 'text',
    });

    if (!result.success) {
      output.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    output.log(result.output);
    process.exit(0);
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    output.error(chalk.red('Error:'), message);
    process.exit(1);
  }
}

/**
 * Register the list-foundation-sections command with the Commander program.
 */
export function registerListFoundationSectionsCommand(program: Command): void {
  program
    .command('list-foundation-sections')
    .description(
      'List every valid foundation section with its JSON path and constraint info'
    )
    .option(
      '--format <format>',
      'Output format: text (default) or json',
      'text'
    )
    .action(listFoundationSectionsCommand);
}
