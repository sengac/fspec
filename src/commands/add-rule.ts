import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { fileManager } from '../utils/file-manager';

interface AddRuleOptions {
  workUnitId: string;
  rule: string;
  cwd?: string;
}

interface AddRuleResult {
  success: boolean;
  ruleCount: number;
}

export async function addRule(options: AddRuleOptions): Promise<AddRuleResult> {
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
      `Can only add rules during discovery/specification phase. ${options.workUnitId} is in '${workUnit.status}' state.`
    );
  }

  // Initialize rules array if it doesn't exist
  if (!workUnit.rules) {
    workUnit.rules = [];
  }

  // Add rule
  workUnit.rules.push(options.rule);

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // LOCK-002: Use fileManager.transaction() for atomic write
  await fileManager.transaction(workUnitsFile, async fileData => {
    Object.assign(fileData, data);
  });

  return {
    success: true,
    ruleCount: workUnit.rules.length,
  };
}

export function registerAddRuleCommand(program: Command): void {
  program
    .command('add-rule')
    .description(
      'Add a business rule to a work unit during specification phase'
    )
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<rule>', 'Business rule description')
    .action(async (workUnitId: string, rule: string) => {
      try {
        await addRule({ workUnitId, rule });
        console.log(chalk.green(`✓ Rule added successfully`));
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to add rule:'), error.message);
        process.exit(1);
      }
    });
}
