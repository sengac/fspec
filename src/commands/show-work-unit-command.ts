import chalk from 'chalk';
import { showWorkUnit } from './show-work-unit';

export async function showWorkUnitCommand(
  workUnitId: string,
  options: { format?: string }
): Promise<void> {
  try {
    const result = await showWorkUnit({
      workUnitId,
      output: (options.format as 'json' | 'text') || 'text',
    });

    if (options.format === 'json') {
      console.log(JSON.stringify(result, null, 2));
    } else {
      console.log(chalk.bold(`\nWork Unit: ${result.id}`));
      console.log('');
      console.log(chalk.cyan('Title:'), result.title);
      console.log(chalk.cyan('Status:'), result.status);

      if (result.description) {
        console.log(chalk.cyan('Description:'), result.description);
      }

      if (result.estimate !== undefined) {
        console.log(chalk.cyan('Estimate:'), result.estimate, 'points');
      }

      if (result.epic) {
        console.log(chalk.cyan('Epic:'), result.epic);
      }

      if (result.parent) {
        console.log(chalk.cyan('Parent:'), result.parent);
      }

      if (result.children && result.children.length > 0) {
        console.log(chalk.cyan('Children:'), result.children.join(', '));
      }

      if (result.blockedBy && result.blockedBy.length > 0) {
        console.log(chalk.cyan('Blocked By:'), result.blockedBy.join(', '));
      }

      if (result.rules && result.rules.length > 0) {
        console.log(chalk.cyan('\nRules:'));
        result.rules.forEach((rule, idx) => {
          console.log(`  ${idx + 1}. ${rule}`);
        });
      }

      if (result.examples && result.examples.length > 0) {
        console.log(chalk.cyan('\nExamples:'));
        result.examples.forEach((example, idx) => {
          console.log(`  ${idx + 1}. ${example}`);
        });
      }

      if (result.questions && result.questions.length > 0) {
        console.log(chalk.cyan('\nQuestions:'));
        result.questions.forEach((question, idx) => {
          console.log(`  ${idx + 1}. ${question}`);
        });
      }

      if (result.assumptions && result.assumptions.length > 0) {
        console.log(chalk.cyan('\nAssumptions:'));
        result.assumptions.forEach((assumption, idx) => {
          console.log(`  ${idx + 1}. ${assumption}`);
        });
      }

      console.log('');
      console.log(chalk.gray('Created:'), new Date(result.createdAt).toLocaleString());
      console.log(chalk.gray('Updated:'), new Date(result.updatedAt).toLocaleString());
      console.log('');
    }

    process.exit(0);
  } catch (error: unknown) {
    if (error instanceof Error) {
      console.error(chalk.red('Error:'), error.message);
    } else {
      console.error(chalk.red('Error: Unknown error occurred'));
    }
    process.exit(1);
  }
}
