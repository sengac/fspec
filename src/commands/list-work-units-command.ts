import chalk from 'chalk';
import { listWorkUnits } from './list-work-units';

export async function listWorkUnitsCommand(options: {
  status?: string;
  prefix?: string;
  epic?: string;
}): Promise<void> {
  try {
    const result = await listWorkUnits({
      status: options.status,
      prefix: options.prefix,
      epic: options.epic,
    });

    if (result.workUnits.length === 0) {
      console.log(chalk.yellow('No work units found'));
      process.exit(0);
    }

    console.log(chalk.bold(`\nWork Units (${result.workUnits.length})`));
    console.log('');

    for (const wu of result.workUnits) {
      console.log(chalk.cyan(wu.id) + chalk.gray(` [${wu.status}]`));
      console.log(`  ${wu.title}`);
      if (wu.epic) {
        console.log(chalk.gray(`  Epic: ${wu.epic}`));
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
