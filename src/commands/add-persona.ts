import { promises as fs } from 'fs';
import { join } from 'path';
import chalk from 'chalk';
import type { GenericFoundation } from '../types/foundation';

export async function addPersona(
  cwd: string,
  name: string,
  description: string,
  goals: string[],
): Promise<void> {
  const foundationPath = join(cwd, 'spec', 'foundation.json');

  // Read existing foundation.json
  let foundation: GenericFoundation;
  try {
    const content = await fs.readFile(foundationPath, 'utf-8');
    foundation = JSON.parse(content);
  } catch (error: unknown) {
    const err = error as NodeJS.ErrnoException;
    if (err.code === 'ENOENT') {
      console.error(chalk.red('✗ foundation.json not found'));
      console.error(
        chalk.yellow(
          '  Run: fspec discover-foundation to create foundation.json',
        ),
      );
      throw new Error('foundation.json not found');
    }
    throw error;
  }

  // Ensure personas array exists
  if (!foundation.personas) {
    foundation.personas = [];
  }

  // Add new persona
  foundation.personas.push({
    name,
    description,
    goals,
  });

  // Write updated foundation.json
  await fs.writeFile(
    foundationPath,
    JSON.stringify(foundation, null, 2) + '\n',
  );

  console.log(chalk.green('✓ Added persona to foundation.json'));
  console.log(chalk.dim(`  Name: ${name}`));
  console.log(chalk.dim(`  Description: ${description}`));
  console.log(chalk.dim(`  Goals: ${goals.join(', ')}`));
}
