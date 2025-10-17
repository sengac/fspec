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
  const draftPath = join(cwd, 'spec', 'foundation.json.draft');
  const foundationPath = join(cwd, 'spec', 'foundation.json');

  // Check for draft first, then foundation.json (draft takes precedence)
  let targetPath = foundationPath;
  let isDraft = false;

  try {
    await fs.access(draftPath);
    targetPath = draftPath;
    isDraft = true;
  } catch {
    // Draft doesn't exist, try foundation.json
  }

  // Read existing foundation file (draft or final)
  let foundation: GenericFoundation;
  try {
    const content = await fs.readFile(targetPath, 'utf-8');
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

  // Write updated file (draft or final)
  await fs.writeFile(
    targetPath,
    JSON.stringify(foundation, null, 2) + '\n',
  );

  const fileName = isDraft ? 'foundation.json.draft' : 'foundation.json';
  console.log(chalk.green(`✓ Added persona to ${fileName}`));
  console.log(chalk.dim(`  Name: ${name}`));
  console.log(chalk.dim(`  Description: ${description}`));
  console.log(chalk.dim(`  Goals: ${goals.join(', ')}`));
}
