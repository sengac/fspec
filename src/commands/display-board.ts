import { readFile } from 'fs/promises';
import { join } from 'path';
import type { Command } from 'commander';
import chalk from 'chalk';

interface WorkUnit {
  id: string;
  title?: string;
  status?: string;
  estimate?: number;
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

interface BoardColumn {
  id: string;
  title?: string;
  estimate?: number;
}

interface BoardResult {
  columns?: Record<string, BoardColumn[]>;
  board: Record<string, string[]>;
  summary: string;
}

export async function displayBoard(options: {
  cwd?: string;
}): Promise<BoardResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  try {
    const content = await readFile(workUnitsFile, 'utf-8');
    const data: WorkUnitsData = JSON.parse(content);

    // Group work units by state
    const columns: Record<string, BoardColumn[]> = {};
    const board: Record<string, string[]> = {};

    let inProgressPoints = 0;
    let completedPoints = 0;

    for (const [status, workUnitIds] of Object.entries(data.states)) {
      columns[status] = workUnitIds.map(id => {
        const wu = data.workUnits[id];
        return {
          id: wu.id,
          title: wu.title,
          estimate: wu.estimate,
        };
      });

      board[status] = workUnitIds;

      // Calculate points
      for (const id of workUnitIds) {
        const wu = data.workUnits[id];
        if (wu.estimate) {
          if (status === 'done') {
            completedPoints += wu.estimate;
          } else {
            inProgressPoints += wu.estimate;
          }
        }
      }
    }

    const summary = `${inProgressPoints} points in progress, ${completedPoints} points completed`;

    return { columns, board, summary };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to display board: ${error.message}`);
    }
    throw error;
  }
}

export function registerBoardCommand(program: Command): void {
  program
    .command('board')
    .description('Display Kanban board of work units')
    .option('--format <format>', 'Output format: text or json', 'text')
    .option('--limit <limit>', 'Max items per column', '25')
    .action(async (options: { format?: string; limit?: string }) => {
      try {
        const result = await displayBoard({ cwd: process.cwd() });

        if (options.format === 'json') {
          console.log(JSON.stringify(result, null, 2));
        } else {
          // Display text format board using Ink
          const limit = parseInt(options.limit || '25', 10);
          const { render } = await import('ink');
          const React = await import('react');
          const { BoardDisplay } = await import('../components/BoardDisplay.js');

          render(
            React.createElement(BoardDisplay, {
              columns: result.columns,
              board: result.board,
              summary: result.summary,
              limit,
            })
          );
        }
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to display board:'), error.message);
        process.exit(1);
      }
    });
}
