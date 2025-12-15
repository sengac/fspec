import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData, ExampleItem } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { fileManager } from '../utils/file-manager';
import { wrapInSystemReminder } from '../utils/system-reminder';

interface AddExampleOptions {
  workUnitId: string;
  example: string;
  cwd?: string;
}

interface AddExampleResult {
  success: boolean;
  exampleCount: number;
  systemReminder?: string;
}

export async function addExample(
  options: AddExampleOptions
): Promise<AddExampleResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec/work-units.json');

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
      `Can only add examples during discovery/specification phase. ${options.workUnitId} is in '${workUnit.status}' state.`
    );
  }

  // Initialize examples array if it doesn't exist
  if (!workUnit.examples) {
    workUnit.examples = [];
  }

  // Initialize nextExampleId if undefined (backward compatibility)
  if (workUnit.nextExampleId === undefined) {
    workUnit.nextExampleId = 0;
  }

  // Create ExampleItem object with stable ID
  const newExample: ExampleItem = {
    id: workUnit.nextExampleId++,
    text: options.example,
    deleted: false,
    createdAt: new Date().toISOString(),
  };

  // Add example
  workUnit.examples.push(newExample);

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // LOCK-002: Use fileManager.transaction() for atomic write
  await fileManager.transaction(workUnitsFile, async fileData => {
    Object.assign(fileData, data);
  });

  // Generate system reminder for example quality check
  const role = workUnit.userStory?.role || 'the user';
  const systemReminder = wrapInSystemReminder(`EXAMPLE CHECK

User story: "As a ${role}..."
Example: "${options.example}"

Does this example describe what ${role} experiences?

  ✓ GOOD: Describes the user's experience from their perspective
  ✗ BAD: Describes internal component/module behavior

If your example mentions implementation details (widgets, modules, internal state),
rewrite it to describe what ${role} sees and does.

DO NOT mention this reminder to the user.`);

  return {
    success: true,
    exampleCount: workUnit.examples.length,
    systemReminder,
  };
}

export function registerAddExampleCommand(program: Command): void {
  program
    .command('add-example')
    .description('Add an example to a work unit during specification phase')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<example>', 'Example description')
    .action(async (workUnitId: string, example: string) => {
      try {
        const result = await addExample({ workUnitId, example });
        console.log(chalk.green(`✓ Example added successfully`));
        if (result.systemReminder) {
          console.log('\n' + result.systemReminder);
        }
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to add example:'), error.message);
        process.exit(1);
      }
    });
}
