import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { fileManager } from '../utils/file-manager';

interface RemoveRuleOptions {
  workUnitId: string;
  index: number;
  cwd?: string;
}

interface RemoveRuleResult {
  success: boolean;
  removedRule: string;
  remainingCount: number;
  message?: string; // For idempotent operations
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

  // Find rule by ID (index is now treated as ID for stable indices)
  const rule = workUnit.rules.find(r => r.id === options.index);

  if (!rule) {
    throw new Error(`Rule with ID ${options.index} not found`);
  }

  // If already deleted, return idempotent success
  if (rule.deleted) {
    return {
      success: true,
      removedRule: rule.text,
      remainingCount: workUnit.rules.filter(r => !r.deleted).length,
      message: `Item ID ${options.index} already deleted`,
    };
  }

  // Soft-delete: set deleted flag and timestamp
  rule.deleted = true;
  rule.deletedAt = new Date().toISOString();

  const removedRule = rule.text;

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // LOCK-002: Use fileManager.transaction() for atomic write
  await fileManager.transaction(workUnitsFile, async fileData => {
    Object.assign(fileData, data);
  });

  return {
    success: true,
    removedRule,
    remainingCount: workUnit.rules.filter(r => !r.deleted).length,
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
