import type { Command } from 'commander';
import { addPersona } from './add-persona';

export function registerAddPersonaCommand(program: Command): void {
  program
    .command('add-persona')
    .description('Add a persona to foundation.json')
    .argument('<name>', 'Persona name')
    .argument('<description>', 'Persona description')
    .option(
      '--goal <goal>',
      'Persona goal (can be repeated)',
      (goal, goals: string[]) => {
        goals.push(goal);
        return goals;
      },
      [] as string[]
    )
    .action(
      async (
        name: string,
        description: string,
        options: { goal: string[] }
      ) => {
        try {
          await addPersona(process.cwd(), name, description, options.goal);
        } catch (error: unknown) {
          const err = error as Error;
          console.error(err.message);
          process.exit(1);
        }
      }
    );
}
