import { readFile } from 'fs/promises';
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

        stateTimings[current.state] = `${durationHours} hour${durationHours !== 1 ? 's' : ''}`;
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
        return wu.questions.some(q => q.includes(mention));
      });
    }

    return { workUnits };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to query work units: ${error.message}`);
    }
    throw error;
  }
}
