import { readFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import { glob } from 'tinyglobby';

interface WorkUnit {
  id: string;
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

export async function validateSpecAlignment(options: {
  workUnitId: string;
  cwd?: string;
}): Promise<{ valid: boolean; warnings?: string[] }> {
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

    // Find all feature files
    const featureFiles = await glob(['spec/features/**/*.feature'], {
      cwd,
      absolute: false,
    });

    // Search for scenarios tagged with work unit ID
    const workUnitTag = `@${options.workUnitId}`;
    let scenariosFound = 0;

    for (const file of featureFiles) {
      const filePath = join(cwd, file);
      const fileContent = await readFile(filePath, 'utf-8');

      // Count scenarios with work unit tag
      const lines = fileContent.split('\n');
      for (let i = 0; i < lines.length; i++) {
        const line = lines[i].trim();
        if (line.includes(workUnitTag) && i + 1 < lines.length) {
          const nextLine = lines[i + 1].trim();
          if (nextLine.startsWith('Scenario:')) {
            scenariosFound++;
          }
        }
      }
    }

    if (scenariosFound === 0) {
      return {
        valid: false,
        warnings: [`No scenarios for ${options.workUnitId}`],
      };
    }

    return { valid: true };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to validate spec alignment: ${error.message}`);
    }
    throw error;
  }
}

export function registerValidateSpecAlignmentCommand(program: Command): void {
  program
    .command('validate-spec-alignment')
    .description('Validate alignment between specs, tests, and implementation')
    .option('--fix', 'Attempt to fix alignment issues')
    .action(async (options: { fix?: boolean }) => {
      try {
        const result = await validateSpecAlignment({ fix: options.fix });
        if (result.aligned) {
          console.log(
            chalk.green(`✓ All specs are aligned with tests and implementation`)
          );
        } else {
          console.error(
            chalk.red(`✗ Found ${result.issues.length} alignment issues`)
          );
          result.issues.forEach((issue: string) =>
            console.error(chalk.red(`  - ${issue}`))
          );
          process.exit(1);
        }
      } catch (error: any) {
        console.error(chalk.red('✗ Validation failed:'), error.message);
        process.exit(1);
      }
    });
}
