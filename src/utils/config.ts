import { readFile, writeFile as fsWriteFile, mkdir } from 'fs/promises';
import { join } from 'path';
import { homedir } from 'os';

/**
 * Deep merge two objects recursively
 * Project-level values override user-level values
 */
function deepMerge(target: any, source: any): any {
  const result = { ...target };

  for (const key in source) {
    if (source[key] && typeof source[key] === 'object' && !Array.isArray(source[key])) {
      result[key] = deepMerge(target[key] || {}, source[key]);
    } else {
      result[key] = source[key];
    }
  }

  return result;
}

/**
 * Load config from a specific path
 * Returns empty object if file doesn't exist or is empty
 * Throws error if JSON is invalid
 */
async function loadConfigFile(path: string): Promise<any> {
  try {
    const content = await readFile(path, 'utf-8');

    // Empty file = valid empty config
    if (!content.trim()) {
      return {};
    }

    try {
      return JSON.parse(content);
    } catch (error) {
      // Invalid JSON = throw error
      const err = error as Error;
      throw new Error(`Invalid JSON in ${path}: ${err.message}`);
    }
  } catch (error: any) {
    // File doesn't exist = return empty object (silent fallback)
    if (error.code === 'ENOENT' || error.message?.includes('Invalid JSON')) {
      if (error.message?.includes('Invalid JSON')) {
        throw error; // Re-throw JSON parse errors
      }
      return {};
    }
    throw error;
  }
}

/**
 * Load and merge user-level and project-level config
 * Project-level overrides user-level (deep merge)
 *
 * @param cwd - Current working directory (project root)
 * @returns Merged configuration object
 */
export async function loadConfig(cwd: string = process.cwd()): Promise<any> {
  // Load user-level config from ~/.fspec/fspec-config.json
  const userConfigPath = join(homedir(), '.fspec', 'fspec-config.json');
  const userConfig = await loadConfigFile(userConfigPath);

  // Load project-level config from <cwd>/spec/fspec-config.json
  const projectConfigPath = join(cwd, 'spec', 'fspec-config.json');
  const projectConfig = await loadConfigFile(projectConfigPath);

  // Deep merge: project-level overrides user-level
  return deepMerge(userConfig, projectConfig);
}

/**
 * Write config to user-level or project-level scope
 *
 * @param scope - 'user' for ~/.fspec/fspec-config.json or 'project' for spec/fspec-config.json
 * @param config - Configuration object to write
 * @param cwd - Current working directory (for project scope)
 */
export async function writeConfig(
  scope: 'user' | 'project',
  config: any,
  cwd: string = process.cwd()
): Promise<void> {
  let configPath: string;
  let configDir: string;

  if (scope === 'user') {
    configDir = join(homedir(), '.fspec');
    configPath = join(configDir, 'fspec-config.json');
  } else {
    configDir = join(cwd, 'spec');
    configPath = join(configDir, 'fspec-config.json');
  }

  // Ensure directory exists
  await mkdir(configDir, { recursive: true });

  // Write formatted JSON
  await fsWriteFile(configPath, JSON.stringify(config, null, 2), 'utf-8');
}
