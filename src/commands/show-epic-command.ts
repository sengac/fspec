import chalk from 'chalk';
import { showEpic } from './show-epic';

export async function showEpicCommand(
  epicId: string,
  options: { format?: string }
): Promise<void> {
  try {
    const result = await showEpic({
      epicId,
      output: (options.format as 'json' | 'text') || 'text',
    });

    if (options.format === 'json') {
      console.log(JSON.stringify(result, null, 2));
    } else {
      console.log(chalk.bold(`\nEpic: ${result.id}`));
      console.log('');
      console.log(chalk.cyan('Title:'), result.title);

      if (result.description) {
        console.log(chalk.cyan('Description:'), result.description);
      }

      if (result.workUnits && result.workUnits.length > 0) {
        console.log(chalk.cyan('\nWork Units:'));
        result.workUnits.forEach((wu) => {
          console.log(`  - ${wu}`);
        });
      }

      console.log('');
      console.log(chalk.gray('Created:'), new Date(result.createdAt).toLocaleString());
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
