import chalk from 'chalk';
import { createWorkUnit } from './create-work-unit';

export async function createWorkUnitCommand(
  prefix: string,
  title: string,
  options: { description?: string; epic?: string; parent?: string }
): Promise<void> {
  try {
    const result = await createWorkUnit({
      prefix,
      title,
      description: options.description,
      epic: options.epic,
      parent: options.parent,
    });

    if (result.success && result.workUnitId) {
      console.log(chalk.green(`✓ Created work unit ${result.workUnitId}`));
      console.log(chalk.gray(`  Title: ${title}`));
      if (options.description) {
        console.log(chalk.gray(`  Description: ${options.description}`));
      }
      if (options.epic) {
        console.log(chalk.gray(`  Epic: ${options.epic}`));
      }
      if (options.parent) {
        console.log(chalk.gray(`  Parent: ${options.parent}`));
      }
      process.exit(0);
    } else {
      console.error(chalk.red('✗ Failed to create work unit'));
      process.exit(1);
    }
  } catch (error: unknown) {
    if (error instanceof Error) {
      console.error(chalk.red('Error:'), error.message);
    } else {
      console.error(chalk.red('Error: Unknown error occurred'));
    }
    process.exit(1);
  }
}
