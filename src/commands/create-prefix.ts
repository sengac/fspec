import { writeFile } from 'fs/promises';
import { join } from 'path';
import { ensurePrefixesFile } from '../utils/ensure-files';

interface Prefix {
  prefix: string;
  description: string;
  epicId?: string;
  createdAt: string;
}

interface PrefixesData {
  prefixes: Record<string, Prefix>;
}

const PREFIX_REGEX = /^[A-Z]{2,6}$/;

export async function createPrefix(options: {
  prefix: string;
  description: string;
  cwd?: string;
}): Promise<{ success: boolean }> {
  const cwd = options.cwd || process.cwd();

  // Validate prefix format: 2-6 uppercase letters
  if (!PREFIX_REGEX.test(options.prefix)) {
    throw new Error('Prefix must be 2-6 uppercase letters (e.g., AUTH, DASH)');
  }

  try {
    // Read existing prefixes or initialize
    const data: PrefixesData = await ensurePrefixesFile(cwd);
    const prefixesFile = join(cwd, 'spec', 'prefixes.json');

    // Check if prefix already exists
    if (data.prefixes[options.prefix]) {
      throw new Error(`Prefix ${options.prefix} already exists`);
    }

    // Create new prefix
    const newPrefix: Prefix = {
      prefix: options.prefix,
      description: options.description,
      createdAt: new Date().toISOString(),
    };

    data.prefixes[options.prefix] = newPrefix;

    // Write back to file
    await writeFile(prefixesFile, JSON.stringify(data, null, 2));

    return { success: true };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to create prefix: ${error.message}`);
    }
    throw error;
  }
}
