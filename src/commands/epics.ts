import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';

interface Epic {
  id: string;
  title: string;
  description?: string;
  workUnits?: string[];
  createdAt: string;
  updatedAt: string;
}

interface EpicsData {
  epics: Record<string, Epic>;
}

interface Prefix {
  name: string;
  description: string;
  color?: string;
}

interface PrefixesData {
  prefixes: Record<string, Prefix>;
}

interface WorkUnit {
  id: string;
  status?: string;
  estimate?: number;
  epic?: string;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

async function loadEpics(cwd: string): Promise<EpicsData> {
  const epicsFile = join(cwd, 'spec', 'epics.json');
  try {
    const content = await readFile(epicsFile, 'utf-8');
    return JSON.parse(content);
  } catch {
    return { epics: {} };
  }
}

async function saveEpics(data: EpicsData, cwd: string): Promise<void> {
  const epicsFile = join(cwd, 'spec', 'epics.json');
  await writeFile(epicsFile, JSON.stringify(data, null, 2));
}

async function loadPrefixes(cwd: string): Promise<PrefixesData> {
  const prefixesFile = join(cwd, 'spec', 'prefixes.json');
  try {
    const content = await readFile(prefixesFile, 'utf-8');
    return JSON.parse(content);
  } catch {
    return { prefixes: {} };
  }
}

async function savePrefixes(data: PrefixesData, cwd: string): Promise<void> {
  const prefixesFile = join(cwd, 'spec', 'prefixes.json');
  await writeFile(prefixesFile, JSON.stringify(data, null, 2));
}

async function loadWorkUnits(cwd: string): Promise<WorkUnitsData> {
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');
  const content = await readFile(workUnitsFile, 'utf-8');
  return JSON.parse(content);
}

export async function createEpic(
  epicId: string,
  title: string,
  options: { cwd: string; description?: string }
): Promise<void> {
  const { cwd, description } = options;

  // Validate epic ID format (lowercase-with-hyphens)
  if (!/^[a-z][a-z0-9-]*$/.test(epicId)) {
    throw new Error('Epic ID must be lowercase with hyphens (e.g., epic-user-management)');
  }

  const epicsData = await loadEpics(cwd);

  if (epicsData.epics[epicId]) {
    throw new Error(`Epic '${epicId}' already exists`);
  }

  epicsData.epics[epicId] = {
    id: epicId,
    title,
    description,
    workUnits: [],
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString()
  };

  await saveEpics(epicsData, cwd);
}

export async function createPrefix(
  prefix: string,
  description: string,
  options: { cwd: string; color?: string }
): Promise<void> {
  const { cwd, color } = options;

  // Validate prefix format (2-6 uppercase letters)
  if (!/^[A-Z]{2,6}$/.test(prefix)) {
    throw new Error('Prefix must be 2-6 uppercase letters (e.g., AUTH, DASH, API)');
  }

  const prefixesData = await loadPrefixes(cwd);

  if (prefixesData.prefixes[prefix]) {
    throw new Error(`Prefix '${prefix}' already exists`);
  }

  prefixesData.prefixes[prefix] = {
    name: prefix,
    description,
    color
  };

  await savePrefixes(prefixesData, cwd);
}

export async function updatePrefix(
  prefix: string,
  updates: { epic?: string; description?: string; color?: string },
  options: { cwd: string }
): Promise<void> {
  const { cwd } = options;
  const prefixesData = await loadPrefixes(cwd);

  if (!prefixesData.prefixes[prefix]) {
    throw new Error(`Prefix '${prefix}' does not exist`);
  }

  const prefixData = prefixesData.prefixes[prefix] as Prefix & { epic?: string };

  if (updates.epic !== undefined) {
    prefixData.epic = updates.epic;
  }

  if (updates.description !== undefined) {
    prefixData.description = updates.description;
  }

  if (updates.color !== undefined) {
    prefixData.color = updates.color;
  }

  await savePrefixes(prefixesData, cwd);
}

export async function showEpicProgress(
  epicId: string,
  options: { cwd: string }
): Promise<string> {
  const { cwd } = options;
  const epicsData = await loadEpics(cwd);

  if (!epicsData.epics[epicId]) {
    throw new Error(`Epic '${epicId}' does not exist`);
  }

  const epic = epicsData.epics[epicId];
  const workUnitsData = await loadWorkUnits(cwd);

  // Get work units for this epic - check epic field OR epic's workUnits array
  const epicWorkUnits = Object.values(workUnitsData.workUnits).filter(wu =>
    wu.epic === epicId || (epic.workUnits && epic.workUnits.includes(wu.id))
  );

  const total = epicWorkUnits.length;
  const completed = epicWorkUnits.filter(wu => wu.status === 'done').length;
  const percentage = total > 0 ? Math.round((completed / total) * 100) : 0;

  const totalPoints = epicWorkUnits.reduce((sum, wu) => sum + (wu.estimate || 0), 0);
  const completedPoints = epicWorkUnits
    .filter(wu => wu.status === 'done')
    .reduce((sum, wu) => sum + (wu.estimate || 0), 0);

  let output = `Epic: ${epic.title}\n`;
  output += `ID: ${epicId}\n\n`;
  output += `Progress: ${completed} of ${total} complete (${percentage}%)\n`;
  output += `Story Points: ${completedPoints} of ${totalPoints} points\n\n`;

  if (epic.description) {
    output += `Description: ${epic.description}\n\n`;
  }

  output += `Work Units:\n`;
  for (const wu of epicWorkUnits) {
    const status = wu.status || 'backlog';
    const estimate = wu.estimate ? `${wu.estimate} pts` : 'no estimate';
    output += `  - ${wu.id} [${status}] (${estimate})\n`;
  }

  return output;
}

export async function listEpics(options: { cwd: string; output?: string }): Promise<string> {
  const { cwd, output } = options;
  const epicsData = await loadEpics(cwd);

  // Try to load work units, but don't fail if file doesn't exist
  let workUnitsData: WorkUnitsData;
  try {
    workUnitsData = await loadWorkUnits(cwd);
  } catch {
    workUnitsData = { workUnits: {}, states: {} };
  }

  if (output === 'json') {
    return JSON.stringify(epicsData.epics, null, 2);
  }

  let text = 'Epics\n';
  text += '=====\n\n';

  if (Object.keys(epicsData.epics).length === 0) {
    text += 'No epics defined.\n';
    return text;
  }

  for (const [id, epic] of Object.entries(epicsData.epics)) {
    // Get work units for this epic - check epic field OR epic's workUnits array
    const workUnits = Object.values(workUnitsData.workUnits).filter(wu =>
      wu.epic === id || (epic.workUnits && epic.workUnits.includes(wu.id))
    );
    const completed = workUnits.filter(wu => wu.status === 'done').length;

    text += `${id}: ${epic.title}\n`;
    text += `  Work units: ${workUnits.length} (${completed} completed)\n`;
    if (epic.description) {
      text += `  Description: ${epic.description}\n`;
    }
    text += '\n';
  }

  return text;
}

export async function deleteEpic(
  epicId: string,
  options: { cwd: string; force?: boolean },
  options2?: { cwd: string }
): Promise<void> {
  // Handle both function signatures for backward compatibility
  const cwd = options.cwd || options2?.cwd;
  const force = options.force;

  if (!cwd) {
    throw new Error('cwd is required');
  }

  const epicsData = await loadEpics(cwd);

  if (!epicsData.epics[epicId]) {
    throw new Error(`Epic '${epicId}' does not exist`);
  }

  delete epicsData.epics[epicId];
  await saveEpics(epicsData, cwd);

  // If force flag is set, clear epic field from all work units
  if (force) {
    try {
      const workUnitsData = await loadWorkUnits(cwd);
      let updated = false;

      for (const workUnit of Object.values(workUnitsData.workUnits)) {
        if (workUnit.epic === epicId) {
          delete workUnit.epic;
          updated = true;
        }
      }

      if (updated) {
        const workUnitsFile = join(cwd, 'spec', 'work-units.json');
        await writeFile(workUnitsFile, JSON.stringify(workUnitsData, null, 2));
      }
    } catch {
      // If work-units.json doesn't exist, that's fine
    }
  }
}
