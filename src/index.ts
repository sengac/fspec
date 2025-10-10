#!/usr/bin/env node

import { Command } from 'commander';
import chalk from 'chalk';
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
import { deleteDiagramCommand } from './commands/delete-diagram';
import { generateFoundationMdCommandCLI } from './commands/generate-foundation-md';
import { generateTagsMdCommandCLI } from './commands/generate-tags-md';
import { addTagToFeatureCommand } from './commands/add-tag-to-feature';
import { removeTagFromFeatureCommand } from './commands/remove-tag-from-feature';
import { listFeatureTagsCommand } from './commands/list-feature-tags';
import { addTagToScenarioCommand } from './commands/add-tag-to-scenario';
import { removeTagFromScenarioCommand } from './commands/remove-tag-from-scenario';
import { listScenarioTagsCommand } from './commands/list-scenario-tags';
// Project management commands
import { createWorkUnitCommand } from './commands/create-work-unit-command';
import { listWorkUnitsCommand } from './commands/list-work-units-command';
import { showWorkUnitCommand } from './commands/show-work-unit-command';
import { createEpicCommand } from './commands/create-epic-command';
import { listEpicsCommand } from './commands/list-epics-command';
import { showEpicCommand } from './commands/show-epic-command';

const program = new Command();

// Custom help display
function displayCustomHelp(): void {
  console.log(chalk.bold('\nfspec - Feature Specification & Project Management for AI Agents'));
  console.log(chalk.dim('Version 0.0.1\n'));

  console.log(chalk.bold('USAGE'));
  console.log('  fspec [command] [options]\n');

  console.log(chalk.bold('COMMAND GROUPS'));
  console.log('  Use ' + chalk.cyan('fspec help <group>') + ' for detailed help on a specific area:\n');
  console.log('  ' + chalk.cyan('spec') + '        - Specification Management (Gherkin features, scenarios, validation)');
  console.log('  ' + chalk.cyan('tags') + '        - Tag Registry & Management');
  console.log('  ' + chalk.cyan('foundation') + '   - Foundation & Architecture Documentation');
  console.log('  ' + chalk.cyan('query') + '       - Query & Reporting (supports scenario-level tags)');
  console.log('  ' + chalk.cyan('project') + '     - Project Management (work units, epics, Kanban workflow)');
  console.log('');

  console.log(chalk.bold('QUICK START'));
  console.log('  ' + chalk.cyan('fspec validate') + '              - Validate all Gherkin feature files');
  console.log('  ' + chalk.cyan('fspec create-feature NAME') + '   - Create a new feature file');
  console.log('  ' + chalk.cyan('fspec list-features') + '         - List all feature files');
  console.log('  ' + chalk.cyan('fspec check') + '                 - Run all validation checks\n');

  console.log(chalk.bold('GET HELP'));
  console.log('  ' + chalk.cyan('fspec --help') + '          - Show this help');
  console.log('  ' + chalk.cyan('fspec help spec') + '       - Specification management commands');
  console.log('  ' + chalk.cyan('fspec help tags') + '       - Tag management commands');
  console.log('  ' + chalk.cyan('fspec help foundation') + ' - Foundation documentation commands');
  console.log('  ' + chalk.cyan('fspec help query') + '      - Query and reporting commands');
  console.log('  ' + chalk.cyan('fspec <command> --help') + ' - Detailed help for specific command\n');

  console.log(chalk.bold('EXAMPLES'));
  console.log('  # Validate specific feature file');
  console.log('  ' + chalk.dim('$ fspec validate spec/features/login.feature'));
  console.log('');
  console.log('  # Filter features by tag');
  console.log('  ' + chalk.dim('$ fspec list-features --tag=@phase1'));
  console.log('');
  console.log('  # Add scenario to feature');
  console.log('  ' + chalk.dim('$ fspec add-scenario user-authentication "Login with valid credentials"'));
  console.log('');
  console.log('  # Query scenarios by tags');
  console.log('  ' + chalk.dim('$ fspec get-scenarios --tag=@phase1 --tag=@critical --format=json'));
  console.log('');

  console.log(chalk.bold('DOCUMENTATION'));
  console.log('  GitHub: ' + chalk.cyan('https://github.com/rquast/fspec'));
  console.log('  README: ' + chalk.dim('See README.md for detailed usage examples'));
  console.log('');
}

function displaySpecHelp(): void {
  console.log(chalk.bold('\nSPECIFICATION MANAGEMENT\n'));

  console.log(chalk.bold('VALIDATION & CHECKING'));
  console.log('  ' + chalk.cyan('fspec validate [file]') + '             Validate Gherkin syntax');
  console.log('    Options:');
  console.log('      -v, --verbose                    Show detailed validation output');
  console.log('    Examples:');
  console.log('      fspec validate                   Validate all feature files');
  console.log('      fspec validate spec/features/login.feature');
  console.log('');
  console.log('  ' + chalk.cyan('fspec check') + '                        Run all validation checks');
  console.log('    Options:');
  console.log('      -v, --verbose                    Show detailed output');
  console.log('');
  console.log('  ' + chalk.cyan('fspec format [file]') + '                Format feature files with Prettier');
  console.log('    Examples:');
  console.log('      fspec format                     Format all feature files');
  console.log('      fspec format spec/features/login.feature');
  console.log('');

  console.log(chalk.bold('FEATURE FILE MANAGEMENT'));
  console.log('  ' + chalk.cyan('fspec create-feature <name>') + '       Create new feature file');
  console.log('    Examples:');
  console.log('      fspec create-feature "User Authentication"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec list-features') + '                List all feature files');
  console.log('    Options:');
  console.log('      --tag=<tag>                      Filter by tag');
  console.log('    Examples:');
  console.log('      fspec list-features --tag=@phase1');
  console.log('');
  console.log('  ' + chalk.cyan('fspec show-feature <feature>') + '      Display feature file contents');
  console.log('    Options:');
  console.log('      --format=<format>                Output format: text or json (default: text)');
  console.log('      --output=<file>                  Write output to file');
  console.log('    Examples:');
  console.log('      fspec show-feature user-authentication');
  console.log('      fspec show-feature login --format=json --output=login.json');
  console.log('');

  console.log(chalk.bold('SCENARIO MANAGEMENT'));
  console.log('  ' + chalk.cyan('fspec add-scenario <feature> <scenario>') + ' Add scenario to feature');
  console.log('    Examples:');
  console.log('      fspec add-scenario login "Successful login"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec update-scenario <feature> <old> <new>') + ' Rename scenario');
  console.log('    Examples:');
  console.log('      fspec update-scenario login "Old Name" "New Name"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec delete-scenario <feature> <scenario>') + ' Delete scenario');
  console.log('    Examples:');
  console.log('      fspec delete-scenario login "Obsolete scenario"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec delete-scenarios') + '             Bulk delete scenarios by tag');
  console.log('    Options:');
  console.log('      --tag=<tag>                      Filter by tag (multiple allowed, AND logic)');
  console.log('      --dry-run                        Preview without making changes');
  console.log('    Examples:');
  console.log('      fspec delete-scenarios --tag=@deprecated --dry-run');
  console.log('      fspec delete-scenarios --tag=@phase1 --tag=@wip');
  console.log('');

  console.log(chalk.bold('STEP MANAGEMENT'));
  console.log('  ' + chalk.cyan('fspec add-step <feature> <scenario> <type> <text>') + ' Add step');
  console.log('    Step types: given, when, then, and, but');
  console.log('    Examples:');
  console.log('      fspec add-step login "Login" given "I am on the login page"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec update-step <feature> <scenario> <step>') + ' Update step');
  console.log('    Options:');
  console.log('      --text=<text>                    New step text');
  console.log('      --keyword=<keyword>              New keyword (Given, When, Then, And, But)');
  console.log('    Examples:');
  console.log('      fspec update-step login "Login" "I am on page" --text "I navigate to page"');
  console.log('      fspec update-step login "Login" "I am on page" --keyword=When');
  console.log('');
  console.log('  ' + chalk.cyan('fspec delete-step <feature> <scenario> <step>') + ' Delete step');
  console.log('    Examples:');
  console.log('      fspec delete-step login "Login" "I am on the login page"');
  console.log('');

  console.log(chalk.bold('DOCUMENTATION IN FEATURES'));
  console.log('  ' + chalk.cyan('fspec add-architecture <feature> <text>') + ' Add architecture docs');
  console.log('    Examples:');
  console.log('      fspec add-architecture login "Uses JWT tokens for sessions"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec add-background <feature> <text>') + ' Add user story');
  console.log('    Examples:');
  console.log('      fspec add-background login "As a user\\nI want to log in\\nSo that..."');
  console.log('');

  console.log(chalk.bold('BULK OPERATIONS'));
  console.log('  ' + chalk.cyan('fspec delete-features') + '              Delete feature files by tag');
  console.log('    Options:');
  console.log('      --tag=<tag>                      Filter by tag (multiple allowed, AND logic)');
  console.log('      --dry-run                        Preview without making changes');
  console.log('    Examples:');
  console.log('      fspec delete-features --tag=@deprecated --dry-run');
  console.log('');
}

function displayTagsHelp(): void {
  console.log(chalk.bold('\nTAG REGISTRY & MANAGEMENT\n'));
  console.log(chalk.dim('Tags are managed in spec/tags.json with auto-generated spec/TAGS.md\n'));

  console.log(chalk.bold('TAG VALIDATION'));
  console.log('  ' + chalk.cyan('fspec validate-tags [file]') + '        Validate tags against registry');
  console.log('    Examples:');
  console.log('      fspec validate-tags              Check all feature files');
  console.log('      fspec validate-tags spec/features/login.feature');
  console.log('');

  console.log(chalk.bold('TAG REGISTRY MANAGEMENT'));
  console.log('  ' + chalk.cyan('fspec register-tag <tag> <category> <desc>') + ' Register new tag');
  console.log('    Examples:');
  console.log('      fspec register-tag @performance "Technical Tags" "Performance features"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec update-tag <tag>') + '             Update existing tag');
  console.log('    Options:');
  console.log('      --category=<category>            New category name');
  console.log('      --description=<desc>             New description');
  console.log('    Examples:');
  console.log('      fspec update-tag @performance --description="Updated description"');
  console.log('      fspec update-tag @performance --category="Technical Tags"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec delete-tag <tag>') + '             Delete tag from registry');
  console.log('    Options:');
  console.log('      --force                          Delete even if used in features');
  console.log('      --dry-run                        Preview without making changes');
  console.log('    Examples:');
  console.log('      fspec delete-tag @deprecated --dry-run');
  console.log('      fspec delete-tag @old --force');
  console.log('');
  console.log('  ' + chalk.cyan('fspec list-tags') + '                   List all registered tags');
  console.log('    Options:');
  console.log('      --category=<category>            Filter by category');
  console.log('    Examples:');
  console.log('      fspec list-tags');
  console.log('      fspec list-tags --category="Technical Tags"');
  console.log('');

  console.log(chalk.bold('TAG OPERATIONS'));
  console.log('  ' + chalk.cyan('fspec retag') + '                       Rename tags across all files');
  console.log('    Options:');
  console.log('      --from=<tag>                     Tag to rename from');
  console.log('      --to=<tag>                       Tag to rename to');
  console.log('      --dry-run                        Preview without making changes');
  console.log('    Examples:');
  console.log('      fspec retag --from=@old-tag --to=@new-tag --dry-run');
  console.log('');
  console.log('  ' + chalk.cyan('fspec tag-stats') + '                   Show tag usage statistics');
  console.log('    Examples:');
  console.log('      fspec tag-stats');
  console.log('');

  console.log(chalk.bold('FEATURE-LEVEL TAG MANAGEMENT'));
  console.log('  ' + chalk.cyan('fspec add-tag-to-feature <file> <tags...>') + ' Add tags to feature');
  console.log('    Options:');
  console.log('      --validate-registry              Validate against spec/tags.json');
  console.log('    Examples:');
  console.log('      fspec add-tag-to-feature spec/features/login.feature @critical');
  console.log('      fspec add-tag-to-feature login.feature @critical @security');
  console.log('');
  console.log('  ' + chalk.cyan('fspec remove-tag-from-feature <file> <tags...>') + ' Remove tags');
  console.log('    Examples:');
  console.log('      fspec remove-tag-from-feature spec/features/login.feature @wip');
  console.log('      fspec remove-tag-from-feature login.feature @wip @deprecated');
  console.log('');
  console.log('  ' + chalk.cyan('fspec list-feature-tags <file>') + '   List tags on feature');
  console.log('    Options:');
  console.log('      --show-categories                Show tag categories from registry');
  console.log('    Examples:');
  console.log('      fspec list-feature-tags spec/features/login.feature');
  console.log('      fspec list-feature-tags login.feature --show-categories');
  console.log('');

  console.log(chalk.bold('SCENARIO-LEVEL TAG MANAGEMENT'));
  console.log('  ' + chalk.cyan('fspec add-tag-to-scenario <file> <scenario> <tags...>') + ' Add tags to scenario');
  console.log('    Options:');
  console.log('      --validate-registry              Validate against spec/tags.json');
  console.log('    Examples:');
  console.log('      fspec add-tag-to-scenario login.feature "Login" @smoke');
  console.log('      fspec add-tag-to-scenario login.feature "Login" @smoke @critical');
  console.log('');
  console.log('  ' + chalk.cyan('fspec remove-tag-from-scenario <file> <scenario> <tags...>') + ' Remove tags');
  console.log('    Examples:');
  console.log('      fspec remove-tag-from-scenario login.feature "Login" @wip');
  console.log('      fspec remove-tag-from-scenario login.feature "Login" @wip @deprecated');
  console.log('');
  console.log('  ' + chalk.cyan('fspec list-scenario-tags <file> <scenario>') + ' List tags on scenario');
  console.log('    Options:');
  console.log('      --show-categories                Show tag categories from registry');
  console.log('    Examples:');
  console.log('      fspec list-scenario-tags login.feature "Login"');
  console.log('      fspec list-scenario-tags login.feature "Login" --show-categories');
  console.log('');

  console.log(chalk.bold('NOTES'));
  console.log('  - All tag write operations modify spec/tags.json');
  console.log('  - spec/TAGS.md is auto-generated - never edit manually');
  console.log('  - Regenerate TAGS.md: ' + chalk.cyan('fspec generate-tags-md'));
  console.log('');
}

function displayFoundationHelp(): void {
  console.log(chalk.bold('\nFOUNDATION & ARCHITECTURE DOCUMENTATION\n'));
  console.log(chalk.dim('Foundation is managed in spec/foundation.json with auto-generated spec/FOUNDATION.md\n'));

  console.log(chalk.bold('VIEW FOUNDATION'));
  console.log('  ' + chalk.cyan('fspec show-foundation') + '              Display foundation content');
  console.log('    Options:');
  console.log('      --section=<section>              Show specific section only');
  console.log('      --format=<format>                Output: text, markdown, json (default: text)');
  console.log('      --output=<file>                  Write output to file');
  console.log('      --list-sections                  List section names only');
  console.log('      --line-numbers                   Show line numbers');
  console.log('    Examples:');
  console.log('      fspec show-foundation');
  console.log('      fspec show-foundation --section "What We Are Building"');
  console.log('      fspec show-foundation --format=json --output=foundation.json');
  console.log('      fspec show-foundation --list-sections');
  console.log('');

  console.log(chalk.bold('UPDATE FOUNDATION'));
  console.log('  ' + chalk.cyan('fspec update-foundation <section> <content>') + ' Update section');
  console.log('    Examples:');
  console.log('      fspec update-foundation "What We Are Building" "A CLI tool for..."');
  console.log('');

  console.log(chalk.bold('MERMAID DIAGRAM MANAGEMENT'));
  console.log('  ' + chalk.cyan('fspec add-diagram <section> <title> <code>') + ' Add/update diagram');
  console.log('    Note: Automatically validates Mermaid syntax before adding');
  console.log('    Examples:');
  console.log('      fspec add-diagram "Architecture" "System Context" "graph TD\\n  A-->B"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec delete-diagram <section> <title>') + ' Delete diagram');
  console.log('    Examples:');
  console.log('      fspec delete-diagram "Architecture Diagrams" "System Context"');
  console.log('');

  console.log(chalk.bold('GENERATION'));
  console.log('  ' + chalk.cyan('fspec generate-foundation-md') + '      Generate FOUNDATION.md from JSON');
  console.log('    Examples:');
  console.log('      fspec generate-foundation-md');
  console.log('');

  console.log(chalk.bold('NOTES'));
  console.log('  - All write operations modify spec/foundation.json');
  console.log('  - spec/FOUNDATION.md is auto-generated - never edit manually');
  console.log('  - Mermaid diagrams are validated with mermaid.parse() before adding');
  console.log('');
}

function displayQueryHelp(): void {
  console.log(chalk.bold('\nQUERY & REPORTING\n'));

  console.log(chalk.bold('QUERY SCENARIOS'));
  console.log('  ' + chalk.cyan('fspec get-scenarios') + '                Get scenarios by tag');
  console.log('    Options:');
  console.log('      --tag=<tag>                      Filter by tag (multiple allowed, AND logic)');
  console.log('      --format=<format>                Output: text or json (default: text)');
  console.log('    Examples:');
  console.log('      fspec get-scenarios --tag=@phase1');
  console.log('      fspec get-scenarios --tag=@phase1 --tag=@critical --format=json');
  console.log('      fspec get-scenarios --tag=@smoke  # Matches scenario-level tags');
  console.log('');

  console.log(chalk.bold('ACCEPTANCE CRITERIA'));
  console.log('  ' + chalk.cyan('fspec show-acceptance-criteria') + '    Show acceptance criteria by tag');
  console.log('    Options:');
  console.log('      --tag=<tag>                      Filter by tag (multiple allowed, AND logic)');
  console.log('      --format=<format>                Output: text, markdown, json (default: text)');
  console.log('      --output=<file>                  Write output to file');
  console.log('    Examples:');
  console.log('      fspec show-acceptance-criteria --tag=@phase1');
  console.log('      fspec show-acceptance-criteria --tag=@phase1 --format=markdown');
  console.log('      fspec show-acceptance-criteria --tag=@critical --format=json --output=acs.json');
  console.log('');

  console.log(chalk.bold('TAG MATCHING'));
  console.log('  - Scenarios inherit feature-level tags');
  console.log('  - Scenarios can have their own scenario-level tags (@smoke, @regression, etc.)');
  console.log('  - Tag matching checks BOTH feature tags AND scenario tags');
  console.log('  - Multiple --tag options use AND logic (all tags must match)');
  console.log('  - Example: Feature @auth + Scenario @smoke matches both @auth and @smoke');
  console.log('');

  console.log(chalk.bold('NOTES'));
  console.log('  - JSON output is ideal for programmatic access and AI agent integration');
  console.log('  - Scenario tags are displayed in output (e.g., [@smoke @critical])');
  console.log('');
}

function displayProjectHelp(): void {
  console.log(chalk.bold('\nPROJECT MANAGEMENT\n'));
  console.log(chalk.dim('Manage work units, epics, and Kanban workflow for ACDD development\n'));

  console.log(chalk.bold('WORK UNIT MANAGEMENT'));
  console.log('  ' + chalk.cyan('fspec create-work-unit <prefix> <title>') + ' Create new work unit');
  console.log('    Options:');
  console.log('      -d, --description <desc>         Work unit description');
  console.log('      -e, --epic <epic>                Epic ID to associate with');
  console.log('      -p, --parent <parent>            Parent work unit ID');
  console.log('    Examples:');
  console.log('      fspec create-work-unit AUTH "User login feature"');
  console.log('      fspec create-work-unit DASH "Dashboard view" -e user-management');
  console.log('');
  console.log('  ' + chalk.cyan('fspec list-work-units') + '                List all work units');
  console.log('    Options:');
  console.log('      -s, --status <status>            Filter by status');
  console.log('      -p, --prefix <prefix>            Filter by prefix');
  console.log('      -e, --epic <epic>                Filter by epic');
  console.log('    Examples:');
  console.log('      fspec list-work-units');
  console.log('      fspec list-work-units -s specifying');
  console.log('      fspec list-work-units -p AUTH -e user-management');
  console.log('');
  console.log('  ' + chalk.cyan('fspec show-work-unit <id>') + '            Display work unit details');
  console.log('    Options:');
  console.log('      -f, --format <format>            Output: text or json (default: text)');
  console.log('    Examples:');
  console.log('      fspec show-work-unit AUTH-001');
  console.log('      fspec show-work-unit AUTH-001 -f json');
  console.log('');

  console.log(chalk.bold('EPIC MANAGEMENT'));
  console.log('  ' + chalk.cyan('fspec create-epic <id> <title>') + '      Create new epic');
  console.log('    Options:');
  console.log('      -d, --description <desc>         Epic description');
  console.log('    Examples:');
  console.log('      fspec create-epic user-management "User Management Features"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec list-epics') + '                     List all epics');
  console.log('    Examples:');
  console.log('      fspec list-epics');
  console.log('');
  console.log('  ' + chalk.cyan('fspec show-epic <id>') + '                 Display epic details');
  console.log('    Options:');
  console.log('      -f, --format <format>            Output: text or json (default: text)');
  console.log('    Examples:');
  console.log('      fspec show-epic user-management');
  console.log('      fspec show-epic user-management -f json');
  console.log('');

  console.log(chalk.bold('WORKFLOW STATES'));
  console.log('  Work units progress through Kanban states:');
  console.log('  backlog → specifying → testing → implementing → validating → done');
  console.log('  (blocked state can occur at any point)');
  console.log('');

  console.log(chalk.bold('NOTES'));
  console.log('  - Work units are stored in spec/work-units.json');
  console.log('  - Epics are stored in spec/epics.json');
  console.log('  - All commands follow ACDD (Acceptance Criteria Driven Development)');
  console.log('  - See spec/features/ for full specification of project management features');
  console.log('');
}

// Custom help command handler
function handleHelpCommand(group?: string): void {
  if (!group) {
    displayCustomHelp();
    return;
  }

  switch (group.toLowerCase()) {
    case 'spec':
    case 'specification':
      displaySpecHelp();
      break;
    case 'tags':
    case 'tag':
      displayTagsHelp();
      break;
    case 'foundation':
    case 'arch':
    case 'architecture':
      displayFoundationHelp();
      break;
    case 'query':
    case 'report':
    case 'reporting':
      displayQueryHelp();
      break;
    case 'project':
    case 'pm':
    case 'work':
      displayProjectHelp();
      break;
    default:
      console.log(chalk.red(`Unknown help topic: ${group}`));
      console.log('Valid topics: spec, tags, foundation, query, project');
      console.log('Use ' + chalk.cyan('fspec --help') + ' for main help\n');
  }
}

program
  .name('fspec')
  .description(
    'Feature Specification & Project Management for AI Agents'
  )
  .version('0.0.1')
  .configureHelp({
    helpWidth: 100
  })
  .addHelpCommand(false)
  .helpOption(false); // Disable default help

// Override help handling completely
program.on('option:help', () => {
  displayCustomHelp();
  process.exit(0);
});

// Add custom help command
program
  .command('help')
  .description('Display help for command groups')
  .argument('[group]', 'Help topic: spec, tags, foundation, query, project')
  .action(handleHelpCommand);

// Handle -h and --help flags manually
const args = process.argv;
if (args.includes('-h') || args.includes('--help')) {
  if (args.length === 3) {
    // Just "fspec --help" or "fspec -h"
    displayCustomHelp();
    process.exit(0);
  }
}

// Show custom help when no command is provided
if (args.length === 2) {
  displayCustomHelp();
  process.exit(0);
}

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

// Add tag to feature command
program
  .command('add-tag-to-feature')
  .description('Add one or more tags to a feature file')
  .argument('<file>', 'Feature file path (e.g., spec/features/login.feature)')
  .argument('<tags...>', 'Tag(s) to add (e.g., @critical @security)')
  .option('--validate-registry', 'Validate tags against spec/tags.json')
  .action(async (file: string, tags: string[], options: { validateRegistry?: boolean }) => {
    await addTagToFeatureCommand(file, tags, options);
  });

// Remove tag from feature command
program
  .command('remove-tag-from-feature')
  .description('Remove one or more tags from a feature file')
  .argument('<file>', 'Feature file path (e.g., spec/features/login.feature)')
  .argument('<tags...>', 'Tag(s) to remove (e.g., @deprecated @wip)')
  .action(async (file: string, tags: string[]) => {
    await removeTagFromFeatureCommand(file, tags);
  });

// List feature tags command
program
  .command('list-feature-tags')
  .description('List all tags on a specific feature file')
  .argument('<file>', 'Feature file path (e.g., spec/features/login.feature)')
  .option('--show-categories', 'Show tag categories from registry')
  .action(async (file: string, options: { showCategories?: boolean }) => {
    await listFeatureTagsCommand(file, options);
  });

// Add tag to scenario command
program
  .command('add-tag-to-scenario')
  .description('Add one or more tags to a specific scenario')
  .argument('<file>', 'Feature file path (e.g., spec/features/login.feature)')
  .argument('<scenario>', 'Scenario name (e.g., "Login with valid credentials")')
  .argument('<tags...>', 'Tag(s) to add (e.g., @smoke @critical)')
  .option('--validate-registry', 'Validate tags against spec/tags.json')
  .action(async (file: string, scenario: string, tags: string[], options: { validateRegistry?: boolean }) => {
    await addTagToScenarioCommand(file, scenario, tags, options);
  });

// Remove tag from scenario command
program
  .command('remove-tag-from-scenario')
  .description('Remove one or more tags from a specific scenario')
  .argument('<file>', 'Feature file path (e.g., spec/features/login.feature)')
  .argument('<scenario>', 'Scenario name (e.g., "Login with valid credentials")')
  .argument('<tags...>', 'Tag(s) to remove (e.g., @wip @deprecated)')
  .action(async (file: string, scenario: string, tags: string[]) => {
    await removeTagFromScenarioCommand(file, scenario, tags);
  });

// List scenario tags command
program
  .command('list-scenario-tags')
  .description('List all tags on a specific scenario')
  .argument('<file>', 'Feature file path (e.g., spec/features/login.feature)')
  .argument('<scenario>', 'Scenario name (e.g., "Login with valid credentials")')
  .option('--show-categories', 'Show tag categories from registry')
  .action(async (file: string, scenario: string, options: { showCategories?: boolean }) => {
    await listScenarioTagsCommand(file, scenario, options);
  });

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

// Delete diagram command
program
  .command('delete-diagram')
  .description('Delete Mermaid diagram from FOUNDATION.md')
  .argument('<section>', 'Section name (e.g., "Architecture Diagrams")')
  .argument('<title>', 'Diagram title to delete')
  .action(deleteDiagramCommand);

// Generate foundation-md command
program
  .command('generate-foundation-md')
  .description('Generate FOUNDATION.md from foundation.json')
  .action(generateFoundationMdCommandCLI);

// Generate tags-md command
program
  .command('generate-tags-md')
  .description('Generate TAGS.md from tags.json')
  .action(generateTagsMdCommandCLI);

// ============================================================================
// PROJECT MANAGEMENT COMMANDS
// ============================================================================

// Create work unit command
program
  .command('create-work-unit')
  .description('Create a new work unit')
  .argument('<prefix>', 'Work unit prefix (e.g., AUTH, DASH)')
  .argument('<title>', 'Work unit title')
  .option('-d, --description <description>', 'Work unit description')
  .option('-e, --epic <epic>', 'Epic ID to associate with')
  .option('-p, --parent <parent>', 'Parent work unit ID')
  .action(createWorkUnitCommand);

// List work units command
program
  .command('list-work-units')
  .description('List all work units')
  .option('-s, --status <status>', 'Filter by status')
  .option('-p, --prefix <prefix>', 'Filter by prefix')
  .option('-e, --epic <epic>', 'Filter by epic')
  .action(listWorkUnitsCommand);

// Show work unit command
program
  .command('show-work-unit')
  .description('Display work unit details')
  .argument('<workUnitId>', 'Work unit ID (e.g., AUTH-001)')
  .option('-f, --format <format>', 'Output format: text or json', 'text')
  .action(showWorkUnitCommand);

// Create epic command
program
  .command('create-epic')
  .description('Create a new epic')
  .argument('<epicId>', 'Epic ID (lowercase-with-hyphens, e.g., user-management)')
  .argument('<title>', 'Epic title')
  .option('-d, --description <description>', 'Epic description')
  .action(createEpicCommand);

// List epics command
program
  .command('list-epics')
  .description('List all epics')
  .action(listEpicsCommand);

// Show epic command
program
  .command('show-epic')
  .description('Display epic details')
  .argument('<epicId>', 'Epic ID')
  .option('-f, --format <format>', 'Output format: text or json', 'text')
  .action(showEpicCommand);

program.parse();
