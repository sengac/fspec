#!/usr/bin/env node

/**
 * Command Help File Validation Script
 *
 * Validates that all fspec commands have corresponding help files
 * and all help files are properly formatted.
 *
 * Coverage:
 * - CLI-013: Validation script to check all commands are linked and help files exist
 *
 * Usage: node scripts/validate-command-help.js
 * Exit Code: 0 if all checks pass, 1 if issues found
 */

import { readdir, readFile } from 'fs/promises';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { existsSync } from 'fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Allow passing project root as command line argument, otherwise use script location
const PROJECT_ROOT = process.argv[2] || join(__dirname, '..');
const COMMANDS_DIR = join(PROJECT_ROOT, 'src', 'commands');

/** @typedef {Object} ValidationIssue
 * @property {string} type - Type of issue
 * @property {string} message - Issue description
 * @property {string} [file] - Related file path
 */

/**
 * Get all registered commands from src/index.ts
 * @returns {Promise<string[]>} Array of command names
 */
async function getRegisteredCommands() {
  const indexPath = join(PROJECT_ROOT, 'src', 'index.ts');
  const content = await readFile(indexPath, 'utf-8');

  // Find all register*Command imports and extract command name
  // Example: import { registerAddRuleCommand } from './commands/add-rule';
  const commandMatches = content.matchAll(/import.*register\w+Command.*from.*'\.\/commands\/([^']+)'/g);

  const commands = [];
  for (const match of commandMatches) {
    const fileName = match[1]; // e.g., 'add-rule'
    // Skip files that don't follow the pattern
    if (fileName.startsWith('register-')) {
      // Files like 'register-add-capability' -> 'add-capability'
      commands.push(fileName.replace('register-', ''));
    } else {
      commands.push(fileName);
    }
  }

  return commands;
}

/**
 * Get all help files from commands directory
 * @returns {Promise<string[]>} Array of help file names (without extension)
 */
async function getHelpFiles() {
  const files = await readdir(COMMANDS_DIR);
  return files
    .filter(f => f.endsWith('-help.ts'))
    .map(f => f.replace('-help.ts', ''));
}

/**
 * Validate that help file exports correct function
 * @param {string} commandName - Command name
 * @returns {Promise<{valid: boolean, issue?: string}>}
 */
async function validateHelpFileExport(commandName) {
  const helpFilePath = join(COMMANDS_DIR, `${commandName}-help.ts`);

  try {
    const content = await readFile(helpFilePath, 'utf-8');

    // Check that file is non-empty
    if (content.trim().length === 0) {
      return { valid: false, issue: 'Help content is empty' };
    }

    // Check for export function (flexible pattern to match various export styles)
    const hasExport = /export\s+(function|const|default)/.test(content);
    if (!hasExport) {
      return { valid: false, issue: 'Invalid help file export: missing export statement' };
    }

    return { valid: true };
  } catch (error) {
    return { valid: false, issue: `Failed to read help file: ${error.message}` };
  }
}

/**
 * Validate help file content formatting consistency
 * @param {string} commandName - Command name
 * @returns {Promise<{valid: boolean, issues: string[]}>}
 */
async function validateHelpFileFormat(commandName) {
  const helpFilePath = join(COMMANDS_DIR, `${commandName}-help.ts`);
  const content = await readFile(helpFilePath, 'utf-8');

  const issues = [];

  // Check for required sections (case-insensitive)
  const requiredSections = ['USAGE', 'EXAMPLES'];
  for (const section of requiredSections) {
    const sectionRegex = new RegExp(`^\\s*${section}`, 'im');
    if (!sectionRegex.test(content)) {
      issues.push(`Missing ${section} section`);
    }
  }

  // Check for empty sections
  const usageMatch = content.match(/USAGE\s*\n\s*\n/i);
  if (usageMatch) {
    issues.push('USAGE section is empty');
  }

  const examplesMatch = content.match(/EXAMPLES\s*\n\s*\n/i);
  if (examplesMatch) {
    issues.push('EXAMPLES section is empty');
  }

  return {
    valid: issues.length === 0,
    issues
  };
}

/**
 * Main validation function
 * @returns {Promise<number>} Exit code (0 = success, 1 = failure)
 */
async function validate() {
  console.log('Validating command-help file linkage...\n');

  /** @type {ValidationIssue[]} */
  const issues = [];

  // Get commands and help files
  const commands = await getRegisteredCommands();
  const helpFiles = await getHelpFiles();

  console.log(`Found ${commands.length} registered commands`);
  console.log(`Found ${helpFiles.length} help files\n`);

  // Check for missing help files
  for (const command of commands) {
    const helpFilePath = join(COMMANDS_DIR, `${command}-help.ts`);
    if (!existsSync(helpFilePath)) {
      issues.push({
        type: 'missing-help',
        message: `Missing help file for command: ${command}`,
        file: `src/commands/${command}-help.ts (expected)`
      });
    } else {
      // Validate export signature
      const exportValidation = await validateHelpFileExport(command);
      if (!exportValidation.valid) {
        issues.push({
          type: 'invalid-export',
          message: `${exportValidation.issue}: ${command}-help.ts`,
          file: `src/commands/${command}-help.ts`
        });
      }

      // Validate content formatting
      const formatValidation = await validateHelpFileFormat(command);
      if (!formatValidation.valid) {
        for (const formatIssue of formatValidation.issues) {
          issues.push({
            type: 'format-inconsistency',
            message: `Inconsistent help format in ${command}-help.ts: ${formatIssue}`,
            file: `src/commands/${command}-help.ts`
          });
        }
      }
    }
  }

  // Check for orphaned help files
  for (const helpFile of helpFiles) {
    if (!commands.includes(helpFile)) {
      issues.push({
        type: 'orphaned-help',
        message: `Orphaned help file: ${helpFile}-help.ts`,
        file: `src/commands/${helpFile}-help.ts`
      });
    }
  }

  // Report results
  if (issues.length === 0) {
    console.log(`✅ All commands validated successfully (checked ${commands.length} commands)`);
    return 0;
  } else {
    console.error(`❌ Found ${issues.length} issue(s):\n`);

    // Group issues by type
    const groupedIssues = issues.reduce((acc, issue) => {
      if (!acc[issue.type]) acc[issue.type] = [];
      acc[issue.type].push(issue);
      return acc;
    }, {});

    for (const [type, typeIssues] of Object.entries(groupedIssues)) {
      console.error(`\n${type.toUpperCase().replace(/-/g, ' ')}:`);
      for (const issue of typeIssues) {
        console.error(`  ❌ ${issue.message}`);
        if (issue.file) {
          console.error(`     File: ${issue.file}`);
        }
      }
    }

    console.error('\n');
    return 1;
  }
}

// Run validation
validate()
  .then(exitCode => {
    process.exit(exitCode);
  })
  .catch(error => {
    console.error('Validation script failed with error:', error);
    process.exit(1);
  });
