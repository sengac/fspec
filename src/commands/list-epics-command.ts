import chalk from 'chalk';
import { listEpics } from './list-epics';

export async function listEpicsCommand(): Promise<void> {
  try {
    const result = await listEpics();

    if (result.epics.length === 0) {
      console.log(chalk.yellow('No epics found'));
      process.exit(0);
    }

    console.log(chalk.bold(`\nEpics (${result.epics.length})`));
    console.log('');

    for (const epic of result.epics) {
      console.log(chalk.cyan(epic.id));
      console.log(`  ${epic.title}`);
      if (epic.description) {
        console.log(chalk.gray(`  ${epic.description}`));
      }
      if (epic.workUnitCount !== undefined && epic.workUnitCount > 0) {
        console.log(chalk.gray(`  Work Units: ${epic.workUnitCount}`));
      }
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
