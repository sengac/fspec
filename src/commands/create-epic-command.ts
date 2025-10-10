import chalk from 'chalk';
import { createEpic } from './create-epic';

export async function createEpicCommand(
  epicId: string,
  title: string,
  options: { description?: string }
): Promise<void> {
  try {
    const result = await createEpic({
      epicId,
      title,
      description: options.description,
    });

    if (result.success) {
      console.log(chalk.green(`✓ Created epic ${epicId}`));
      console.log(chalk.gray(`  Title: ${title}`));
      if (options.description) {
        console.log(chalk.gray(`  Description: ${options.description}`));
      }
      process.exit(0);
    } else {
      console.error(chalk.red('✗ Failed to create epic'));
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
