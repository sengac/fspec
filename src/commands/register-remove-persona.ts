import type { Command } from 'commander';
import { removePersona } from './remove-persona';

export function registerRemovePersonaCommand(program: Command): void {
  program
    .command('remove-persona')
    .description('Remove a persona from foundation.json')
    .argument('<name>', 'Persona name to remove')
    .action(async (name: string) => {
      try {
        await removePersona(process.cwd(), name);
        process.exit(0);
      } catch (error: any) {
        process.exit(1);
      }
    });
}
