import { readFile, writeFile } from 'fs/promises';
import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';

interface WorkUnit {
  id: string;
  title?: string;
  status?: string;
  epic?: string;
  estimate?: number;
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

interface StateHistoryEntry {
  state: string;
  timestamp: string;
  reason?: string;
}

export async function queryWorkUnits(options: {
  workUnitId?: string;
  status?: string;
  epic?: string;
  prefix?: string;
  type?: 'story' | 'task' | 'bug';
  sort?: string;
  order?: 'asc' | 'desc';
  format?: 'json' | 'csv' | 'text';
  output?: string;
  showCycleTime?: boolean;
  hasQuestions?: boolean;
  questionsFor?: string;
  cwd?: string;
}): Promise<{
  workUnits?: WorkUnit[];
  stateTimings?: Record<string, string>;
  totalCycleTime?: string;
}> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  try {
    const content = await readFile(workUnitsFile, 'utf-8');
    const data: WorkUnitsData = JSON.parse(content);

    // Single work unit with cycle time
    if (options.workUnitId && options.showCycleTime) {
      const workUnit = data.workUnits[options.workUnitId];
      if (!workUnit) {
        throw new Error(`Work unit '${options.workUnitId}' does not exist`);
      }

      const stateHistory = (workUnit.stateHistory || []) as StateHistoryEntry[];
      const stateTimings: Record<string, string> = {};
      let totalMs = 0;

      for (let i = 0; i < stateHistory.length - 1; i++) {
        const current = stateHistory[i];
        const next = stateHistory[i + 1];
        const currentTime = new Date(current.timestamp);
        const nextTime = new Date(next.timestamp);
        const durationMs = nextTime.getTime() - currentTime.getTime();
        const durationHours = Math.round(durationMs / (1000 * 60 * 60));

        stateTimings[current.state] =
          `${durationHours} hour${durationHours !== 1 ? 's' : ''}`;
        totalMs += durationMs;
      }

      const totalHours = Math.round(totalMs / (1000 * 60 * 60));

      return {
        stateTimings,
        totalCycleTime: `${totalHours} hour${totalHours !== 1 ? 's' : ''}`,
      };
    }

    // Get all work units as array
    let workUnits = Object.values(data.workUnits);

    // Apply filters
    if (options.status) {
      workUnits = workUnits.filter(wu => wu.status === options.status);
    }

    if (options.epic) {
      workUnits = workUnits.filter(wu => wu.epic === options.epic);
    }

    if (options.prefix) {
      workUnits = workUnits.filter(wu => wu.id.startsWith(options.prefix + '-'));
    }

    // Filter by type
    if (options.type) {
      workUnits = workUnits.filter(wu => {
        const type = wu.type || 'story'; // Default to 'story' for backward compatibility
        return type === options.type;
      });
    }

    // Filter by hasQuestions
    if (options.hasQuestions !== undefined) {
      if (options.hasQuestions) {
        workUnits = workUnits.filter(wu => (wu.questions?.length || 0) > 0);
      } else {
        workUnits = workUnits.filter(wu => (wu.questions?.length || 0) === 0);
      }
    }

    // Filter by questionsFor (person mentioned)
    if (options.questionsFor) {
      // Handle both @bob and bob formats
      const mention = options.questionsFor.startsWith('@')
        ? options.questionsFor
        : `@${options.questionsFor}`;
      workUnits = workUnits.filter(wu => {
        if (!wu.questions) {
          return false;
        }
        return wu.questions.some(q => {
          // Handle QuestionItem objects
          if (typeof q === 'object' && 'text' in q) {
            return q.text.includes(mention);
          }
          // Backward compatibility for string questions
          if (typeof q === 'string') {
            return q.includes(mention);
          }
          return false;
        });
      });
    }

    // Apply sorting
    if (options.sort) {
      const sortKey = options.sort as keyof WorkUnit;
      const order = options.order || 'asc';

      workUnits.sort((a, b) => {
        const aVal = a[sortKey];
        const bVal = b[sortKey];

        if (aVal === undefined || bVal === undefined) {
          return 0;
        }

        let comparison = 0;
        if (typeof aVal === 'string' && typeof bVal === 'string') {
          comparison = aVal.localeCompare(bVal);
        } else if (typeof aVal === 'number' && typeof bVal === 'number') {
          comparison = aVal - bVal;
        }

        return order === 'desc' ? -comparison : comparison;
      });
    }

    // Handle output format
    if (options.format === 'csv' && options.output) {
      const csvLines: string[] = [];

      // Header
      csvLines.push('id,title,status,createdAt,updatedAt');

      // Data rows
      for (const wu of workUnits) {
        const title = (wu.title || '').replace(/,/g, '');
        const status = wu.status || '';
        const createdAt = (wu.createdAt as string) || '';
        const updatedAt = (wu.updatedAt as string) || '';
        csvLines.push(`${wu.id},${title},${status},${createdAt},${updatedAt}`);
      }

      await writeFile(options.output, csvLines.join('\n'), 'utf-8');
    }

    return { workUnits };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to query work units: ${error.message}`);
    }
    throw error;
  }
}

export function registerQueryWorkUnitsCommand(program: Command): void {
  program
    .command('query-work-units')
    .description('Query work units with advanced filters')
    .option('--status <status>', 'Filter by status')
    .option('--prefix <prefix>', 'Filter by prefix')
    .option('--epic <epic>', 'Filter by epic')
    .option('--type <type>', 'Filter by work item type: story, task, or bug')
    .option('--format <format>', 'Output format: text or json', 'text')
    .action(
      async (options: {
        status?: string;
        prefix?: string;
        epic?: string;
        type?: 'story' | 'task' | 'bug';
        format?: string;
      }) => {
        try {
          const result = await queryWorkUnits({
            status: options.status as any,
            prefix: options.prefix,
            epic: options.epic,
            type: options.type,
            format: options.format as 'text' | 'json',
          });
          if (options.format === 'json') {
            console.log(JSON.stringify(result, null, 2));
          }
        } catch (error: any) {
          console.error(chalk.red('✗ Query failed:'), error.message);
          process.exit(1);
        }
      }
    );
}
