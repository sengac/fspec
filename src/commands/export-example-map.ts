import { writeFile, mkdir } from 'fs/promises';
import type { Command } from 'commander';
import { join, dirname } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

interface ExportExampleMapOptions {
  workUnitId: string;
  file: string;
  cwd?: string;
}

interface ExportExampleMapResult {
  success: boolean;
  outputFile: string;
  rulesCount: number;
  examplesCount: number;
  questionsCount: number;
  assumptionsCount: number;
}

export async function exportExampleMap(
  options: ExportExampleMapOptions
): Promise<ExportExampleMapResult> {
  const cwd = options.cwd || process.cwd();
  const outputFilePath = options.file;

  // Read work units (auto-creates file if missing)
  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);

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
    rulesCount: exported.rules,
    examplesCount: exported.examples,
    questionsCount: exported.questions,
    assumptionsCount: exported.assumptions,
  };
}

export function registerExportExampleMapCommand(program: Command): void {
  program
    .command('export-example-map')
    .description('Export example mapping data from work unit to JSON file')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<file>', 'Output JSON file path')
    .action(async (workUnitId: string, file: string) => {
      try {
        const result = await exportExampleMap({ workUnitId, file });
        console.log(chalk.green(`✓ Exported to ${result.outputFile}`));
      } catch (error: any) {
        console.error(
          chalk.red('✗ Failed to export example map:'),
          error.message
        );
        process.exit(1);
      }
    });
}
