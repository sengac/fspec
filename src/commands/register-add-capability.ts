import type { Command } from 'commander';
import { addCapability } from './add-capability';

export function registerAddCapabilityCommand(program: Command): void {
  program
    .command('add-capability')
    .description('Add a capability to foundation.json')
    .argument('<name>', 'Capability name')
    .argument('<description>', 'Capability description')
    .action(async (name: string, description: string) => {
      try {
        await addCapability(process.cwd(), name, description);
      } catch (error: unknown) {
        const err = error as Error;
        console.error(err.message);
        process.exit(1);
      }
    });
}
