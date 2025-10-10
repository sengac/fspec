import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';

interface WorkUnit {
  id: string;
  status?: string;
  completedAt?: string;
  updatedAt: string;
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

interface StateTransition {
  from: string;
  event: string;
  to: string;
  recordCompletion?: boolean;
}

const STATE_TRANSITIONS: StateTransition[] = [
  { from: 'testing', event: 'tests-pass', to: 'implementing' },
  { from: 'validating', event: 'validation-pass', to: 'done', recordCompletion: true },
];

export async function autoAdvance(options: {
  workUnitId: string;
  from: string;
  event: string;
  cwd?: string;
}): Promise<{ success: boolean; newState?: string }> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  try {
    // Load work units
    const content = await readFile(workUnitsFile, 'utf-8');
    const data: WorkUnitsData = JSON.parse(content);

    // Check if work unit exists
    if (!data.workUnits[options.workUnitId]) {
      throw new Error(`Work unit ${options.workUnitId} not found`);
    }

    const workUnit = data.workUnits[options.workUnitId];

    // Find matching transition
    const transition = STATE_TRANSITIONS.find(
      t => t.from === options.from && t.event === options.event
    );

    if (!transition) {
      throw new Error(`No transition defined for ${options.from} + ${options.event}`);
    }

    // Verify current state matches
    if (workUnit.status !== options.from) {
      throw new Error(
        `Work unit is in ${workUnit.status} state, expected ${options.from}`
      );
    }

    // Remove from old state array
    if (data.states[options.from]) {
      data.states[options.from] = data.states[options.from].filter(
        id => id !== options.workUnitId
      );
    }

    // Add to new state array
    if (!data.states[transition.to]) {
      data.states[transition.to] = [];
    }
    data.states[transition.to].push(options.workUnitId);

    // Update work unit
    workUnit.status = transition.to;
    workUnit.updatedAt = new Date().toISOString();

    // Record completion timestamp if transitioning to done
    if (transition.recordCompletion) {
      workUnit.completedAt = new Date().toISOString();
    }

    // Save work units
    await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

    return {
      success: true,
      newState: transition.to,
    };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to auto-advance: ${error.message}`);
    }
    throw error;
  }
}
