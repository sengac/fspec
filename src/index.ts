#!/usr/bin/env node

import { Command } from 'commander';
import { validateCommand } from './commands/validate.js';
import { createFeatureCommand } from './commands/create-feature.js';
import { listFeaturesCommand } from './commands/list-features.js';
import { formatCommand } from './commands/format.js';

const program = new Command();

program
  .name('fspec')
  .description('Standardized CLI tool for AI agents to manage Gherkin-based feature specifications')
  .version('1.0.0');

// Validate command
program
  .command('validate')
  .description('Validate Gherkin syntax in feature files')
  .argument('[file]', 'Feature file to validate (validates all if not specified)')
  .option('-v, --verbose', 'Show detailed validation output', false)
  .action(validateCommand);

// Create feature command
program
  .command('create-feature')
  .description('Create a new feature file with template')
  .argument('<name>', 'Feature name (e.g., "User Authentication")')
  .action(createFeatureCommand);

// List features command
program
  .command('list-features')
  .description('List all feature files')
  .option('--tag <tag>', 'Filter by tag (e.g., --tag=@phase1)')
  .action(listFeaturesCommand);

// Format command
program
  .command('format')
  .description('Format feature files with Prettier')
  .argument('[file]', 'Feature file to format (formats all if not specified)')
  .action(formatCommand);

// TODO: Add more commands
// - create-feature
// - add-scenario
// - add-step
// - add-architecture
// - add-background
// - list-features
// - show-feature
// - add-diagram
// - update-foundation
// - show-foundation
// - register-tag
// - validate-tags
// - list-tags
// - tag-stats
// - format
// - check

program.parse();
