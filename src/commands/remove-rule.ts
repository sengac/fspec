import { writeFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

interface RemoveRuleOptions {
  workUnitId: string;
  index: number;
  cwd?: string;
}

interface RemoveRuleResult {
  success: boolean;
  removedRule: string;
  remainingCount: number;
}

export async function removeRule(
  options: RemoveRuleOptions
): Promise<RemoveRuleResult> {
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
      `Can only remove rules during discovery/specification phase. ${options.workUnitId} is in '${workUnit.status}' state.`
    );
  }

  // Validate rules array exists
  if (!workUnit.rules || workUnit.rules.length === 0) {
    throw new Error(`Work unit ${options.workUnitId} has no rules`);
  }

  // Validate index
  if (options.index < 0 || options.index >= workUnit.rules.length) {
    throw new Error(
      `Index ${options.index} out of range. Valid indices: 0-${workUnit.rules.length - 1}`
    );
  }

  // Remove rule
  const [removedRule] = workUnit.rules.splice(options.index, 1);

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

  return {
    success: true,
    removedRule,
    remainingCount: workUnit.rules.length,
  };
}

export function registerRemoveRuleCommand(program: Command): void {
  program
    .command('remove-rule')
    .description('Remove a business rule from a work unit by index')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<index>', 'Rule index (0-based)')
    .action(async (workUnitId: string, index: string) => {
      try {
        const result = await removeRule({
          workUnitId,
          index: parseInt(index, 10),
        });
        console.log(chalk.green(`✓ Removed rule: "${result.removedRule}"`));
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to remove rule:'), error.message);
        process.exit(1);
      }
    });
}
