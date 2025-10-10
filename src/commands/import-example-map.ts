import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData } from '../types';

interface ExampleMapData {
  rules?: string[];
  examples?: string[];
  questions?: string[];
  assumptions?: string[];
}

interface ImportExampleMapOptions {
  workUnitId: string;
  file: string;
  cwd?: string;
}

interface ImportExampleMapResult {
  success: boolean;
  imported: {
    rules: number;
    examples: number;
    questions: number;
    assumptions: number;
  };
}

export async function importExampleMap(
  options: ImportExampleMapOptions
): Promise<ImportExampleMapResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec/work-units.json');
  const jsonFilePath = options.file;

  // Read work units
  const workUnitsContent = await readFile(workUnitsFile, 'utf-8');
  const data: WorkUnitsData = JSON.parse(workUnitsContent);

  // Validate work unit exists
  if (!data.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = data.workUnits[options.workUnitId];

  // Validate work unit is in specifying state
  if (workUnit.status !== 'specifying') {
    throw new Error(
      `Can only import example mapping during discovery/specification phase. ${options.workUnitId} is in '${workUnit.status}' state.`
    );
  }

  // Read JSON file
  const jsonContent = await readFile(jsonFilePath, 'utf-8');
  const exampleMapData: ExampleMapData = JSON.parse(jsonContent);

  // Import data
  const imported = {
    rules: 0,
    examples: 0,
    questions: 0,
    assumptions: 0,
  };

  if (exampleMapData.rules && Array.isArray(exampleMapData.rules)) {
    workUnit.rules = [...(workUnit.rules || []), ...exampleMapData.rules];
    imported.rules = exampleMapData.rules.length;
  }

  if (exampleMapData.examples && Array.isArray(exampleMapData.examples)) {
    workUnit.examples = [...(workUnit.examples || []), ...exampleMapData.examples];
    imported.examples = exampleMapData.examples.length;
  }

  if (exampleMapData.questions && Array.isArray(exampleMapData.questions)) {
    workUnit.questions = [...(workUnit.questions || []), ...exampleMapData.questions];
    imported.questions = exampleMapData.questions.length;
  }

  if (exampleMapData.assumptions && Array.isArray(exampleMapData.assumptions)) {
    workUnit.assumptions = [...(workUnit.assumptions || []), ...exampleMapData.assumptions];
    imported.assumptions = exampleMapData.assumptions.length;
  }

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

  return {
    success: true,
    imported,
  };
}
