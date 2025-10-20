import { promises as fs } from 'fs';
import { join } from 'path';
import chalk from 'chalk';
import type { GenericFoundation } from '../types/foundation';

export async function removePersona(cwd: string, name: string): Promise<void> {
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
          '  Run: fspec discover-foundation to create foundation.json'
        )
      );
      throw new Error('foundation.json not found');
    }
    throw error;
  }

  // Ensure personas array exists
  if (!foundation.personas || foundation.personas.length === 0) {
    console.error(chalk.red(`✗ Persona "${name}" not found`));
    console.error(chalk.yellow('  No personas exist in foundation'));
    throw new Error(`Persona "${name}" not found`);
  }

  // Find persona by exact name match (case-sensitive)
  const index = foundation.personas.findIndex(p => p.name === name);
  if (index === -1) {
    const availableNames = foundation.personas.map(p => p.name).join(', ');
    console.error(chalk.red(`✗ Persona "${name}" not found`));
    console.error(chalk.yellow(`  Available personas: ${availableNames}`));
    throw new Error(`Persona "${name}" not found`);
  }

  // Remove persona
  foundation.personas.splice(index, 1);

  // Write updated file (draft or final)
  await fs.writeFile(targetPath, JSON.stringify(foundation, null, 2) + '\n');

  const fileName = isDraft ? 'foundation.json.draft' : 'foundation.json';
  console.log(chalk.green(`✓ Removed persona "${name}" from ${fileName}`));
}
