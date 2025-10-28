import { readFile } from 'fs/promises';
import { join } from 'path';
import { glob } from 'tinyglobby';
import type { Command } from 'commander';
import { fileManager } from '../utils/file-manager';

interface WorkUnit {
  id: string;
  status?: string;
  metrics?: {
    iterations?: number;
    actualTokens?: number;
  };
  stateHistory?: Array<{ state: string; timestamp: string }>;
  updatedAt: string;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

async function loadWorkUnits(cwd: string): Promise<WorkUnitsData> {
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');
  const content = await readFile(workUnitsFile, 'utf-8');
  return JSON.parse(content);
}

async function saveWorkUnits(data: WorkUnitsData, cwd: string): Promise<void> {
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');
  // LOCK-002: Use fileManager.transaction() for atomic write
  await fileManager.transaction(workUnitsFile, async fileData => {
    Object.assign(fileData, data);
  });
}

export async function recordWorkUnitIteration(
  workUnitId: string,
  options: { cwd: string }
): Promise<void> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[workUnitId];

  if (!workUnit.metrics) {
    workUnit.metrics = {};
  }

  if (!workUnit.metrics.iterations) {
    workUnit.metrics.iterations = 0;
  }

  workUnit.metrics.iterations += 1;
  workUnit.updatedAt = new Date().toISOString();

  await saveWorkUnits(workUnitsData, cwd);
}

export async function recordWorkUnitTokens(
  workUnitId: string,
  tokens: number,
  options: { cwd: string }
): Promise<void> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[workUnitId];

  if (!workUnit.metrics) {
    workUnit.metrics = {};
  }

  if (!workUnit.metrics.actualTokens) {
    workUnit.metrics.actualTokens = 0;
  }

  workUnit.metrics.actualTokens += tokens;
  workUnit.updatedAt = new Date().toISOString();

  await saveWorkUnits(workUnitsData, cwd);
}

export async function autoAdvanceWorkUnitState(
  workUnitId: string,
  transition: {
    fromState: string;
    event: 'tests-pass' | 'validation-pass' | 'specs-complete';
  },
  options: { cwd: string }
): Promise<void> {
  const { cwd } = options;
  const { fromState, event } = transition;
  const workUnitsData = await loadWorkUnits(cwd);

  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[workUnitId];

  // Verify current state matches fromState
  if (workUnit.status !== fromState) {
    throw new Error(
      `Work unit '${workUnitId}' is in state '${workUnit.status}', expected '${fromState}'`
    );
  }

  // Determine next state based on event
  let nextState: string;

  if (event === 'tests-pass' && fromState === 'testing') {
    nextState = 'implementing';
  } else if (event === 'validation-pass' && fromState === 'validating') {
    nextState = 'done';
  } else if (event === 'specs-complete' && fromState === 'specifying') {
    nextState = 'testing';
  } else {
    throw new Error(`Invalid transition: ${event} from ${fromState}`);
  }

  // Update state
  workUnit.status = nextState;

  // Update state history
  if (!workUnit.stateHistory) {
    workUnit.stateHistory = [];
  }
  workUnit.stateHistory.push({
    state: nextState,
    timestamp: new Date().toISOString(),
  });

  // Update states index
  if (workUnitsData.states[fromState]) {
    workUnitsData.states[fromState] = workUnitsData.states[fromState].filter(
      id => id !== workUnitId
    );
  }
  if (!workUnitsData.states[nextState]) {
    workUnitsData.states[nextState] = [];
  }
  if (!workUnitsData.states[nextState].includes(workUnitId)) {
    workUnitsData.states[nextState].push(workUnitId);
  }

  workUnit.updatedAt = new Date().toISOString();

  await saveWorkUnits(workUnitsData, cwd);
}

export async function validateWorkUnitSpecAlignment(
  workUnitId: string,
  options: { cwd: string }
): Promise<{
  aligned: boolean;
  scenariosFound: number;
  features: string[];
}> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const featuresDir = join(cwd, 'spec', 'features');
  const featureFiles = await glob('**/*.feature', { cwd: featuresDir });

  let scenariosFound = 0;
  const features: string[] = [];

  for (const file of featureFiles) {
    const filePath = join(featuresDir, file);
    const content = await readFile(filePath, 'utf-8');

    // Check if file contains tag @WORK-UNIT-ID
    const tagPattern = new RegExp(`@${workUnitId}\\b`, 'g');
    const matches = content.match(tagPattern);

    if (matches) {
      scenariosFound += matches.length;
      features.push(file);
    }
  }

  return {
    aligned: scenariosFound > 0,
    scenariosFound,
    features,
  };
}

export async function workflowAutomation(
  action: string,
  workUnitId: string,
  options: {
    tokens?: number;
    event?: 'tests-pass' | 'validation-pass' | 'specs-complete';
    fromState?: string;
    cwd?: string;
  }
): Promise<void> {
  const cwd = options.cwd || process.cwd();

  if (action === 'record-iteration') {
    await recordWorkUnitIteration(workUnitId, { cwd });
  } else if (action === 'record-tokens' && options.tokens) {
    await recordWorkUnitTokens(workUnitId, options.tokens, { cwd });
  } else if (action === 'auto-advance' && options.event && options.fromState) {
    await autoAdvanceWorkUnitState(
      workUnitId,
      { fromState: options.fromState, event: options.event },
      { cwd }
    );
  } else if (action === 'validate-alignment') {
    await validateWorkUnitSpecAlignment(workUnitId, { cwd });
  } else {
    throw new Error(`Invalid action: ${action}`);
  }
}

export function registerWorkflowAutomationCommand(program: Command): void {
  program
    .command('workflow-automation')
    .description(
      'Workflow automation utilities for work units (record iterations, track tokens, auto-advance state, validate spec alignment)'
    )
    .argument(
      '<action>',
      'Action to perform: record-iteration, record-tokens, auto-advance, validate-alignment'
    )
    .argument('<work-unit-id>', 'Work unit ID (required for all actions)')
    .option(
      '--tokens <count>',
      'Number of tokens to record (used with record-tokens)',
      (value: string) => parseInt(value, 10)
    )
    .option(
      '--event <event>',
      'Event triggering state change: tests-pass, validation-pass, specs-complete (used with auto-advance)'
    )
    .option(
      '--from-state <state>',
      'Current state before transition (used with auto-advance for safety check)'
    )
    .action(workflowAutomation);
}
