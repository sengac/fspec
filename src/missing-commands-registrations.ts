// This file contains all the missing command registrations
// This is temporary - will be merged into index.ts

import chalk from 'chalk';
import { Command } from 'commander';

// Import all missing commands
import { prioritizeWorkUnit } from './commands/prioritize-work-unit';
import { updateWorkUnit } from './commands/update-work-unit';
import { deleteWorkUnit } from './commands/delete-work-unit';
import { updateWorkUnitStatus } from './commands/update-work-unit-status';
import { updateWorkUnitEstimate } from './commands/update-work-unit-estimate';
import { addDependency } from './commands/add-dependency';
import { addDependencies } from './commands/add-dependencies';
import { removeDependency } from './commands/remove-dependency';
import { clearDependencies } from './commands/clear-dependencies';
import { exportDependencies } from './commands/export-dependencies';

export function registerMissingCommands(program: Command): void {
  // ============================================================================
  // WORK UNIT MANAGEMENT COMMANDS
  // ============================================================================

  // Prioritize work unit command
  program
    .command('prioritize-work-unit')
    .description('Change the priority order of a work unit in the backlog')
    .argument('<workUnitId>', 'Work unit ID to prioritize')
    .option('--position <position>', 'Position: top, bottom, or numeric index')
    .option('--before <workUnitId>', 'Place before this work unit')
    .option('--after <workUnitId>', 'Place after this work unit')
    .action(async (workUnitId: string, options: { position?: string; before?: string; after?: string }) => {
      try {
        const parsedPosition = options.position === 'top' ? 'top'
          : options.position === 'bottom' ? 'bottom'
          : options.position ? parseInt(options.position, 10)
          : undefined;

        await prioritizeWorkUnit({
          workUnitId,
          position: parsedPosition as 'top' | 'bottom' | number | undefined,
          before: options.before,
          after: options.after,
        });
        console.log(chalk.green(`✓ Work unit ${workUnitId} prioritized successfully`));
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to prioritize work unit:'), error.message);
        process.exit(1);
      }
    });

  // Update work unit command
  program
    .command('update-work-unit')
    .description('Update work unit properties')
    .argument('<workUnitId>', 'Work unit ID to update')
    .option('-t, --title <title>', 'New title')
    .option('-d, --description <description>', 'New description')
    .option('-e, --epic <epic>', 'Epic ID')
    .option('-p, --parent <parent>', 'Parent work unit ID')
    .action(async (workUnitId: string, options: { title?: string; description?: string; epic?: string; parent?: string }) => {
      try {
        await updateWorkUnit({
          workUnitId,
          ...options,
        });
        console.log(chalk.green(`✓ Work unit ${workUnitId} updated successfully`));
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to update work unit:'), error.message);
        process.exit(1);
      }
    });

  // Delete work unit command
  program
    .command('delete-work-unit')
    .description('Delete a work unit')
    .argument('<workUnitId>', 'Work unit ID to delete')
    .option('--force', 'Force deletion without checks')
    .option('--skip-confirmation', 'Skip confirmation prompt')
    .option('--cascade-dependencies', 'Remove all dependencies before deleting')
    .action(async (workUnitId: string, options: { force?: boolean; skipConfirmation?: boolean; cascadeDependencies?: boolean }) => {
      try {
        const result = await deleteWorkUnit({
          workUnitId,
          ...options,
        });
        console.log(chalk.green(`✓ Work unit ${workUnitId} deleted successfully`));
        if (result.warnings && result.warnings.length > 0) {
          result.warnings.forEach((warning: string) => console.log(chalk.yellow(`⚠ ${warning}`)));
        }
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to delete work unit:'), error.message);
        process.exit(1);
      }
    });

  // Update work unit status command
  program
    .command('update-work-unit-status')
    .description('Update work unit status (follows ACDD workflow)')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<status>', 'New status: backlog, specifying, testing, implementing, validating, done, blocked')
    .option('--blocked-reason <reason>', 'Reason for blocked status (required if status is blocked)')
    .option('--reason <reason>', 'Reason for status change')
    .action(async (workUnitId: string, status: string, options: { blockedReason?: string; reason?: string }) => {
      try {
        const result = await updateWorkUnitStatus({
          workUnitId,
          status: status as 'backlog' | 'specifying' | 'testing' | 'implementing' | 'validating' | 'done' | 'blocked',
          ...options,
        });
        console.log(chalk.green(`✓ Work unit ${workUnitId} status updated to ${status}`));
        if (result.warnings && result.warnings.length > 0) {
          result.warnings.forEach((warning: string) => console.log(chalk.yellow(`⚠ ${warning}`)));
        }
        // Output system reminder (visible to AI, invisible to users)
        if (result.systemReminder) {
          console.log('\n' + result.systemReminder);
        }
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to update work unit status:'), error.message);
        process.exit(1);
      }
    });

  // Update work unit estimate command
  program
    .command('update-work-unit-estimate')
    .description('Update work unit estimate (Fibonacci: 1,2,3,5,8,13,21)')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<estimate>', 'Story points estimate (Fibonacci number)')
    .action(async (workUnitId: string, estimate: string) => {
      try {
        await updateWorkUnitEstimate({
          workUnitId,
          estimate: parseInt(estimate, 10),
        });
        console.log(chalk.green(`✓ Work unit ${workUnitId} estimate set to ${estimate}`));
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to update estimate:'), error.message);
        process.exit(1);
      }
    });

  // ============================================================================
  // DEPENDENCY MANAGEMENT COMMANDS
  // ============================================================================

  // Add dependency command
  program
    .command('add-dependency')
    .description('Add a dependency relationship between work units')
    .argument('<workUnitId>', 'Work unit ID')
    .option('--blocks <targetId>', 'Work unit that this blocks')
    .option('--blocked-by <targetId>', 'Work unit that blocks this')
    .option('--depends-on <targetId>', 'Work unit this depends on (soft dependency)')
    .option('--relates-to <targetId>', 'Related work unit')
    .action(async (workUnitId: string, options: { blocks?: string; blockedBy?: string; dependsOn?: string; relatesTo?: string }) => {
      try {
        await addDependency({
          workUnitId,
          ...options,
        });
        console.log(chalk.green(`✓ Dependency added successfully`));
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to add dependency:'), error.message);
        process.exit(1);
      }
    });

  // Add dependencies (plural) command
  program
    .command('add-dependencies')
    .description('Add multiple dependency relationships at once')
    .argument('<workUnitId>', 'Work unit ID')
    .option('--blocks <ids...>', 'Work unit IDs that this blocks')
    .option('--blocked-by <ids...>', 'Work unit IDs that block this')
    .option('--depends-on <ids...>', 'Work unit IDs this depends on')
    .option('--relates-to <ids...>', 'Related work unit IDs')
    .action(async (workUnitId: string, options: { blocks?: string[]; blockedBy?: string[]; dependsOn?: string[]; relatesTo?: string[] }) => {
      try {
        const result = await addDependencies({
          workUnitId,
          dependencies: {
            blocks: options.blocks,
            blockedBy: options.blockedBy,
            dependsOn: options.dependsOn,
            relatesTo: options.relatesTo,
          },
        });
        console.log(chalk.green(`✓ Added ${result.added} dependencies successfully`));
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to add dependencies:'), error.message);
        process.exit(1);
      }
    });

  // Remove dependency command
  program
    .command('remove-dependency')
    .description('Remove a dependency relationship between work units')
    .argument('<workUnitId>', 'Work unit ID')
    .option('--blocks <targetId>', 'Remove blocks relationship')
    .option('--blocked-by <targetId>', 'Remove blockedBy relationship')
    .option('--depends-on <targetId>', 'Remove dependsOn relationship')
    .option('--relates-to <targetId>', 'Remove relatesTo relationship')
    .action(async (workUnitId: string, options: { blocks?: string; blockedBy?: string; dependsOn?: string; relatesTo?: string }) => {
      try {
        await removeDependency({
          workUnitId,
          ...options,
        });
        console.log(chalk.green(`✓ Dependency removed successfully`));
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to remove dependency:'), error.message);
        process.exit(1);
      }
    });

  // Clear dependencies command
  program
    .command('clear-dependencies')
    .description('Remove all dependencies from a work unit')
    .argument('<workUnitId>', 'Work unit ID')
    .option('--confirm', 'Confirm clearing all dependencies')
    .action(async (workUnitId: string, options: { confirm?: boolean }) => {
      try {
        await clearDependencies({
          workUnitId,
          ...options,
        });
        console.log(chalk.green(`✓ All dependencies cleared from ${workUnitId}`));
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to clear dependencies:'), error.message);
        process.exit(1);
      }
    });

  // Export dependencies command
  program
    .command('export-dependencies')
    .description('Export dependency graph visualization')
    .argument('<format>', 'Output format: mermaid or json')
    .argument('<output>', 'Output file path')
    .action(async (format: string, output: string) => {
      try {
        const result = await exportDependencies({
          format: format as 'mermaid' | 'json',
          output,
        });
        console.log(chalk.green(`✓ Dependencies exported to ${result.outputFile}`));
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to export dependencies:'), error.message);
        process.exit(1);
      }
    });
}
