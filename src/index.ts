#!/usr/bin/env node

import { Command } from 'commander';
import { validateCommand } from './commands/validate';
import { createFeatureCommand } from './commands/create-feature';
import { listFeaturesCommand } from './commands/list-features';
import { formatCommand } from './commands/format';
import { validateTagsCommand } from './commands/validate-tags';
import { registerTagCommand } from './commands/register-tag';
import { listTagsCommand } from './commands/list-tags';
import { tagStatsCommand } from './commands/tag-stats';
import { addScenarioCommand } from './commands/add-scenario';
import { addStepCommand } from './commands/add-step';

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

// Validate tags command
program
  .command('validate-tags')
  .description('Validate feature file tags against TAGS.md registry')
  .argument('[file]', 'Feature file to validate (validates all if not specified)')
  .action(validateTagsCommand);

// Register tag command
program
  .command('register-tag')
  .description('Register a new tag in TAGS.md registry')
  .argument('<tag>', 'Tag name (e.g., "@my-tag")')
  .argument('<category>', 'Category name (e.g., "Technical Tags")')
  .argument('<description>', 'Tag description')
  .action(registerTagCommand);

// List tags command
program
  .command('list-tags')
  .description('List all registered tags from TAGS.md')
  .option('--category <category>', 'Filter by category (e.g., "Phase Tags")')
  .action(listTagsCommand);

// Tag stats command
program
  .command('tag-stats')
  .description('Show tag usage statistics across all feature files')
  .action(tagStatsCommand);

// Add scenario command
program
  .command('add-scenario')
  .description('Add a new scenario to an existing feature file')
  .argument('<feature>', 'Feature file name or path (e.g., "login" or "spec/features/login.feature")')
  .argument('<scenario>', 'Scenario name (e.g., "Successful login")')
  .action(addScenarioCommand);

// Add step command
program
  .command('add-step')
  .description('Add a step to an existing scenario')
  .argument('<feature>', 'Feature file name or path')
  .argument('<scenario>', 'Scenario name')
  .argument('<type>', 'Step type: given, when, then, and, but')
  .argument('<text>', 'Step text')
  .action(addStepCommand);

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
