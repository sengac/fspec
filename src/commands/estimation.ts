import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';

interface WorkUnit {
  id: string;
  title?: string;
  status?: string;
  estimate?: number;
  actualTokens?: number;
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
  await writeFile(workUnitsFile, JSON.stringify(data, null, 2));
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

export async function recordTokens(
  workUnitId: string,
  tokens: number,
  options: { cwd: string }
): Promise<void> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[workUnitId] as WorkUnit & {
    metrics?: { actualTokens?: number };
  };

  // Store in metrics.actualTokens to match test expectations
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
      estimated: number;
      actualTokens: number;
      iterations: number;
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

    if (!workUnit.actualTokens) {
      throw new Error(`Work unit '${workUnitId}' has no actual metrics`);
    }

    // Determine if within expected range (rough heuristic)
    const expectedTokens = workUnit.estimate * 20000; // ~20k tokens per point
    const variance =
      Math.abs(workUnit.actualTokens - expectedTokens) / expectedTokens;
    const comparison =
      variance < 0.3 ? 'Within expected range' : 'Outside expected range';

    return {
      estimated: workUnit.estimate,
      actualTokens: workUnit.actualTokens,
      iterations: workUnit.iterations || 0,
      comparison,
    };
  }

  // Aggregate query across all completed work
  const completedWorkUnits = Object.values(workUnitsData.workUnits).filter(
    wu => wu.status === 'done' && wu.estimate && wu.actualTokens
  );

  const byPoints: Record<
    string,
    { totalTokens: number; totalIterations: number; count: number }
  > = {};

  for (const wu of completedWorkUnits) {
    const key = `${wu.estimate}-point`;
    if (!byPoints[key]) {
      byPoints[key] = { totalTokens: 0, totalIterations: 0, count: 0 };
    }
    byPoints[key].totalTokens += wu.actualTokens!;
    byPoints[key].totalIterations += wu.iterations || 0;
    byPoints[key].count += 1;
  }

  const result: Record<
    string,
    { avgTokens: number; avgIterations: number; samples: number }
  > = {};

  for (const [key, stats] of Object.entries(byPoints)) {
    result[key] = {
      avgTokens: Math.round(stats.totalTokens / stats.count),
      avgIterations:
        Math.round((stats.totalIterations / stats.count) * 10) / 10,
      samples: stats.count,
    };
  }

  if (output === 'json') {
    return JSON.stringify(result, null, 2);
  }

  // Text output
  let text = 'Estimate Accuracy Analysis\n';
  text += '==========================\n\n';

  for (const [key, stats] of Object.entries(result)) {
    text += `${key}:\n`;
    text += `  Average tokens: ${stats.avgTokens.toLocaleString()}\n`;
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
    wu => wu.status === 'done' && wu.estimate && wu.actualTokens
  );

  const byPrefix: Record<
    string,
    { totalEstimated: number; totalActual: number; count: number }
  > = {};

  for (const wu of completedWorkUnits) {
    const prefix = wu.id.split('-')[0];
    if (!byPrefix[prefix]) {
      byPrefix[prefix] = { totalEstimated: 0, totalActual: 0, count: 0 };
    }

    // Expected tokens based on estimate (rough heuristic: 20k per point)
    const expectedTokens = wu.estimate! * 20000;
    byPrefix[prefix].totalEstimated += expectedTokens;
    byPrefix[prefix].totalActual += wu.actualTokens!;
    byPrefix[prefix].count += 1;
  }

  const result: Record<
    string,
    { avgAccuracy: string; recommendation: string }
  > = {};

  for (const [prefix, stats] of Object.entries(byPrefix)) {
    const accuracyRatio = stats.totalActual / stats.totalEstimated;
    const percentageOff = Math.round((accuracyRatio - 1) * 100);

    let avgAccuracy: string;
    let recommendation: string;

    if (Math.abs(percentageOff) <= 10) {
      avgAccuracy = `estimates ${Math.abs(percentageOff)}% ${percentageOff > 0 ? 'low' : 'high'}`;
      recommendation = 'estimates are well-calibrated';
    } else if (percentageOff > 0) {
      avgAccuracy = `estimates ${percentageOff}% low`;
      const pointsToAdd = Math.ceil(percentageOff / 20);
      recommendation = `increase estimates by ${pointsToAdd}-${pointsToAdd + 1} points`;
    } else {
      avgAccuracy = `estimates ${Math.abs(percentageOff)}% high`;
      const pointsToReduce = Math.ceil(Math.abs(percentageOff) / 20);
      recommendation = `decrease estimates by ${pointsToReduce}-${pointsToReduce + 1} points`;
    }

    result[prefix] = { avgAccuracy, recommendation };
  }

  if (output === 'json') {
    return JSON.stringify(result, null, 2);
  }

  // Text output
  let text = 'Estimate Accuracy by Prefix\n';
  text += '============================\n\n';

  for (const [prefix, stats] of Object.entries(result)) {
    text += `${prefix}:\n`;
    text += `  Accuracy: ${stats.avgAccuracy}\n`;
    text += `  Recommendation: ${stats.recommendation}\n\n`;
  }

  return text;
}

export async function queryEstimationGuide(options: {
  cwd: string;
}): Promise<string> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  const completedWorkUnits = Object.values(workUnitsData.workUnits).filter(
    wu => {
      // Check both actualTokens and metrics.actualTokens
      const hasTokens =
        wu.actualTokens ||
        (wu as WorkUnit & { metrics?: { actualTokens?: number } }).metrics
          ?.actualTokens;
      return wu.status === 'done' && wu.estimate && hasTokens;
    }
  );

  if (completedWorkUnits.length < 3) {
    return 'Insufficient data. Complete at least 3 work units with estimates to generate guidance.';
  }

  const byPoints: Record<
    number,
    { tokenValues: number[]; iterationValues: number[] }
  > = {};

  for (const wu of completedWorkUnits) {
    const points = wu.estimate!;
    const tokens =
      wu.actualTokens ||
      (wu as WorkUnit & { metrics?: { actualTokens?: number } }).metrics
        ?.actualTokens ||
      0;
    const iters = wu.iterations || 0;

    if (!byPoints[points]) {
      byPoints[points] = {
        tokenValues: [],
        iterationValues: [],
      };
    }

    byPoints[points].tokenValues.push(tokens);
    byPoints[points].iterationValues.push(iters);
  }

  let text = 'Estimation Guide (Based on Historical Data)\n';
  text += '============================================\n\n';

  for (const points of FIBONACCI_SEQUENCE) {
    if (!byPoints[points]) continue;

    const stats = byPoints[points];
    const minTokens = Math.min(...stats.tokenValues);
    const maxTokens = Math.max(...stats.tokenValues);
    const avgIter =
      stats.iterationValues.reduce((a, b) => a + b, 0) /
      stats.iterationValues.length;
    const minIter = Math.min(...stats.iterationValues);
    const maxIter = Math.max(...stats.iterationValues);
    const confidence =
      stats.tokenValues.length >= 3
        ? 'high'
        : stats.tokenValues.length >= 2
          ? 'medium'
          : 'low';

    text += `${points} point${points > 1 ? 's' : ''}:\n`;
    text += `  Expected tokens: ${Math.round(minTokens / 1000)}k-${Math.round(maxTokens / 1000)}k\n`;
    text += `  Expected iterations: ${minIter}-${maxIter}\n`;
    text += `  Confidence: ${confidence} (${stats.tokenValues.length} sample${stats.tokenValues.length > 1 ? 's' : ''})\n\n`;
  }

  return text;
}
