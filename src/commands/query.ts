import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';

interface WorkUnit {
  id: string;
  title?: string;
  status?: string;
  estimate?: number;
  epic?: string;
  actualTokens?: number;
  iterations?: number;
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

export async function queryWorkUnitsByStatus(
  status: string,
  options: { cwd: string; output?: string }
): Promise<string> {
  const { cwd, output } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  const results = Object.values(workUnitsData.workUnits).filter(
    wu => wu.status === status
  );

  if (output === 'json') {
    return JSON.stringify(results, null, 2);
  }

  // Text output
  let text = `Found ${results.length} work unit${results.length !== 1 ? 's' : ''} with status: ${status}\n\n`;

  for (const wu of results) {
    text += `${wu.id}: ${wu.title || '(no title)'}\n`;
    text += `  Status: ${wu.status || 'backlog'}\n`;
    if (wu.estimate) {
      text += `  Estimate: ${wu.estimate} points\n`;
    }
    text += '\n';
  }

  return text;
}

export async function queryWorkUnitsByEpic(
  epic: string,
  options: { cwd: string; output?: string }
): Promise<string> {
  const { cwd, output } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  const results = Object.values(workUnitsData.workUnits).filter(
    wu => wu.epic === epic
  );

  if (output === 'json') {
    return JSON.stringify(results, null, 2);
  }

  // Text output
  let text = `Found ${results.length} work unit${results.length !== 1 ? 's' : ''} in epic: ${epic}\n\n`;

  for (const wu of results) {
    text += `${wu.id}: ${wu.title || '(no title)'}\n`;
    text += `  Status: ${wu.status || 'backlog'}\n`;
    if (wu.estimate) {
      text += `  Estimate: ${wu.estimate} points\n`;
    }
    text += '\n';
  }

  return text;
}

export async function queryWorkUnitsCompound(
  filters: {
    status?: string;
    epic?: string;
    prefix?: string;
    estimateGte?: number;
    estimateLte?: number;
    hasQuestions?: boolean;
    output?: string;
  },
  options: { cwd: string }
): Promise<string> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  let results = Object.values(workUnitsData.workUnits);

  // Apply filters (AND logic)
  if (filters.status) {
    results = results.filter(wu => wu.status === filters.status);
  }

  if (filters.epic) {
    results = results.filter(wu => wu.epic === filters.epic);
  }

  if (filters.prefix) {
    results = results.filter(wu => wu.id.startsWith(filters.prefix + '-'));
  }

  if (filters.estimateGte !== undefined) {
    results = results.filter(
      wu => wu.estimate && wu.estimate >= filters.estimateGte!
    );
  }

  if (filters.estimateLte !== undefined) {
    results = results.filter(
      wu => wu.estimate && wu.estimate <= filters.estimateLte!
    );
  }

  if (filters.hasQuestions) {
    results = results.filter(wu => {
      const unit = wu as WorkUnit & { questions?: string[] };
      return unit.questions && unit.questions.length > 0;
    });
  }

  if (filters.output === 'json') {
    return JSON.stringify(results, null, 2);
  }

  // Text output
  let text = `Found ${results.length} work unit${results.length !== 1 ? 's' : ''}\n\n`;

  for (const wu of results) {
    text += `${wu.id}: ${wu.title || '(no title)'}\n`;
    text += `  Status: ${wu.status || 'backlog'}\n`;
    if (wu.estimate) {
      text += `  Estimate: ${wu.estimate} points\n`;
    }
    if (wu.epic) {
      text += `  Epic: ${wu.epic}\n`;
    }
    text += '\n';
  }

  return text;
}

export async function generateStatisticalReport(options: {
  cwd: string;
}): Promise<string> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  const workUnits = Object.values(workUnitsData.workUnits);
  const total = workUnits.length;

  const byStatus: Record<string, number> = {};
  const pointsByStatus: Record<string, number> = {};

  for (const wu of workUnits) {
    const status = wu.status || 'backlog';
    byStatus[status] = (byStatus[status] || 0) + 1;
    pointsByStatus[status] = (pointsByStatus[status] || 0) + (wu.estimate || 0);
  }

  const totalPoints = Object.values(pointsByStatus).reduce(
    (sum, points) => sum + points,
    0
  );
  const completedPoints = pointsByStatus.done || 0;
  const inProgressPoints =
    (pointsByStatus.implementing || 0) + (pointsByStatus.validating || 0);
  const remainingPoints =
    (pointsByStatus.backlog || 0) +
    (pointsByStatus.specifying || 0) +
    (pointsByStatus.testing || 0);

  let report = 'Statistical Report\n';
  report += '==================\n\n';

  report += `Total work units: ${total}\n`;
  report += `Total story points: ${totalPoints}\n\n`;

  report += 'By Status:\n';
  for (const [status, count] of Object.entries(byStatus)) {
    const points = pointsByStatus[status] || 0;
    report += `  ${status}: ${count} work units (${points} points)\n`;
  }
  report += '\n';

  report += 'Summary:\n';
  report += `  Completed: ${completedPoints} points\n`;
  report += `  In progress: ${inProgressPoints} points\n`;
  report += `  Remaining: ${remainingPoints} points\n`;

  return report;
}

export async function exportWorkUnits(
  options: {
    output: string;
    format: 'json' | 'csv' | 'markdown';
    status?: string;
  },
  config: { cwd: string }
): Promise<void> {
  const { cwd } = config;
  const { output, format, status } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  let workUnits = Object.values(workUnitsData.workUnits);

  // Filter by status if provided
  if (status) {
    workUnits = workUnits.filter(wu => wu.status === status);
  }

  if (format === 'json') {
    await writeFile(output, JSON.stringify(workUnits, null, 2));
    return;
  }

  if (format === 'csv') {
    let csv = 'id,title,status,estimate\n';
    for (const wu of workUnits) {
      csv += `${wu.id},${wu.title || ''},${wu.status || 'backlog'},${wu.estimate || ''}\n`;
    }
    await writeFile(output, csv);
    return;
  }

  if (format === 'markdown') {
    let md = '# Work Units\n\n';
    md += '| ID | Title | Status | Estimate |\n';
    md += '|----|-------|--------|----------|\n';
    for (const wu of workUnits) {
      md += `| ${wu.id} | ${wu.title || ''} | ${wu.status || 'backlog'} | ${wu.estimate || ''} |\n`;
    }
    await writeFile(output, md);
    return;
  }
}

export async function displayKanbanBoard(options: {
  cwd: string;
}): Promise<string> {
  const { cwd } = options;
  const workUnitsData = await loadWorkUnits(cwd);

  const states = [
    'backlog',
    'specifying',
    'testing',
    'implementing',
    'validating',
    'done',
  ];
  const columnWidth = 20;

  // Group work units by state
  const byState: Record<string, WorkUnit[]> = {};
  for (const state of states) {
    byState[state] = [];
  }

  for (const wu of Object.values(workUnitsData.workUnits)) {
    const state = wu.status || 'backlog';
    if (byState[state]) {
      byState[state].push(wu);
    }
  }

  // Build ASCII board
  let board = '';

  // Header
  board += '┌';
  for (let i = 0; i < states.length; i++) {
    board += '─'.repeat(columnWidth);
    if (i < states.length - 1) board += '┬';
  }
  board += '┐\n';

  // State names with counts
  board += '│';
  for (const state of states) {
    const count = byState[state].length;
    const stateName = state.charAt(0).toUpperCase() + state.slice(1);
    let label = `${stateName} (${count})`;
    // Truncate label if it's too long
    if (label.length > columnWidth - 1) {
      label = label.substring(0, columnWidth - 1);
    }
    const padding = Math.max(0, columnWidth - label.length);
    board += label + ' '.repeat(padding) + '│';
  }
  board += '\n';

  // Separator
  board += '├';
  for (let i = 0; i < states.length; i++) {
    board += '─'.repeat(columnWidth);
    if (i < states.length - 1) board += '┼';
  }
  board += '┤\n';

  // Work units (show first 3 per column)
  const maxRows = Math.max(...states.map(s => Math.min(byState[s].length, 3)));

  // Show up to 2 rows per work unit (first row: ID+title, second row: estimate)
  const maxDisplayRows = Math.max(
    ...states.map(s => Math.min(byState[s].length * 2, 6))
  );

  for (let row = 0; row < maxDisplayRows || row < 1; row++) {
    board += '│';
    for (const state of states) {
      const wuIndex = Math.floor(row / 2);
      const isFirstRow = row % 2 === 0;
      const wu = byState[state][wuIndex];

      if (wu) {
        if (isFirstRow) {
          // First row: ID and title
          let label = `${wu.id} ${wu.title || ''}`;
          if (label.length > columnWidth - 1) {
            label = label.substring(0, columnWidth - 4) + '...';
          }
          const padding = Math.max(0, columnWidth - label.length);
          board += label + ' '.repeat(padding) + '│';
        } else {
          // Second row: estimate
          const estimate = wu.estimate ? `[${wu.estimate}pts]` : '';
          const padding = Math.max(0, columnWidth - estimate.length);
          board += estimate + ' '.repeat(padding) + '│';
        }
      } else {
        board += ' '.repeat(columnWidth) + '│';
      }
    }
    board += '\n';
  }

  // Footer
  board += '└';
  for (let i = 0; i < states.length; i++) {
    board += '─'.repeat(columnWidth);
    if (i < states.length - 1) board += '┴';
  }
  board += '┘\n';

  // Summary
  const totalInProgress = states
    .filter(s => s !== 'done' && s !== 'backlog')
    .reduce(
      (sum, s) =>
        sum + byState[s].reduce((s2, wu) => s2 + (wu.estimate || 0), 0),
      0
    );

  const totalCompleted = byState.done.reduce(
    (sum, wu) => sum + (wu.estimate || 0),
    0
  );

  board += `\nTotal: ${totalInProgress} points in progress | ${totalCompleted} points completed\n`;

  return board;
}
