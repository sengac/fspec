import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { fileManager } from '../utils/file-manager';

interface RestoreRuleOptions {
  workUnitId: string;
  index?: number;
  ids?: string; // Comma-separated IDs for bulk restore
  cwd?: string;
}

interface RestoreRuleResult {
  success: boolean;
  restoredRule: string;
  activeCount: number;
  restoredCount?: number; // For bulk operations
  message?: string; // For idempotent operations
}

export async function restoreRule(
  options: RestoreRuleOptions
): Promise<RestoreRuleResult> {
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
      `Can only restore rules during discovery/specification phase. ${options.workUnitId} is in '${workUnit.status}' state.`
    );
  }

  // Validate rules array exists
  if (!workUnit.rules || workUnit.rules.length === 0) {
    throw new Error(`Work unit ${options.workUnitId} has no rules`);
  }

  // Handle bulk restore if ids parameter is provided
  if (options.ids) {
    const indices = options.ids.split(',').map(id => parseInt(id.trim(), 10));
    const restoredRules: string[] = [];

    for (const index of indices) {
      const rule = workUnit.rules.find(r => r.id === index);
      if (!rule) {
        throw new Error(`Rule with ID ${index} not found`);
      }
      if (!rule.deleted) {
        continue; // Skip already active items
      }
      rule.deleted = false;
      delete rule.deletedAt;
      restoredRules.push(rule.text);
    }

    workUnit.updatedAt = new Date().toISOString();

    await fileManager.transaction(workUnitsFile, async fileData => {
      Object.assign(fileData, data);
    });

    return {
      success: true,
      restoredRule: restoredRules.join(', '),
      activeCount: workUnit.rules.filter(r => !r.deleted).length,
      restoredCount: restoredRules.length,
    };
  }

  // Single restore: Find rule by ID (index is now treated as ID for stable indices)
  const rule = workUnit.rules.find(r => r.id === options.index);

  if (!rule) {
    throw new Error(`Rule with ID ${options.index} not found`);
  }

  // If already active, return idempotent success
  if (!rule.deleted) {
    return {
      success: true,
      restoredRule: rule.text,
      activeCount: workUnit.rules.filter(r => !r.deleted).length,
      message: `Item ID ${options.index} already active`,
    };
  }

  // Restore: clear deleted flag and timestamp
  rule.deleted = false;
  delete rule.deletedAt;

  const restoredRule = rule.text;

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // LOCK-002: Use fileManager.transaction() for atomic write
  await fileManager.transaction(workUnitsFile, async fileData => {
    Object.assign(fileData, data);
  });

  return {
    success: true,
    restoredRule,
    activeCount: workUnit.rules.filter(r => !r.deleted).length,
  };
}

export function registerRestoreRuleCommand(program: Command): void {
  program
    .command('restore-rule')
    .description('Restore a soft-deleted business rule by ID')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<index>', 'Rule ID (0-based)')
    .action(async (workUnitId: string, index: string) => {
      try {
        const result = await restoreRule({
          workUnitId,
          index: parseInt(index, 10),
        });
        console.log(chalk.green(`✓ Restored rule: "${result.restoredRule}"`));
        if (result.message) {
          console.log(chalk.dim(`  ${result.message}`));
        }
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to restore rule:'), error.message);
        process.exit(1);
      }
    });
}
