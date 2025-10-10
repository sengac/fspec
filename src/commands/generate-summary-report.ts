import { readFile } from 'fs/promises';
import { join } from 'path';

interface WorkUnit {
  id: string;
  status?: string;
  estimate?: number;
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

interface SummaryReport {
  totalWorkUnits: number;
  byStatus: Record<string, number>;
  totalStoryPoints: number;
  velocity: {
    completedPoints: number;
    completedWorkUnits: number;
  };
}

export async function generateSummaryReport(options: {
  cwd?: string;
}): Promise<SummaryReport> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  try {
    const content = await readFile(workUnitsFile, 'utf-8');
    const data: WorkUnitsData = JSON.parse(content);

    const workUnits = Object.values(data.workUnits);

    // Count by status
    const byStatus: Record<string, number> = {};
    for (const wu of workUnits) {
      const status = wu.status || 'unknown';
      byStatus[status] = (byStatus[status] || 0) + 1;
    }

    // Calculate story points
    const totalStoryPoints = workUnits.reduce(
      (sum, wu) => sum + (wu.estimate || 0),
      0
    );

    // Calculate velocity (completed work)
    const completedWorkUnits = workUnits.filter(wu => wu.status === 'done');
    const completedPoints = completedWorkUnits.reduce(
      (sum, wu) => sum + (wu.estimate || 0),
      0
    );

    return {
      totalWorkUnits: workUnits.length,
      byStatus,
      totalStoryPoints,
      velocity: {
        completedPoints,
        completedWorkUnits: completedWorkUnits.length,
      },
    };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to generate summary report: ${error.message}`);
    }
    throw error;
  }
}
