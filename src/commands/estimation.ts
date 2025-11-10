import { readFile } from 'fs/promises';
import { join } from 'path';
import { fileManager } from '../utils/file-manager';

interface WorkUnit {
  id: string;
  title?: string;
  status?: string;
  estimate?: number;
  iterations?: number;
  stateHistory?: Array<{ state: string; timestamp: string }>;
  createdAt?: string;
  updatedAt: string;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

const FIBONACCI_SEQUENCE = [1, 2, 3, 5, 8, 13, 21];

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

export async function assignEstimate(
  workUnitId: string,
  estimate: number,
  options: { cwd: string }
): Promise<void> {
  const { cwd } = options;

  // Validate Fibonacci sequence
  if (!FIBONACCI_SEQUENCE.includes(estimate)) {
    throw new Error(
      `Estimate must be Fibonacci sequence value: ${FIBONACCI_SEQUENCE.join(', ')}`
    );
  }

  const workUnitsData = await loadWorkUnits(cwd);

  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[workUnitId];
  workUnit.estimate = estimate;
  workUnit.updatedAt = new Date().toISOString();

  await saveWorkUnits(workUnitsData, cwd);
}

export async function incrementIteration(
  workUnitId: string,
  options: { cwd: string }
): Promise<void> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[workUnitId] as WorkUnit & {
    metrics?: { iterations?: number };
  };

  // Store in metrics.iterations to match test expectations
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

export async function calculateCycleTime(
  workUnitId: string,
  options: { cwd: string }
): Promise<number> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[workUnitId];

  if (!workUnit.stateHistory || workUnit.stateHistory.length === 0) {
    return 0;
  }

  // Find first timestamp (backlog or created)
  const firstTimestamp = workUnit.stateHistory[0].timestamp;

  // Find last timestamp (current state)
  const lastTimestamp =
    workUnit.stateHistory[workUnit.stateHistory.length - 1].timestamp;

  // Calculate hours between
  const startTime = new Date(firstTimestamp).getTime();
  const endTime = new Date(lastTimestamp).getTime();
  const hours = (endTime - startTime) / (1000 * 60 * 60);

  return Math.round(hours);
}

export async function queryEstimateAccuracy(
  workUnitId: string | null,
  options: { cwd: string; output?: string }
): Promise<
  | string
  | {
      estimated: string;
      actual: string;
      comparison: string;
    }
> {
  const { cwd, output } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  // Single work unit query
  if (workUnitId) {
    if (!workUnitsData.workUnits[workUnitId]) {
      throw new Error(`Work unit '${workUnitId}' does not exist`);
    }

    const workUnit = workUnitsData.workUnits[workUnitId];

    if (!workUnit.estimate) {
      throw new Error(`Work unit '${workUnitId}' has no estimate`);
    }

    // Token tracking removed - always return 0 tokens
    const comparison = 'N/A (token tracking removed)';

    return {
      estimated: `${workUnit.estimate} points`,
      actual: `${workUnit.iterations || 0} iterations`,
      comparison,
    };
  }

  // Aggregate query across all completed work
  const completedWorkUnits = Object.values(workUnitsData.workUnits).filter(
    wu => wu.status === 'done' && wu.estimate
  );

  const byPoints: Record<string, { totalIterations: number; count: number }> =
    {};

  for (const wu of completedWorkUnits) {
    const key = `${wu.estimate}`;
    if (!byPoints[key]) {
      byPoints[key] = { totalIterations: 0, count: 0 };
    }
    byPoints[key].totalIterations += wu.iterations || 0;
    byPoints[key].count += 1;
  }

  const result: Record<string, { avgIterations: number; samples: number }> = {};

  for (const [key, stats] of Object.entries(byPoints)) {
    result[key] = {
      avgIterations:
        Math.round((stats.totalIterations / stats.count) * 10) / 10,
      samples: stats.count,
    };
  }

  if (output === 'json') {
    return JSON.stringify({ byStoryPoints: result }, null, 2);
  }

  // Text output
  let text = 'Estimate Accuracy Analysis\n';
  text += '==========================\n\n';

  for (const [key, stats] of Object.entries(result)) {
    text += `${key} point${key !== '1' ? 's' : ''}:\n`;
    text += `  Average iterations: ${stats.avgIterations}\n`;
    text += `  Samples: ${stats.samples}\n\n`;
  }

  return text;
}

export async function queryEstimateAccuracyByPrefix(options: {
  cwd: string;
  output?: string;
}): Promise<string> {
  const { cwd, output } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  const completedWorkUnits = Object.values(workUnitsData.workUnits).filter(
    wu => wu.status === 'done' && wu.estimate
  );

  const byPrefix: Record<string, { totalIterations: number; count: number }> =
    {};

  for (const wu of completedWorkUnits) {
    const prefix = wu.id.split('-')[0];
    if (!byPrefix[prefix]) {
      byPrefix[prefix] = { totalIterations: 0, count: 0 };
    }

    byPrefix[prefix].totalIterations += wu.iterations || 0;
    byPrefix[prefix].count += 1;
  }

  const result: Record<string, { avgAccuracy: number; samples: number }> = {};

  for (const [prefix, stats] of Object.entries(byPrefix)) {
    result[prefix] = {
      avgAccuracy: Math.round((stats.totalIterations / stats.count) * 10) / 10,
      samples: stats.count,
    };
  }

  if (output === 'json') {
    return JSON.stringify(result, null, 2);
  }

  // Text output
  let text = 'Estimate Accuracy by Prefix\n';
  text += '============================\n\n';

  for (const [prefix, stats] of Object.entries(result)) {
    text += `${prefix}:\n`;
    text += `  Average iterations: ${stats.avgAccuracy}\n`;
    text += `  Samples: ${stats.samples}\n\n`;
  }

  return text;
}

export async function queryEstimationGuide(options: {
  cwd: string;
}): Promise<string> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  const completedWorkUnits = Object.values(workUnitsData.workUnits).filter(
    wu => wu.status === 'done' && wu.estimate
  );

  if (completedWorkUnits.length < 3) {
    return 'Insufficient data. Complete at least 3 work units with estimates to generate guidance.';
  }

  const byPoints: Record<number, { iterationValues: number[] }> = {};

  for (const wu of completedWorkUnits) {
    const points = wu.estimate!;
    const iters = wu.iterations || 0;

    if (!byPoints[points]) {
      byPoints[points] = {
        iterationValues: [],
      };
    }

    byPoints[points].iterationValues.push(iters);
  }

  let text = 'Estimation Guide (Based on Historical Data)\n';
  text += '============================================\n\n';

  for (const points of FIBONACCI_SEQUENCE) {
    if (!byPoints[points]) continue;

    const stats = byPoints[points];
    const avgIter =
      stats.iterationValues.reduce((a, b) => a + b, 0) /
      stats.iterationValues.length;
    const minIter = Math.min(...stats.iterationValues);
    const maxIter = Math.max(...stats.iterationValues);
    const confidence =
      stats.iterationValues.length >= 3
        ? 'high'
        : stats.iterationValues.length >= 2
          ? 'medium'
          : 'low';

    text += `${points} point${points > 1 ? 's' : ''}:\n`;
    text += `  Expected iterations: ${minIter}-${maxIter} (avg: ${avgIter.toFixed(1)})\n`;
    text += `  Confidence: ${confidence} (${stats.iterationValues.length} sample${stats.iterationValues.length > 1 ? 's' : ''})\n\n`;
  }

  return text;
}
