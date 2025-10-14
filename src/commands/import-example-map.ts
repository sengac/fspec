import { readFile, writeFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

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
  rulesCount: number;
  examplesCount: number;
  questionsCount: number;
  assumptionsCount: number;
}

export async function importExampleMap(
  options: ImportExampleMapOptions
): Promise<ImportExampleMapResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec/work-units.json');
  const jsonFilePath = options.file;

  // Read work units (auto-creates file if missing)
  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);

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
    workUnit.examples = [
      ...(workUnit.examples || []),
      ...exampleMapData.examples,
    ];
    imported.examples = exampleMapData.examples.length;
  }

  if (exampleMapData.questions && Array.isArray(exampleMapData.questions)) {
    workUnit.questions = [
      ...(workUnit.questions || []),
      ...exampleMapData.questions,
    ];
    imported.questions = exampleMapData.questions.length;
  }

  if (exampleMapData.assumptions && Array.isArray(exampleMapData.assumptions)) {
    workUnit.assumptions = [
      ...(workUnit.assumptions || []),
      ...exampleMapData.assumptions,
    ];
    imported.assumptions = exampleMapData.assumptions.length;
  }

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

  return {
    success: true,
    rulesCount: imported.rules,
    examplesCount: imported.examples,
    questionsCount: imported.questions,
    assumptionsCount: imported.assumptions,
  };
}

export function registerImportExampleMapCommand(program: Command): void {
  program
    .command('import-example-map')
    .description('Import example mapping data from JSON file to work unit')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<file>', 'Input JSON file path')
    .action(async (workUnitId: string, file: string) => {
      try {
        const result = await importExampleMap({ workUnitId, file });
        const total =
          result.rulesCount +
          result.examplesCount +
          result.questionsCount +
          result.assumptionsCount;
        console.log(
          chalk.green(
            `✓ Imported ${total} items: ${result.rulesCount} rules, ${result.examplesCount} examples, ${result.questionsCount} questions, ${result.assumptionsCount} assumptions`
          )
        );
      } catch (error: any) {
        console.error(
          chalk.red('✗ Failed to import example map:'),
          error.message
        );
        process.exit(1);
      }
    });
}
