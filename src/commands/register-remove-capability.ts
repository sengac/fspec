import type { Command } from 'commander';
import { removeCapability } from './remove-capability';

export function registerRemoveCapabilityCommand(program: Command): void {
  program
    .command('remove-capability')
    .description('Remove a capability from foundation.json')
    .argument('<name>', 'Capability name to remove')
    .action(async (name: string) => {
      try {
        await removeCapability(process.cwd(), name);
        process.exit(0);
      } catch (error: any) {
        process.exit(1);
      }
    });
}
