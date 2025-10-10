import { readFile } from 'fs/promises';
import { join } from 'path';

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
