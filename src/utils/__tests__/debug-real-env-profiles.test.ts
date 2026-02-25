/**
 * Debug test for profile loading in real environment
 *
 * Run this test WITHOUT changing HOME to see what the real TUI would see.
 * This helps debug why profiles aren't loading in the actual application.
 */

import { describe, it, expect } from 'vitest';
import { join } from 'path';
import { existsSync } from 'fs';
import { readFile } from 'fs/promises';

describe('Debug: Real Environment Profile Loading', () => {
  it('should show what the real TUI sees', async () => {
    // What is HOME in this environment?
    const realHome = process.env.HOME;
    console.log('========================================');
    console.log('REAL ENVIRONMENT DEBUG');
    console.log('========================================');
    console.log('process.env.HOME:', realHome);

    // What is getFspecUserDir returning?
    const { getFspecUserDir } = await import('../config');
    const fspecDir = getFspecUserDir();
    console.log('getFspecUserDir():', fspecDir);

    // Does the config file exist?
    const configPath = join(fspecDir, 'fspec-config.json');
    console.log('Config path:', configPath);
    console.log('Config exists:', existsSync(configPath));

    // Read the config file directly
    if (existsSync(configPath)) {
      const content = await readFile(configPath, 'utf-8');
      console.log('Config content:');
      console.log(content);
    }

    // Now try to load via loadConfig
    const { loadConfig } = await import('../config');
    const config = await loadConfig(process.cwd());
    console.log('\nloadConfig() result:');
    console.log('  providers:', config?.providers ? 'exists' : 'MISSING');
    if (config?.providers) {
      console.log('  openai:', config.providers.openai ? 'exists' : 'MISSING');
      if (config.providers.openai) {
        console.log(
          '  openai.profiles:',
          config.providers.openai.profiles ? 'exists' : 'MISSING'
        );
        if (config.providers.openai.profiles) {
          console.log(
            '  profiles:',
            Object.keys(config.providers.openai.profiles)
          );
        }
      }
    }

    // Try loadProviderProfiles directly
    const { loadProviderProfiles } = await import('../provider-config');
    const profiles = await loadProviderProfiles('openai');
    console.log('\nloadProviderProfiles("openai"):');
    console.log('  Result:', JSON.stringify(profiles, null, 2));
    console.log('  Profile count:', Object.keys(profiles).length);

    // This test always passes - it's just for debug output
    expect(true).toBe(true);
  });
});
