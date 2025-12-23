#!/usr/bin/env node
/**
 * Pre-publish check for @sengac/fspec
 *
 * Ensures that file: references are replaced with proper npm package versions
 * before publishing to npm.
 *
 * This check runs automatically via npm's prepublishOnly hook.
 */

import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const packageJsonPath = join(__dirname, '..', 'package.json');

try {
  const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));
  const deps = packageJson.dependencies || {};

  const fileRefs = Object.entries(deps).filter(([, version]) =>
    typeof version === 'string' && version.startsWith('file:')
  );

  if (fileRefs.length > 0) {
    console.error('\n❌ ERROR: Cannot publish with file: dependencies!\n');
    console.error('The following dependencies use local file references:');
    fileRefs.forEach(([name, version]) => {
      console.error(`  - ${name}: ${version}`);
    });
    console.error('\nBefore publishing @sengac/fspec, you must:');
    console.error('');
    console.error('  1. First publish @sengac/codelet-napi:');
    console.error('     git tag codelet-napi-v0.1.0');
    console.error('     git push origin codelet-napi-v0.1.0');
    console.error('');
    console.error('  2. Update package.json dependency:');
    console.error('     Change: "codelet-napi": "file:codelet/napi"');
    console.error('     To:     "@sengac/codelet-napi": "^0.1.0"');
    console.error('');
    console.error('  3. Run: npm publish');
    console.error('');
    console.error('  4. After publishing, revert to file reference for development:');
    console.error('     git checkout package.json');
    console.error('');
    process.exit(1);
  }

  console.log('✓ No file: dependencies found - safe to publish');
  process.exit(0);
} catch (error) {
  console.error('Error checking package.json:', error.message);
  process.exit(1);
}
