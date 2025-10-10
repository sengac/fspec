import { readFile, writeFile, mkdir } from 'fs/promises';
import { join, dirname } from 'path';
import type { WorkUnitsData } from '../types';

interface ExportExampleMapOptions {
  workUnitId: string;
  output: string;
  cwd?: string;
}

interface ExportExampleMapResult {
  success: boolean;
  outputFile: string;
  exported: {
    rules: number;
    examples: number;
    questions: number;
    assumptions: number;
  };
}

export async function exportExampleMap(
  options: ExportExampleMapOptions
): Promise<ExportExampleMapResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec/work-units.json');
  const outputFilePath = options.output;

  // Read work units
  const content = await readFile(workUnitsFile, 'utf-8');
  const data: WorkUnitsData = JSON.parse(content);

  // Validate work unit exists
  if (!data.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = data.workUnits[options.workUnitId];

  // Build export data
  const exportData = {
    workUnitId: options.workUnitId,
    title: workUnit.title,
    rules: workUnit.rules || [],
    examples: workUnit.examples || [],
    questions: workUnit.questions || [],
    assumptions: workUnit.assumptions || [],
  };

  const exported = {
    rules: exportData.rules.length,
    examples: exportData.examples.length,
    questions: exportData.questions.length,
    assumptions: exportData.assumptions.length,
  };

  // Ensure output directory exists
  await mkdir(dirname(outputFilePath), { recursive: true });

  // Write JSON file
  await writeFile(outputFilePath, JSON.stringify(exportData, null, 2));

  return {
    success: true,
    outputFile: outputFilePath,
    exported,
  };
}
