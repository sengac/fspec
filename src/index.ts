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
import { getScenariosCommand } from './commands/get-scenarios';
import { showAcceptanceCriteriaCommand } from './commands/show-acceptance-criteria';
import { updateTagCommand } from './commands/update-tag';
import { deleteScenarioCommand } from './commands/delete-scenario';
import { deleteStepCommand } from './commands/delete-step';
import { updateScenarioCommand } from './commands/update-scenario';
import { updateStepCommand } from './commands/update-step';
import { deleteTagCommand } from './commands/delete-tag';
import { deleteScenariosByTagCommand } from './commands/delete-scenarios-by-tag';
import { deleteFeaturesByTagCommand } from './commands/delete-features-by-tag';
import { retagCommand } from './commands/retag';
import { addArchitectureCommand } from './commands/add-architecture';
import { addBackgroundCommand } from './commands/add-background';
import { showFeatureCommand } from './commands/show-feature';
import { checkCommand } from './commands/check';
import { addDiagramCommand } from './commands/add-diagram';
import { updateFoundationCommand } from './commands/update-foundation';
import { showFoundationCommand } from './commands/show-foundation';

const program = new Command();

program
  .name('fspec')
  .description(
    'Standardized CLI tool for AI agents to manage Gherkin-based feature specifications'
  )
  .version('1.0.0');

// Validate command
program
  .command('validate')
  .description('Validate Gherkin syntax in feature files')
  .argument(
    '[file]',
    'Feature file to validate (validates all if not specified)'
  )
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
  .argument(
    '[file]',
    'Feature file to validate (validates all if not specified)'
  )
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
  .argument(
    '<feature>',
    'Feature file name or path (e.g., "login" or "spec/features/login.feature")'
  )
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

// Get scenarios command
program
  .command('get-scenarios')
  .description('Get all scenarios matching specified tags')
  .option(
    '--tag <tag>',
    'Filter by tag (can specify multiple times)',
    (value, previous) => {
      return previous ? [...previous, value] : [value];
    }
  )
  .option('--format <format>', 'Output format: text or json', 'text')
  .action(getScenariosCommand);

// Show acceptance criteria command
program
  .command('show-acceptance-criteria')
  .description('Show acceptance criteria for features matching tags')
  .option(
    '--tag <tag>',
    'Filter by tag (can specify multiple times)',
    (value, previous) => {
      return previous ? [...previous, value] : [value];
    }
  )
  .option('--format <format>', 'Output format: text, markdown, or json', 'text')
  .option('--output <file>', 'Write output to file')
  .action(showAcceptanceCriteriaCommand);

// Update tag command
program
  .command('update-tag')
  .description('Update an existing tag in TAGS.md registry')
  .argument('<tag>', 'Tag name (e.g., "@phase1")')
  .option('--category <category>', 'New category name')
  .option('--description <description>', 'New description')
  .action(updateTagCommand);

// Delete scenario command
program
  .command('delete-scenario')
  .description('Delete a scenario from a feature file')
  .argument(
    '<feature>',
    'Feature file name or path (e.g., "login" or "spec/features/login.feature")'
  )
  .argument('<scenario>', 'Scenario name to delete')
  .action(deleteScenarioCommand);

// Delete step command
program
  .command('delete-step')
  .description('Delete a step from a scenario')
  .argument('<feature>', 'Feature file name or path')
  .argument('<scenario>', 'Scenario name')
  .argument('<step>', 'Step text to delete (with or without keyword)')
  .action(deleteStepCommand);

// Update scenario command
program
  .command('update-scenario')
  .description('Rename a scenario in a feature file')
  .argument('<feature>', 'Feature file name or path')
  .argument('<old-name>', 'Current scenario name')
  .argument('<new-name>', 'New scenario name')
  .action(updateScenarioCommand);

// Update step command
program
  .command('update-step')
  .description('Update step text or keyword in a scenario')
  .argument('<feature>', 'Feature file name or path')
  .argument('<scenario>', 'Scenario name')
  .argument('<current-step>', 'Current step text (with or without keyword)')
  .option('--text <text>', 'New step text')
  .option(
    '--keyword <keyword>',
    'New step keyword (Given, When, Then, And, But)'
  )
  .action(updateStepCommand);

// Delete tag command
program
  .command('delete-tag')
  .description('Delete a tag from TAGS.md registry')
  .argument('<tag>', 'Tag name (e.g., "@deprecated")')
  .option('--force', 'Delete tag even if used in feature files')
  .option('--dry-run', 'Show what would be deleted without making changes')
  .action(deleteTagCommand);

// Delete scenarios by tag command
program
  .command('delete-scenarios')
  .description('Bulk delete scenarios by tag across multiple files')
  .option(
    '--tag <tag>',
    'Filter by tag (can specify multiple times for AND logic)',
    (value, previous) => {
      return previous ? [...previous, value] : [value];
    }
  )
  .option('--dry-run', 'Preview deletions without making changes')
  .action(deleteScenariosByTagCommand);

// Delete features by tag command
program
  .command('delete-features')
  .description('Bulk delete feature files by tag')
  .option(
    '--tag <tag>',
    'Filter by tag (can specify multiple times for AND logic)',
    (value, previous) => {
      return previous ? [...previous, value] : [value];
    }
  )
  .option('--dry-run', 'Preview deletions without making changes')
  .action(deleteFeaturesByTagCommand);

// Retag command
program
  .command('retag')
  .description('Bulk rename tags across all feature files')
  .option('--from <tag>', 'Tag to rename from (e.g., @old-tag)')
  .option('--to <tag>', 'Tag to rename to (e.g., @new-tag)')
  .option('--dry-run', 'Preview changes without making modifications')
  .action(retagCommand);

// Add architecture command
program
  .command('add-architecture')
  .description('Add or update architecture documentation in a feature file')
  .argument(
    '<feature>',
    'Feature file name or path (e.g., "login" or "spec/features/login.feature")'
  )
  .argument('<text>', 'Architecture documentation text (can be multi-line)')
  .action(addArchitectureCommand);

// Add background command
program
  .command('add-background')
  .description(
    'Add or update Background (user story) section in a feature file'
  )
  .argument(
    '<feature>',
    'Feature file name or path (e.g., "login" or "spec/features/login.feature")'
  )
  .argument('<text>', 'User story text (As a... I want to... So that...)')
  .action(addBackgroundCommand);

// Show feature command
program
  .command('show-feature')
  .description('Display feature file contents')
  .argument(
    '<feature>',
    'Feature file name or path (e.g., "login" or "spec/features/login.feature")'
  )
  .option('--format <format>', 'Output format: text or json', 'text')
  .option('--output <file>', 'Write output to file')
  .action(showFeatureCommand);

// Check command
program
  .command('check')
  .description('Run all validation checks (Gherkin syntax, tags, formatting)')
  .option('-v, --verbose', 'Show detailed validation output', false)
  .action(checkCommand);

// Add diagram command
program
  .command('add-diagram')
  .description('Add or update Mermaid diagram in FOUNDATION.md')
  .argument('<section>', 'Section name (e.g., "Architecture", "Data Flow")')
  .argument('<title>', 'Diagram title')
  .argument('<code>', 'Mermaid diagram code')
  .action(addDiagramCommand);

// Update foundation command
program
  .command('update-foundation')
  .description('Update section content in FOUNDATION.md')
  .argument('<section>', 'Section name (e.g., "What We Are Building", "Why")')
  .argument('<content>', 'Section content (can be multi-line)')
  .action(updateFoundationCommand);

// Show foundation command
program
  .command('show-foundation')
  .description('Display FOUNDATION.md content')
  .option('--section <section>', 'Show specific section only')
  .option('--format <format>', 'Output format: text, markdown, or json', 'text')
  .option('--output <file>', 'Write output to file')
  .option('--list-sections', 'List section names only', false)
  .option('--line-numbers', 'Show line numbers', false)
  .action(showFoundationCommand);

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
