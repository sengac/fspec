import chalk from 'chalk';
import type { Command } from 'commander';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import type { WorkUnitsData, WorkUnit } from '../types';

interface OrphanedWorkUnit {
  id: string;
  title: string;
  status: string;
  suggestedActions: string[];
}

interface QueryOrphansOptions {
  cwd?: string;
  output?: 'json' | 'text';
  excludeDone?: boolean;
}

interface QueryOrphansResult {
  orphans: OrphanedWorkUnit[];
}

/**
 * Detect orphaned work units with no epic or dependencies
 *
 * Rules:
 * 1. An orphaned work unit has NO dependency relationships (no blocks, blockedBy, dependsOn, or relatesTo)
 * 2. An orphaned work unit has NO epic assignment (epic field is null or empty)
 * 3. A work unit must have either an epic OR at least one relationship to not be considered orphaned
 * 4. Include all statuses by default (even 'done' work can be orphaned)
 */
export async function queryOrphans(
  options: QueryOrphansOptions = {}
): Promise<QueryOrphansResult> {
  const cwd = options.cwd || process.cwd();
  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);
  const orphans: OrphanedWorkUnit[] = [];
  const workUnits = Object.values(data.workUnits);

  for (const wu of workUnits) {
    // Check if work unit has an epic
    const hasEpic = wu.epic && wu.epic.trim().length > 0;

    // Check if work unit has any relationships
    const hasRelationships =
      (wu.blocks && wu.blocks.length > 0) ||
      (wu.blockedBy && wu.blockedBy.length > 0) ||
      (wu.dependsOn && wu.dependsOn.length > 0) ||
      (wu.relatesTo && wu.relatesTo.length > 0);

    // A work unit is orphaned if it has NEITHER epic NOR relationships
    const isOrphaned = !hasEpic && !hasRelationships;

    if (!isOrphaned) {
      continue;
    }

    // Optional: exclude 'done' status if requested
    if (options.excludeDone && wu.status === 'done') {
      continue;
    }

    // Work unit is orphaned
    orphans.push({
      id: wu.id,
      title: wu.title,
      status: wu.status,
      suggestedActions: ['Assign epic', 'Add relationship', 'Delete'],
    });
  }

  return {
    orphans,
  };
}

export function registerQueryOrphansCommand(program: Command): void {
  program
    .command('query-orphans')
    .description('Detect orphaned work units with no epic or dependencies')
    .option(
      '--output <format>',
      'Output format: json or text (default: text)',
      'text'
    )
    .option('--exclude-done', 'Exclude work units in done status', false)
    .action(
      async (options: { output?: 'json' | 'text'; excludeDone?: boolean }) => {
        try {
          const result = await queryOrphans({
            output: options.output,
            excludeDone: options.excludeDone,
          });

          if (options.output === 'json') {
            console.log(JSON.stringify(result, null, 2));
          } else {
            // Text output
            if (result.orphans.length === 0) {
              console.log(chalk.green('✓ No orphaned work units found.'));
              console.log(
                chalk.dim(
                  'All work units have either an epic assignment or dependency relationships.'
                )
              );
              return;
            }

            console.log(
              chalk.yellow(
                `\nFound ${result.orphans.length} orphaned work unit(s):\n`
              )
            );

            result.orphans.forEach((orphan, index) => {
              console.log(
                `${index + 1}. ${chalk.cyan(orphan.id)} - ${orphan.title} (${chalk.dim(orphan.status)})`
              );
              console.log(
                `   ${chalk.red('⚠')} No epic or dependency relationships`
              );
              console.log(`   ${chalk.bold('Suggested actions:')}`);
              orphan.suggestedActions.forEach(action => {
                console.log(`     • ${action}`);
              });
              console.log('');
            });

            console.log(chalk.dim('To fix orphaned work units:'));
            console.log(
              chalk.dim('  fspec update-work-unit <id> --epic=<epic-name>')
            );
            console.log(
              chalk.dim(
                '  fspec add-dependency <id> --depends-on=<other-id>  (or --blocks, --relates-to)'
              )
            );
            console.log(chalk.dim('  fspec delete-work-unit <id>'));
          }
        } catch (error: any) {
          console.error(chalk.red('✗ Failed to query orphans:'), error.message);
          process.exit(1);
        }
      }
    );
}
