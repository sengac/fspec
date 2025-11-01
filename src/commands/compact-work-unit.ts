import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { fileManager } from '../utils/file-manager';

interface CompactWorkUnitOptions {
  workUnitId: string;
  force?: boolean;
  cwd?: string;
}

interface CompactWorkUnitResult {
  success: boolean;
  removedCounts: {
    rules: number;
    examples: number;
    questions: number;
    architectureNotes: number;
  };
  remainingCounts: {
    rules: number;
    examples: number;
    questions: number;
    architectureNotes: number;
  };
  warning?: string;
}

export async function compactWorkUnit(
  options: CompactWorkUnitOptions
): Promise<CompactWorkUnitResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  // Read work units (auto-creates file if missing)
  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);

  // Validate work unit exists
  if (!data.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = data.workUnits[options.workUnitId];

  // Require --force flag when not in done status
  let warning: string | undefined;
  if (workUnit.status !== 'done') {
    if (!options.force) {
      throw new Error(
        `Cannot compact work unit in '${workUnit.status}' status. Use --force to confirm compaction during active development.`
      );
    }
    // Set warning when force is used during non-done status
    warning = `Compacting during '${workUnit.status}' status permanently removes deleted items. Use with caution.`;
  }

  // Track counts before and after
  const removedCounts = {
    rules: 0,
    examples: 0,
    questions: 0,
    architectureNotes: 0,
  };

  const remainingCounts = {
    rules: 0,
    examples: 0,
    questions: 0,
    architectureNotes: 0,
  };

  // Compact rules (remove deleted and renumber IDs)
  if (workUnit.rules && workUnit.rules.length > 0) {
    const originalLength = workUnit.rules.length;
    workUnit.rules = workUnit.rules.filter(r => !r.deleted);
    // Renumber IDs sequentially starting from 0
    workUnit.rules.forEach((rule, index) => {
      rule.id = index;
    });
    removedCounts.rules = originalLength - workUnit.rules.length;
    remainingCounts.rules = workUnit.rules.length;
  }

  // Compact examples (remove deleted and renumber IDs)
  if (workUnit.examples && workUnit.examples.length > 0) {
    const originalLength = workUnit.examples.length;
    workUnit.examples = workUnit.examples.filter(e => !e.deleted);
    // Renumber IDs sequentially starting from 0
    workUnit.examples.forEach((example, index) => {
      example.id = index;
    });
    removedCounts.examples = originalLength - workUnit.examples.length;
    remainingCounts.examples = workUnit.examples.length;
  }

  // Compact questions (remove deleted and renumber IDs)
  if (workUnit.questions && workUnit.questions.length > 0) {
    const originalLength = workUnit.questions.length;
    workUnit.questions = workUnit.questions.filter(q => !q.deleted);
    // Renumber IDs sequentially starting from 0
    workUnit.questions.forEach((question, index) => {
      question.id = index;
    });
    removedCounts.questions = originalLength - workUnit.questions.length;
    remainingCounts.questions = workUnit.questions.length;
  }

  // Compact architecture notes (remove deleted and renumber IDs)
  if (workUnit.architectureNotes && workUnit.architectureNotes.length > 0) {
    const originalLength = workUnit.architectureNotes.length;
    workUnit.architectureNotes = workUnit.architectureNotes.filter(
      n => !n.deleted
    );
    // Renumber IDs sequentially starting from 0
    workUnit.architectureNotes.forEach((note, index) => {
      note.id = index;
    });
    removedCounts.architectureNotes =
      originalLength - workUnit.architectureNotes.length;
    remainingCounts.architectureNotes = workUnit.architectureNotes.length;
  }

  // Reset nextId counters to match the new array lengths
  if (workUnit.rules) {
    workUnit.nextRuleId = workUnit.rules.length;
  }
  if (workUnit.examples) {
    workUnit.nextExampleId = workUnit.examples.length;
  }
  if (workUnit.questions) {
    workUnit.nextQuestionId = workUnit.questions.length;
  }
  if (workUnit.architectureNotes) {
    workUnit.nextArchitectureNoteId = workUnit.architectureNotes.length;
  }

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // Update metadata
  if (data.meta) {
    data.meta.lastUpdated = new Date().toISOString();
  }

  // LOCK-002: Use fileManager.transaction() for atomic write
  await fileManager.transaction(workUnitsFile, async fileData => {
    Object.assign(fileData, data);
  });

  return {
    success: true,
    removedCounts,
    remainingCounts,
    ...(warning && { warning }),
  };
}

export function registerCompactWorkUnitCommand(program: Command): void {
  program
    .command('compact-work-unit')
    .description('Permanently remove all soft-deleted items from a work unit')
    .argument('<workUnitId>', 'Work unit ID')
    .action(async (workUnitId: string) => {
      try {
        const result = await compactWorkUnit({ workUnitId });

        const totalRemoved =
          result.removedCounts.rules +
          result.removedCounts.examples +
          result.removedCounts.questions +
          result.removedCounts.architectureNotes;

        if (totalRemoved === 0) {
          console.log(chalk.dim('No deleted items to remove'));
        } else {
          console.log(chalk.green(`✓ Compacted work unit ${workUnitId}`));
          console.log(chalk.dim('  Removed items:'));
          if (result.removedCounts.rules > 0) {
            console.log(chalk.dim(`    Rules: ${result.removedCounts.rules}`));
          }
          if (result.removedCounts.examples > 0) {
            console.log(
              chalk.dim(`    Examples: ${result.removedCounts.examples}`)
            );
          }
          if (result.removedCounts.questions > 0) {
            console.log(
              chalk.dim(`    Questions: ${result.removedCounts.questions}`)
            );
          }
          if (result.removedCounts.architectureNotes > 0) {
            console.log(
              chalk.dim(
                `    Architecture Notes: ${result.removedCounts.architectureNotes}`
              )
            );
          }
        }
      } catch (error: any) {
        console.error(
          chalk.red('✗ Failed to compact work unit:'),
          error.message
        );
        process.exit(1);
      }
    });
}
