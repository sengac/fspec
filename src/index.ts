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
import { showFoundationSchemaCommand } from './commands/show-foundation-schema';
import { validateFoundationSchemaCommand } from './commands/validate-foundation-schema';
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
import { createWorkUnitCommand } from './commands/create-work-unit';
import { listWorkUnitsCommand } from './commands/list-work-units';
import { showWorkUnitCommand } from './commands/show-work-unit';
import { createEpicCommand } from './commands/create-epic';
import { listEpicsCommand } from './commands/list-epics';
import { showEpicCommand } from './commands/show-epic';
import { listPrefixes } from './commands/list-prefixes';
// Additional work unit management commands
import { prioritizeWorkUnit } from './commands/prioritize-work-unit';
import { updateWorkUnit } from './commands/update-work-unit';
import { deleteWorkUnit } from './commands/delete-work-unit';
import { updateWorkUnitStatus } from './commands/update-work-unit-status';
import { updateWorkUnitEstimate } from './commands/update-work-unit-estimate';
import { validateWorkUnits } from './commands/validate-work-units';
import { repairWorkUnits } from './commands/repair-work-units';
import { exportWorkUnits } from './commands/export-work-units';
// Dependency management commands
import { addDependency } from './commands/add-dependency';
import { addDependencies } from './commands/add-dependencies';
import { removeDependency } from './commands/remove-dependency';
import { clearDependencies } from './commands/clear-dependencies';
import { exportDependencies } from './commands/export-dependencies';
// Prefix and epic management commands
import { createPrefix } from './commands/create-prefix';
import { updatePrefix } from './commands/update-prefix';
import { deleteEpic } from './commands/delete-epic';
// Example mapping commands
import { addExample } from './commands/add-example';
import { addQuestion } from './commands/add-question';
import { addRule } from './commands/add-rule';
import { removeExample } from './commands/remove-example';
import { removeQuestion } from './commands/remove-question';
import { removeRule } from './commands/remove-rule';
import { answerQuestion } from './commands/answer-question';
import { importExampleMap } from './commands/import-example-map';
import { exportExampleMap } from './commands/export-example-map';
import { generateScenarios } from './commands/generate-scenarios';
// Feature documentation commands
import { addAssumption } from './commands/add-assumption';
import { setUserStoryCommand } from './commands/set-user-story';
// Workflow and automation commands
import { autoAdvance } from './commands/auto-advance';
import { displayBoard } from './commands/display-board';
// Query and metrics commands
import { queryWorkUnits } from './commands/query-work-units';
import { queryDependencyStats } from './commands/query-dependency-stats';
import { queryExampleMappingStats } from './commands/query-example-mapping-stats';
import { queryMetrics } from './commands/query-metrics';
import { queryEstimateAccuracy } from './commands/query-estimate-accuracy';
import { queryEstimationGuide } from './commands/query-estimation-guide';
import { recordMetric } from './commands/record-metric';
import { recordTokens } from './commands/record-tokens';
import { recordIteration } from './commands/record-iteration';
import { generateSummaryReport } from './commands/generate-summary-report';
// Validation commands
import { validateSpecAlignment } from './commands/validate-spec-alignment';
// Dependencies command (display/show)
import { showDependencies } from './commands/dependencies';
// Setup and init commands
import { init } from './commands/init';

const program = new Command();

// Custom help display
export function displayCustomHelpWithNote(): void {
  console.log(
    chalk.bold(
      '\nfspec - Feature Specification & Project Management for AI Agents'
    )
  );
  console.log(chalk.dim('Version 0.0.1\n'));

  console.log(chalk.bold('USAGE'));
  console.log('  fspec [command] [options]\n');

  console.log(chalk.bold('COMMAND GROUPS'));
  console.log(
    '  Use ' +
      chalk.cyan('fspec help <group>') +
      ' for detailed help on a specific area:\n'
  );
  console.log(
    '  ' +
      chalk.cyan('specs') +
      '     - Write and manage Gherkin feature files (create, edit, validate)'
  );
  console.log(
    '  ' +
      chalk.cyan('work') +
      '      - Track work units through ACDD workflow (Kanban, dependencies, board)'
  );
  console.log(
    '  ' +
      chalk.cyan('discovery') +
      ' - Collaborative discovery with example mapping (questions, rules, examples)'
  );
  console.log(
    '  ' +
      chalk.cyan('metrics') +
      '   - Track progress and quality (estimates, metrics, reports, statistics)'
  );
  console.log(
    '  ' +
      chalk.cyan('setup') +
      '     - Configure project structure (tags, epics, prefixes, foundation docs)'
  );
  console.log('');

  console.log(chalk.bold('QUICK START'));
  console.log(
    '  ' +
      chalk.cyan('fspec init') +
      '                  - Initialize /fspec and /rspec slash commands'
  );
  console.log(
    '  ' +
      chalk.cyan('fspec validate') +
      '              - Validate all Gherkin feature files'
  );
  console.log(
    '  ' +
      chalk.cyan('fspec create-feature NAME') +
      '   - Create a new feature file'
  );
  console.log(
    '  ' +
      chalk.cyan('fspec list-features') +
      '         - List all feature files'
  );
  console.log(
    '  ' +
      chalk.cyan('fspec check') +
      '                 - Run all validation checks\n'
  );

  console.log(chalk.bold('GET HELP'));
  console.log(
    '  ' + chalk.cyan('fspec --help') + '            - Show this help'
  );
  console.log(
    '  ' +
      chalk.cyan('fspec <command> --help') +
      '  - Get detailed help for any command'
  );
  console.log(
    '  ' +
      chalk.cyan('fspec help specs') +
      '        - Gherkin feature file commands'
  );
  console.log(
    '  ' +
      chalk.cyan('fspec help work') +
      '         - Work unit and Kanban workflow commands'
  );
  console.log(
    '  ' + chalk.cyan('fspec help discovery') + '    - Example mapping commands'
  );
  console.log(
    '  ' +
      chalk.cyan('fspec help metrics') +
      '      - Progress tracking and reporting commands'
  );
  console.log(
    '  ' +
      chalk.cyan('fspec help setup') +
      '        - Configuration and setup commands\n'
  );

  console.log(chalk.bold('EXAMPLES'));
  console.log('  # Validate specific feature file');
  console.log('  ' + chalk.dim('$ fspec validate spec/features/login.feature'));
  console.log('');
  console.log('  # Filter features by tag');
  console.log('  ' + chalk.dim('$ fspec list-features --tag=@phase1'));
  console.log('');
  console.log('  # Add scenario to feature');
  console.log(
    '  ' +
      chalk.dim(
        '$ fspec add-scenario user-authentication "Login with valid credentials"'
      )
  );
  console.log('');
  console.log('  # Query scenarios by tags');
  console.log(
    '  ' +
      chalk.dim(
        '$ fspec get-scenarios --tag=@phase1 --tag=@critical --format=json'
      )
  );
  console.log('');

  console.log(chalk.bold('DOCUMENTATION'));
  console.log('  GitHub: ' + chalk.cyan('https://github.com/sengac/fspec'));
  console.log(
    '  README: ' + chalk.dim('See README.md for detailed usage examples')
  );
  console.log('');
  process.exit(0);
}

// ===== SPECS HELP =====
function displaySpecsHelp(): void {
  console.log(chalk.bold('\nGHERKIN SPECIFICATIONS'));
  console.log(chalk.dim('Write and manage Gherkin feature files\n'));

  console.log('Use this when you need to:');
  console.log('  • Create new feature files with proper Gherkin structure');
  console.log('  • Add, edit, or delete scenarios and steps');
  console.log('  • Add background stories and architecture notes');
  console.log('  • Validate Gherkin syntax using official Cucumber parser');
  console.log('  • Format feature files for consistency');
  console.log('  • List and show features with filtering by tags');
  console.log('  • Bulk operations on features and scenarios\n');

  console.log(chalk.bold('FEATURES'));
  console.log(
    '  ' +
      chalk.cyan('fspec create-feature <name>') +
      '      Create new feature file'
  );
  console.log('    Examples:');
  console.log('      fspec create-feature "User Authentication"');
  console.log('');
  console.log(
    '  ' +
      chalk.cyan('fspec list-features') +
      '              List all feature files'
  );
  console.log('    Options:');
  console.log('      --tag=<tag>                      Filter by tag');
  console.log('    Examples:');
  console.log('      fspec list-features --tag=@phase1');
  console.log('');
  console.log(
    '  ' +
      chalk.cyan('fspec show-feature <name>') +
      '         Show feature details'
  );
  console.log('    Options:');
  console.log('      --format <format>                Output: text or json');
  console.log('      --output <file>                  Write to file');
  console.log('    Examples:');
  console.log('      fspec show-feature user-authentication');
  console.log('      fspec show-feature login --format=json');
  console.log('');
  console.log(
    '  ' +
      chalk.cyan('fspec delete-features-by-tag') +
      '      Delete features by tag'
  );
  console.log('    Options:');
  console.log('      --tag=<tag>                      Tag to match (required)');
  console.log(
    '      --dry-run                        Preview without deleting'
  );
  console.log('    Examples:');
  console.log('      fspec delete-features-by-tag --tag=@deprecated --dry-run');
  console.log('');

  console.log(chalk.bold('SCENARIOS'));
  console.log(
    '  ' +
      chalk.cyan('fspec add-scenario <feature> <title>') +
      ' Add scenario to feature'
  );
  console.log('    Examples:');
  console.log('      fspec add-scenario login "Login with valid credentials"');
  console.log('');
  console.log(
    '  ' +
      chalk.cyan('fspec update-scenario <feature> <old> <new>') +
      ' Update scenario name'
  );
  console.log('    Examples:');
  console.log('      fspec update-scenario login "Old Name" "New Name"');
  console.log('');
  console.log(
    '  ' +
      chalk.cyan('fspec delete-scenario <feature> <title>') +
      ' Delete scenario'
  );
  console.log('    Examples:');
  console.log('      fspec delete-scenario login "Deprecated scenario"');
  console.log('');
  console.log(
    '  ' +
      chalk.cyan('fspec get-scenarios') +
      '               Query scenarios by tag'
  );
  console.log('    Options:');
  console.log(
    '      --tag=<tag>                      Filter by tag (AND logic)'
  );
  console.log('      --format=<format>                Output: text or json');
  console.log('    Examples:');
  console.log('      fspec get-scenarios --tag=@phase1 --tag=@critical');
  console.log('');
  console.log(
    '  ' +
      chalk.cyan('fspec show-acceptance-criteria') +
      '    Show acceptance criteria'
  );
  console.log('    Options:');
  console.log('      --tag=<tag>                      Filter by tag');
  console.log(
    '      --format=<format>                Output: text, markdown, json'
  );
  console.log('      --output=<file>                  Write to file');
  console.log('    Examples:');
  console.log(
    '      fspec show-acceptance-criteria --tag=@phase1 --format=markdown'
  );
  console.log('');
  console.log(
    '  ' +
      chalk.cyan('fspec delete-scenarios-by-tag') +
      '     Delete scenarios by tag'
  );
  console.log('    Options:');
  console.log('      --tag=<tag>                      Tag to match');
  console.log(
    '      --dry-run                        Preview without deleting'
  );
  console.log('');

  console.log(chalk.bold('STEPS'));
  console.log(
    '  ' + chalk.cyan('fspec add-step <feature> <scenario> <keyword> <text>')
  );
  console.log('    Examples:');
  console.log(
    '      fspec add-step login "Valid login" given "I am on the login page"'
  );
  console.log('');
  console.log(
    '  ' + chalk.cyan('fspec update-step <feature> <scenario> <old-text>')
  );
  console.log('    Options:');
  console.log('      --text <new-text>                New step text');
  console.log(
    '      --keyword <keyword>              New keyword (Given/When/Then/And/But)'
  );
  console.log('    Examples:');
  console.log(
    '      fspec update-step login "Valid" "old step" --text "new step"'
  );
  console.log('');
  console.log(
    '  ' + chalk.cyan('fspec delete-step <feature> <scenario> <text>')
  );
  console.log('    Examples:');
  console.log(
    '      fspec delete-step login "Valid login" "I am on the login page"'
  );
  console.log('');

  console.log(chalk.bold('CONTENT'));
  console.log(
    '  ' +
      chalk.cyan('fspec add-background <feature> <text>') +
      ' Add/update background story'
  );
  console.log('    Examples:');
  console.log(
    '      fspec add-background login "As a user\\nI want to log in"'
  );
  console.log('');
  console.log(
    '  ' +
      chalk.cyan('fspec add-architecture <feature> <text>') +
      ' Add architecture notes'
  );
  console.log('    Examples:');
  console.log('      fspec add-architecture login "Uses JWT tokens"');
  console.log('');

  console.log(chalk.bold('VALIDATION & FORMATTING'));
  console.log(
    '  ' +
      chalk.cyan('fspec validate') +
      '                   Validate Gherkin syntax'
  );
  console.log('    Options:');
  console.log('      --verbose                        Show detailed output');
  console.log('    Examples:');
  console.log('      fspec validate');
  console.log('      fspec validate spec/features/login.feature');
  console.log('');
  console.log(
    '  ' +
      chalk.cyan('fspec format') +
      '                     Format feature files'
  );
  console.log('    Examples:');
  console.log('      fspec format');
  console.log('      fspec format spec/features/login.feature');
  console.log('');
  console.log(
    '  ' +
      chalk.cyan('fspec check') +
      '                      Run all validation checks'
  );
  console.log('    Options:');
  console.log('      --verbose                        Show detailed output');
  console.log('    Examples:');
  console.log('      fspec check --verbose');
  console.log('');
  console.log(
    '  ' +
      chalk.cyan('fspec validate-tags') +
      '              Validate tag usage'
  );
  console.log('    Examples:');
  console.log('      fspec validate-tags');
  console.log('');

  console.log(chalk.bold('TAG MANAGEMENT (Feature/Scenario Level)'));
  console.log('  ' + chalk.cyan('fspec add-tag-to-feature <file> <tag>'));
  console.log('    Options:');
  console.log(
    '      --validate-registry              Check tag exists in registry'
  );
  console.log('    Examples:');
  console.log(
    '      fspec add-tag-to-feature spec/features/login.feature @critical'
  );
  console.log('');
  console.log('  ' + chalk.cyan('fspec remove-tag-from-feature <file> <tag>'));
  console.log('    Examples:');
  console.log(
    '      fspec remove-tag-from-feature spec/features/login.feature @wip'
  );
  console.log('');
  console.log('  ' + chalk.cyan('fspec list-feature-tags <file>'));
  console.log('    Options:');
  console.log('      --show-categories                Show tag categories');
  console.log('');
  console.log(
    '  ' + chalk.cyan('fspec add-tag-to-scenario <file> <scenario> <tag>')
  );
  console.log('    Examples:');
  console.log(
    '      fspec add-tag-to-scenario login.feature "Valid login" @smoke'
  );
  console.log('');
  console.log(
    '  ' + chalk.cyan('fspec remove-tag-from-scenario <file> <scenario> <tag>')
  );
  console.log('  ' + chalk.cyan('fspec list-scenario-tags <file> <scenario>'));
  console.log('');
}

function displayWorkHelp(): void {
  console.log(chalk.bold('\nWORK UNIT MANAGEMENT'));
  console.log(chalk.dim('Track work units through ACDD workflow\n'));

  console.log('Use this when you need to:');
  console.log('  • Create and organize work units into a backlog');
  console.log(
    '  • Move work through Kanban states (backlog → specifying → testing → implementing → validating → done)'
  );
  console.log('  • Prioritize work in the backlog');
  console.log('  • Manage dependencies between work units');
  console.log('  • Block work units when progress is prevented');
  console.log('  • Display the Kanban board showing current state');
  console.log('  • Auto-advance ready work units through workflow');
  console.log('  • Update work unit details (title, description, estimates)\n');

  console.log(chalk.bold('WORK UNITS'));
  console.log('  ' + chalk.cyan('fspec create-work-unit <prefix> <title>'));
  console.log('    Options:');
  console.log('      -e, --epic <epic>                Associate with epic');
  console.log('      --description <desc>             Work unit description');
  console.log('    Examples:');
  console.log('      fspec create-work-unit AUTH "User login feature"');
  console.log(
    '      fspec create-work-unit DASH "Dashboard" -e user-management'
  );
  console.log('');
  console.log('  ' + chalk.cyan('fspec list-work-units'));
  console.log('    Options:');
  console.log('      -s, --status <status>            Filter by status');
  console.log('      --prefix <prefix>                Filter by prefix');
  console.log('      --epic <epic>                    Filter by epic');
  console.log('    Examples:');
  console.log('      fspec list-work-units');
  console.log('      fspec list-work-units -s specifying');
  console.log('');
  console.log('  ' + chalk.cyan('fspec show-work-unit <id>'));
  console.log('    Examples:');
  console.log('      fspec show-work-unit AUTH-001');
  console.log('');
  console.log('  ' + chalk.cyan('fspec update-work-unit <id>'));
  console.log('    Options:');
  console.log('      --title <title>                  Update title');
  console.log('      --description <desc>             Update description');
  console.log('      --epic <epic>                    Update epic');
  console.log('      --parent <parent-id>             Set parent work unit');
  console.log('    Examples:');
  console.log('      fspec update-work-unit AUTH-001 --title "New Title"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec update-work-unit-status <id> <status>'));
  console.log('    Examples:');
  console.log('      fspec update-work-unit-status AUTH-001 specifying');
  console.log('      fspec update-work-unit-status AUTH-001 implementing');
  console.log('');
  console.log(
    '  ' + chalk.cyan('fspec update-work-unit-estimate <id> <estimate>')
  );
  console.log('    Examples:');
  console.log('      fspec update-work-unit-estimate AUTH-001 5');
  console.log('');
  console.log('  ' + chalk.cyan('fspec prioritize-work-unit <id>'));
  console.log('    Options:');
  console.log(
    '      --position <position>            Position: top, bottom, or number'
  );
  console.log(
    '      --before <id>                    Place before this work unit'
  );
  console.log(
    '      --after <id>                     Place after this work unit'
  );
  console.log('    Examples:');
  console.log('      fspec prioritize-work-unit AUTH-003 --position=top');
  console.log('      fspec prioritize-work-unit AUTH-001 --before=AUTH-002');
  console.log('');
  console.log('  ' + chalk.cyan('fspec delete-work-unit <id>'));
  console.log('    Examples:');
  console.log('      fspec delete-work-unit AUTH-001');
  console.log('');
  console.log('  ' + chalk.cyan('fspec repair-work-units'));
  console.log('    Description: Fix work unit state inconsistencies');
  console.log('    Examples:');
  console.log('      fspec repair-work-units');
  console.log('');
  console.log('  ' + chalk.cyan('fspec validate-work-units'));
  console.log('    Description: Validate work units against schema');
  console.log('    Examples:');
  console.log('      fspec validate-work-units');
  console.log('');
  console.log('  ' + chalk.cyan('fspec export-work-units'));
  console.log('    Options:');
  console.log('      --format <format>                Output: json or csv');
  console.log('      --output <file>                  Output file path');
  console.log('    Examples:');
  console.log('      fspec export-work-units --format=json --output=work.json');
  console.log('');
  console.log('  ' + chalk.cyan('fspec query-work-units'));
  console.log('    Options:');
  console.log('      --status <status>                Filter by status');
  console.log('      --prefix <prefix>                Filter by prefix');
  console.log('      --epic <epic>                    Filter by epic');
  console.log('      --format <format>                Output: text or json');
  console.log('    Examples:');
  console.log(
    '      fspec query-work-units --status=implementing --format=json'
  );
  console.log('');

  console.log(chalk.bold('DEPENDENCIES'));
  console.log('  ' + chalk.cyan('fspec add-dependency <id> [depends-on-id]'));
  console.log(
    '    Description: Add dependency relationship (shorthand or options)'
  );
  console.log('    Options:');
  console.log(
    '      --blocks <id>                    Work unit that this blocks'
  );
  console.log(
    '      --blocked-by <id>                Work unit that blocks this'
  );
  console.log(
    '      --depends-on <id>                Work unit this depends on'
  );
  console.log('      --relates-to <id>                Related work unit');
  console.log('    Examples:');
  console.log(
    '      fspec add-dependency AUTH-002 AUTH-001              # Shorthand: AUTH-002 depends on AUTH-001'
  );
  console.log(
    '      fspec add-dependency AUTH-002 --blocks=API-001      # AUTH-002 blocks API-001'
  );
  console.log(
    '      fspec add-dependency UI-001 --blocked-by=API-001    # UI-001 is blocked by API-001'
  );
  console.log('');
  console.log(
    '  ' + chalk.cyan('fspec add-dependencies <id> <dep1> <dep2>...')
  );
  console.log('    Examples:');
  console.log('      fspec add-dependencies DASH-001 AUTH-001 AUTH-002');
  console.log('');
  console.log(
    '  ' + chalk.cyan('fspec remove-dependency <id> [depends-on-id]')
  );
  console.log('    Options:');
  console.log(
    '      --blocks <id>                    Remove blocks relationship'
  );
  console.log(
    '      --blocked-by <id>                Remove blockedBy relationship'
  );
  console.log(
    '      --depends-on <id>                Remove dependsOn relationship'
  );
  console.log(
    '      --relates-to <id>                Remove relatesTo relationship'
  );
  console.log('    Examples:');
  console.log(
    '      fspec remove-dependency AUTH-002 AUTH-001           # Remove dependsOn'
  );
  console.log(
    '      fspec remove-dependency AUTH-002 --blocks=API-001   # Remove blocks'
  );
  console.log('');
  console.log('  ' + chalk.cyan('fspec clear-dependencies <id>'));
  console.log('    Examples:');
  console.log('      fspec clear-dependencies AUTH-002');
  console.log('');
  console.log('  ' + chalk.cyan('fspec dependencies <id>'));
  console.log('    Description: Show dependencies for a work unit');
  console.log('    Examples:');
  console.log('      fspec dependencies AUTH-002');
  console.log('');
  console.log('  ' + chalk.cyan('fspec export-dependencies'));
  console.log('    Options:');
  console.log('      --format <format>                Output: json or mermaid');
  console.log('      --output <file>                  Output file path');
  console.log('    Examples:');
  console.log('      fspec export-dependencies --format=mermaid');
  console.log('');
  console.log('  ' + chalk.cyan('fspec query-dependency-stats'));
  console.log('    Options:');
  console.log('      --format <format>                Output: text or json');
  console.log('    Examples:');
  console.log('      fspec query-dependency-stats');
  console.log('');

  console.log(chalk.bold('WORKFLOW AUTOMATION'));
  console.log('  ' + chalk.cyan('fspec auto-advance'));
  console.log('    Description: Automatically advance ready work units');
  console.log('    Options:');
  console.log(
    '      --dry-run                        Show what would be advanced'
  );
  console.log('    Examples:');
  console.log('      fspec auto-advance --dry-run');
  console.log('      fspec auto-advance');
  console.log('');
  console.log('  ' + chalk.cyan('fspec board'));
  console.log('    Description: Display Kanban board of all work');
  console.log('    Options:');
  console.log(
    '      --format <format>                Output: text or json (default: text)'
  );
  console.log(
    '      --limit <limit>                  Max items per column (default: 25)'
  );
  console.log('    Examples:');
  console.log('      fspec board');
  console.log('      fspec board --format=json');
  console.log('      fspec board --limit=50');
  console.log('');
  console.log('  ' + chalk.cyan('fspec workflow-automation <id>'));
  console.log('    Description: Check if work unit can auto-advance');
  console.log('    Examples:');
  console.log('      fspec workflow-automation AUTH-001');
  console.log('');
}

// ===== DISCOVERY HELP =====
function displayDiscoveryHelp(): void {
  console.log(chalk.bold('\nEXAMPLE MAPPING'));
  console.log(chalk.dim('Collaborative discovery with example mapping\n'));

  console.log('Use this when you need to:');
  console.log('  • Add concrete examples to explore behavior');
  console.log('  • Add questions that need answers before implementation');
  console.log('  • Add business rules that govern the feature');
  console.log('  • Add assumptions about requirements or constraints');
  console.log('  • Answer questions collaboratively (AI asks, human answers)');
  console.log('  • Generate Gherkin scenarios from examples');
  console.log('  • Import/export example maps for collaboration');
  console.log('  • Query example mapping coverage statistics\n');

  console.log(chalk.bold('EXAMPLES'));
  console.log('  ' + chalk.cyan('fspec add-example <work-unit-id> <example>'));
  console.log('    Description: Add example to work unit during specification');
  console.log('    Examples:');
  console.log(
    '      fspec add-example AUTH-001 "User logs in with valid email"'
  );
  console.log('');
  console.log('  ' + chalk.cyan('fspec remove-example <work-unit-id> <index>'));
  console.log('    Description: Remove example by index (0-based)');
  console.log('    Examples:');
  console.log('      fspec remove-example AUTH-001 0');
  console.log('');

  console.log(chalk.bold('QUESTIONS'));
  console.log(
    '  ' + chalk.cyan('fspec add-question <work-unit-id> <question>')
  );
  console.log(
    '    Description: Add question to work unit during specification'
  );
  console.log('    Examples:');
  console.log('      fspec add-question AUTH-001 "Should we support OAuth?"');
  console.log('');
  console.log(
    '  ' + chalk.cyan('fspec answer-question <work-unit-id> <index>')
  );
  console.log('    Options:');
  console.log('      --answer <answer>                Answer text');
  console.log(
    '      --add-to <type>                  Add to: rule, assumption, or none (default: none)'
  );
  console.log('    Examples:');
  console.log(
    '      fspec answer-question AUTH-001 0 --answer "Yes" --add-to rule'
  );
  console.log('');
  console.log(
    '  ' + chalk.cyan('fspec remove-question <work-unit-id> <index>')
  );
  console.log('    Description: Remove question by index (0-based)');
  console.log('    Examples:');
  console.log('      fspec remove-question AUTH-001 0');
  console.log('');

  console.log(chalk.bold('RULES'));
  console.log('  ' + chalk.cyan('fspec add-rule <work-unit-id> <rule>'));
  console.log(
    '    Description: Add business rule to work unit during specification'
  );
  console.log('    Examples:');
  console.log('      fspec add-rule AUTH-001 "Password must be 8+ characters"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec remove-rule <work-unit-id> <index>'));
  console.log('    Description: Remove rule by index (0-based)');
  console.log('    Examples:');
  console.log('      fspec remove-rule AUTH-001 0');
  console.log('');

  console.log(chalk.bold('ASSUMPTIONS'));
  console.log(
    '  ' + chalk.cyan('fspec add-assumption <work-unit-id> <assumption>')
  );
  console.log(
    '    Description: Add assumption to work unit during specification'
  );
  console.log('    Examples:');
  console.log(
    '      fspec add-assumption AUTH-001 "Email verification is handled externally"'
  );
  console.log('');

  console.log(chalk.bold('IMPORT/EXPORT'));
  console.log(
    '  ' + chalk.cyan('fspec import-example-map <work-unit-id> <file>')
  );
  console.log('    Description: Import examples, rules, questions from JSON');
  console.log('    Examples:');
  console.log('      fspec import-example-map AUTH-001 examples.json');
  console.log('');
  console.log(
    '  ' + chalk.cyan('fspec export-example-map <work-unit-id> <file>')
  );
  console.log('    Description: Export examples, rules, questions to JSON');
  console.log('    Examples:');
  console.log('      fspec export-example-map AUTH-001 examples.json');
  console.log('');

  console.log(chalk.bold('GENERATION'));
  console.log('  ' + chalk.cyan('fspec generate-scenarios <work-unit-id>'));
  console.log(
    '    Description: Generate Gherkin scenarios from example mapping data'
  );
  console.log('    Examples:');
  console.log('      fspec generate-scenarios AUTH-001');
  console.log('');

  console.log(chalk.bold('STATISTICS'));
  console.log('  ' + chalk.cyan('fspec query-example-mapping-stats'));
  console.log('    Description: Show example mapping coverage');
  console.log('    Examples:');
  console.log('      fspec query-example-mapping-stats');
  console.log('');
}

// ===== METRICS HELP =====
function displayMetricsHelp(): void {
  console.log(chalk.bold('\nPROGRESS TRACKING & METRICS'));
  console.log(chalk.dim('Track progress and quality\n'));

  console.log('Use this when you need to:');
  console.log('  • Record time spent, tokens used, iterations completed');
  console.log('  • Update and track work unit estimates');
  console.log('  • Query estimation accuracy (actual vs estimated)');
  console.log('  • Get estimation guidance based on historical data');
  console.log('  • Query project metrics and trends');
  console.log('  • Generate comprehensive summary reports');
  console.log('  • Export work units for external analysis');
  console.log('  • Query dependency statistics and bottlenecks\n');

  console.log(chalk.bold('RECORDING METRICS'));
  console.log('  ' + chalk.cyan('fspec record-metric <id> <name> <value>'));
  console.log('    Examples:');
  console.log('      fspec record-metric AUTH-001 time-spent 120');
  console.log('');
  console.log('  ' + chalk.cyan('fspec record-tokens <id> <tokens>'));
  console.log('    Examples:');
  console.log('      fspec record-tokens AUTH-001 15000');
  console.log('');
  console.log('  ' + chalk.cyan('fspec record-iteration <id>'));
  console.log('    Description: Increment iteration count');
  console.log('    Examples:');
  console.log('      fspec record-iteration AUTH-001');
  console.log('');

  console.log(chalk.bold('QUERYING METRICS'));
  console.log('  ' + chalk.cyan('fspec query-metrics'));
  console.log('    Options:');
  console.log(
    '      --metric <metric>                Specific metric to query'
  );
  console.log(
    '      --format <format>                Output: text or json (default: text)'
  );
  console.log('    Examples:');
  console.log('      fspec query-metrics --format=json');
  console.log('');
  console.log('  ' + chalk.cyan('fspec query-estimate-accuracy'));
  console.log('    Description: Show estimation accuracy');
  console.log('    Examples:');
  console.log('      fspec query-estimate-accuracy');
  console.log('');
  console.log('  ' + chalk.cyan('fspec query-estimation-guide <id>'));
  console.log('    Description: Get estimation guidance');
  console.log('    Examples:');
  console.log('      fspec query-estimation-guide AUTH-001');
  console.log('');

  console.log(chalk.bold('REPORTING'));
  console.log('  ' + chalk.cyan('fspec generate-summary-report'));
  console.log('    Description: Generate comprehensive project report');
  console.log('    Options:');
  console.log(
    '      --format <format>                Output: markdown or json (default: markdown)'
  );
  console.log('      --output <file>                  Output file path');
  console.log('    Examples:');
  console.log(
    '      fspec generate-summary-report --format=markdown --output=report.md'
  );
  console.log('');
}

// ===== SETUP HELP =====
function displaySetupHelp(): void {
  console.log(chalk.bold('\nCONFIGURATION & SETUP'));
  console.log(chalk.dim('Configure project structure\n'));

  console.log('Use this when you need to:');
  console.log('  • Register tags in the centralized registry');
  console.log('  • Update or delete tags across all files');
  console.log('  • Bulk rename tags (retag operations)');
  console.log('  • Create and manage epics (high-level initiatives)');
  console.log('  • Create and manage prefixes (namespaces for work unit IDs)');
  console.log('  • Add and validate Mermaid architecture diagrams');
  console.log('  • Update foundation documentation');
  console.log('  • Manage feature-level and scenario-level tags');
  console.log('  • Validate tag usage across the project\n');

  console.log(chalk.bold('TAG REGISTRY'));
  console.log(
    '  ' + chalk.cyan('fspec register-tag <tag> <category> <description>')
  );
  console.log('    Examples:');
  console.log(
    '      fspec register-tag @performance "Technical Tags" "Performance-critical"'
  );
  console.log('');
  console.log('  ' + chalk.cyan('fspec update-tag <tag>'));
  console.log('    Options:');
  console.log('      --description <desc>             New description');
  console.log('      --category <category>            New category');
  console.log('    Examples:');
  console.log(
    '      fspec update-tag @performance --description="New description"'
  );
  console.log('');
  console.log('  ' + chalk.cyan('fspec delete-tag <tag>'));
  console.log('    Options:');
  console.log('      --force                          Delete even if used');
  console.log('      --dry-run                        Preview deletion');
  console.log('    Examples:');
  console.log('      fspec delete-tag @deprecated --dry-run');
  console.log('');
  console.log('  ' + chalk.cyan('fspec list-tags'));
  console.log('    Options:');
  console.log('      --category <category>            Filter by category');
  console.log('    Examples:');
  console.log('      fspec list-tags --category="Technical Tags"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec tag-stats'));
  console.log('    Description: Show tag usage statistics');
  console.log('    Examples:');
  console.log('      fspec tag-stats');
  console.log('');
  console.log('  ' + chalk.cyan('fspec retag'));
  console.log('    Options:');
  console.log('      --from=<tag>                     Old tag');
  console.log('      --to=<tag>                       New tag');
  console.log('      --dry-run                        Preview changes');
  console.log('    Examples:');
  console.log('      fspec retag --from=@old-tag --to=@new-tag --dry-run');
  console.log('');

  console.log(chalk.bold('EPICS & PREFIXES'));
  console.log('  ' + chalk.cyan('fspec create-epic <name> <title>'));
  console.log('    Examples:');
  console.log(
    '      fspec create-epic user-management "User Management Features"'
  );
  console.log('');
  console.log('  ' + chalk.cyan('fspec list-epics'));
  console.log('    Examples:');
  console.log('      fspec list-epics');
  console.log('');
  console.log('  ' + chalk.cyan('fspec show-epic <name>'));
  console.log('    Examples:');
  console.log('      fspec show-epic user-management');
  console.log('');
  console.log('  ' + chalk.cyan('fspec delete-epic <name>'));
  console.log('    Examples:');
  console.log('      fspec delete-epic old-epic');
  console.log('');
  console.log('  ' + chalk.cyan('fspec create-prefix <prefix> <description>'));
  console.log('    Examples:');
  console.log('      fspec create-prefix AUTH "Authentication features"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec update-prefix <prefix> <description>'));
  console.log('    Examples:');
  console.log('      fspec update-prefix AUTH "Updated description"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec list-prefixes'));
  console.log('    Examples:');
  console.log('      fspec list-prefixes');
  console.log('');

  console.log(chalk.bold('FOUNDATION & DOCUMENTATION'));
  console.log(
    '  ' + chalk.cyan('fspec add-diagram <section> <title> <content>')
  );
  console.log('    Description: Add Mermaid diagram with validation');
  console.log('    Examples:');
  console.log(
    '      fspec add-diagram "Architecture" "System" "graph TD\\n  A-->B"'
  );
  console.log('');
  console.log(
    '  ' + chalk.cyan('fspec update-diagram <section> <title> <content>')
  );
  console.log('    Examples:');
  console.log(
    '      fspec update-diagram "Architecture" "System" "graph TB\\n  A-->B"'
  );
  console.log('');
  console.log('  ' + chalk.cyan('fspec delete-diagram <section> <title>'));
  console.log('    Examples:');
  console.log('      fspec delete-diagram "Architecture" "System"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec list-diagrams'));
  console.log('    Examples:');
  console.log('      fspec list-diagrams');
  console.log('');
  console.log('  ' + chalk.cyan('fspec show-diagram <section> <title>'));
  console.log('    Examples:');
  console.log('      fspec show-diagram "Architecture" "System"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec update-foundation <section> <content>'));
  console.log('    Examples:');
  console.log(
    '      fspec update-foundation "What We Are Building" "A CLI tool..."'
  );
  console.log('');
  console.log('  ' + chalk.cyan('fspec show-foundation'));
  console.log('    Options:');
  console.log('      --section <section>              Show specific section');
  console.log(
    '      --format <format>                Output: text, json, or markdown'
  );
  console.log('      --output <file>                  Write to file');
  console.log('      --list-sections                  List all sections');
  console.log('      --line-numbers                   Show line numbers');
  console.log('    Examples:');
  console.log('      fspec show-foundation --section "What We Are Building"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec generate-foundation-md'));
  console.log('    Description: Regenerate FOUNDATION.md from foundation.json');
  console.log('    Examples:');
  console.log('      fspec generate-foundation-md');
  console.log('');
  console.log('  ' + chalk.cyan('fspec generate-tags-md'));
  console.log('    Description: Regenerate TAGS.md from tags.json');
  console.log('    Examples:');
  console.log('      fspec generate-tags-md');
  console.log('');
  console.log('  ' + chalk.cyan('fspec validate-json-schema'));
  console.log('    Description: Validate JSON schema compliance');
  console.log('    Examples:');
  console.log('      fspec validate-json-schema');
  console.log('');
}

// Custom help command handler
function handleHelpCommand(group?: string): void {
  if (!group) {
    displayCustomHelpWithNote();
    return;
  }

  switch (group.toLowerCase()) {
    case 'specs':
    case 'spec':
    case 'features':
    case 'gherkin':
      displaySpecsHelp();
      break;
    case 'work':
    case 'workflow':
    case 'kanban':
    case 'units':
      displayWorkHelp();
      break;
    case 'discovery':
    case 'example-mapping':
    case 'examples':
      displayDiscoveryHelp();
      break;
    case 'metrics':
    case 'reports':
    case 'reporting':
    case 'stats':
      displayMetricsHelp();
      break;
    case 'setup':
    case 'config':
    case 'configuration':
    case 'tags':
    case 'foundation':
      displaySetupHelp();
      break;
    default:
      console.log(chalk.red(`Unknown help topic: ${group}`));
      console.log('Valid topics: specs, work, discovery, metrics, setup');
      console.log('Use ' + chalk.cyan('fspec --help') + ' for main help\n');
  }
}

program
  .name('fspec')
  .description('Feature Specification & Project Management for AI Agents')
  .version('0.0.1')
  .configureHelp({
    helpWidth: 100,
  })
  .addHelpCommand(false)
  .helpOption('-h, --help', 'Display help for command'); // Enable help option for all commands

// Add custom help command
program
  .command('help')
  .description('Display help for command groups')
  .argument('[group]', 'Help topic: spec, tags, foundation, query, project')
  .action(handleHelpCommand);

// Note: Help handling moved to async main() function with help interceptor
// Manual help handling removed to allow help interceptor to work properly

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
  .action(
    async (
      file: string,
      tags: string[],
      options: { validateRegistry?: boolean }
    ) => {
      await addTagToFeatureCommand(file, tags, options);
    }
  );

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
  .argument(
    '<scenario>',
    'Scenario name (e.g., "Login with valid credentials")'
  )
  .argument('<tags...>', 'Tag(s) to add (e.g., @smoke @critical)')
  .option('--validate-registry', 'Validate tags against spec/tags.json')
  .action(
    async (
      file: string,
      scenario: string,
      tags: string[],
      options: { validateRegistry?: boolean }
    ) => {
      await addTagToScenarioCommand(file, scenario, tags, options);
    }
  );

// Remove tag from scenario command
program
  .command('remove-tag-from-scenario')
  .description('Remove one or more tags from a specific scenario')
  .argument('<file>', 'Feature file path (e.g., spec/features/login.feature)')
  .argument(
    '<scenario>',
    'Scenario name (e.g., "Login with valid credentials")'
  )
  .argument('<tags...>', 'Tag(s) to remove (e.g., @wip @deprecated)')
  .action(async (file: string, scenario: string, tags: string[]) => {
    await removeTagFromScenarioCommand(file, scenario, tags);
  });

// List scenario tags command
program
  .command('list-scenario-tags')
  .description('List all tags on a specific scenario')
  .argument('<file>', 'Feature file path (e.g., spec/features/login.feature)')
  .argument(
    '<scenario>',
    'Scenario name (e.g., "Login with valid credentials")'
  )
  .option('--show-categories', 'Show tag categories from registry')
  .action(
    async (
      file: string,
      scenario: string,
      options: { showCategories?: boolean }
    ) => {
      await listScenarioTagsCommand(file, scenario, options);
    }
  );

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

// Show foundation schema command
program
  .command('show-foundation-schema')
  .description('Display foundation.json JSON Schema with guidance for AI agents')
  .action(showFoundationSchemaCommand);

// Validate foundation schema command
program
  .command('validate-foundation-schema')
  .description('Validate foundation.json against JSON Schema')
  .action(validateFoundationSchemaCommand);

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
  .action(async (options: any) => {
    await listWorkUnitsCommand(options);
  });

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
  .argument(
    '<epicId>',
    'Epic ID (lowercase-with-hyphens, e.g., user-management)'
  )
  .argument('<title>', 'Epic title')
  .option('-d, --description <description>', 'Epic description')
  .action(async (epicId: string, title: string, options?: any) => {
    await createEpicCommand(epicId, title, options);
  });

// List epics command
program
  .command('list-epics')
  .description('List all epics')
  .action(listEpicsCommand);

// List prefixes command
program
  .command('list-prefixes')
  .description('List all prefixes')
  .action(async () => {
    try {
      const result = await listPrefixes({});
      if (result.prefixes.length === 0) {
        console.log(chalk.yellow('No prefixes found'));
        process.exit(0);
      }
      console.log(chalk.bold(`\nPrefixes (${result.prefixes.length})`));
      console.log('');
      for (const prefix of result.prefixes) {
        console.log(chalk.cyan(prefix.prefix));
        console.log(chalk.gray(`  ${prefix.description}`));
        if (prefix.totalWorkUnits > 0) {
          console.log(
            chalk.gray(
              `  Work Units: ${prefix.completedWorkUnits}/${prefix.totalWorkUnits} (${prefix.completionPercentage}%)`
            )
          );
        }
        console.log('');
      }
      process.exit(0);
    } catch (error: unknown) {
      if (error instanceof Error) {
        console.error(chalk.red('Error:'), error.message);
      } else {
        console.error(chalk.red('Error: Unknown error occurred'));
      }
      process.exit(1);
    }
  });

// Show epic command
program
  .command('show-epic')
  .description('Display epic details')
  .argument('<epicId>', 'Epic ID')
  .option('-f, --format <format>', 'Output format: text or json', 'text')
  .action(showEpicCommand);

// ============================================================================
// ADDITIONAL WORK UNIT MANAGEMENT COMMANDS
// ============================================================================

// Prioritize work unit command
program
  .command('prioritize-work-unit')
  .description('Change the priority order of a work unit in the backlog')
  .argument('<workUnitId>', 'Work unit ID to prioritize')
  .option('--position <position>', 'Position: top, bottom, or numeric index')
  .option('--before <workUnitId>', 'Place before this work unit')
  .option('--after <workUnitId>', 'Place after this work unit')
  .action(
    async (
      workUnitId: string,
      options: { position?: string; before?: string; after?: string }
    ) => {
      try {
        const parsedPosition =
          options.position === 'top'
            ? 'top'
            : options.position === 'bottom'
              ? 'bottom'
              : options.position
                ? parseInt(options.position, 10)
                : undefined;

        await prioritizeWorkUnit({
          workUnitId,
          position: parsedPosition as 'top' | 'bottom' | number | undefined,
          before: options.before,
          after: options.after,
        });
        console.log(
          chalk.green(`✓ Work unit ${workUnitId} prioritized successfully`)
        );
      } catch (error: any) {
        console.error(
          chalk.red('✗ Failed to prioritize work unit:'),
          error.message
        );
        process.exit(1);
      }
    }
  );

// Update work unit command
program
  .command('update-work-unit')
  .description('Update work unit properties')
  .argument('<workUnitId>', 'Work unit ID to update')
  .option('-t, --title <title>', 'New title')
  .option('-d, --description <description>', 'New description')
  .option('-e, --epic <epic>', 'Epic ID')
  .option('-p, --parent <parent>', 'Parent work unit ID')
  .action(
    async (
      workUnitId: string,
      options: {
        title?: string;
        description?: string;
        epic?: string;
        parent?: string;
      }
    ) => {
      try {
        await updateWorkUnit({
          workUnitId,
          ...options,
        });
        console.log(
          chalk.green(`✓ Work unit ${workUnitId} updated successfully`)
        );
      } catch (error: any) {
        console.error(
          chalk.red('✗ Failed to update work unit:'),
          error.message
        );
        process.exit(1);
      }
    }
  );

// Delete work unit command
program
  .command('delete-work-unit')
  .description('Delete a work unit')
  .argument('<workUnitId>', 'Work unit ID to delete')
  .option('--force', 'Force deletion without checks')
  .option('--skip-confirmation', 'Skip confirmation prompt')
  .option('--cascade-dependencies', 'Remove all dependencies before deleting')
  .action(
    async (
      workUnitId: string,
      options: {
        force?: boolean;
        skipConfirmation?: boolean;
        cascadeDependencies?: boolean;
      }
    ) => {
      try {
        const result = await deleteWorkUnit({
          workUnitId,
          ...options,
        });
        console.log(
          chalk.green(`✓ Work unit ${workUnitId} deleted successfully`)
        );
        if (result.warnings && result.warnings.length > 0) {
          result.warnings.forEach((warning: string) =>
            console.log(chalk.yellow(`⚠ ${warning}`))
          );
        }
      } catch (error: any) {
        console.error(
          chalk.red('✗ Failed to delete work unit:'),
          error.message
        );
        process.exit(1);
      }
    }
  );

// Update work unit status command
program
  .command('update-work-unit-status')
  .description('Update work unit status (follows ACDD workflow)')
  .argument('[workUnitId]', 'Work unit ID')
  .argument(
    '[status]',
    'New status: backlog, specifying, testing, implementing, validating, done, blocked'
  )
  .option(
    '--blocked-reason <reason>',
    'Reason for blocked status (required if status is blocked)'
  )
  .option('--reason <reason>', 'Reason for status change')
  .action(
    async (
      workUnitId: string,
      status: string,
      options: { blockedReason?: string; reason?: string }
    ) => {
      try {
        const result = await updateWorkUnitStatus({
          workUnitId,
          status: status as
            | 'backlog'
            | 'specifying'
            | 'testing'
            | 'implementing'
            | 'validating'
            | 'done'
            | 'blocked',
          blockedReason: options.blockedReason,
          reason: options.reason,
        });
        console.log(
          chalk.green(`✓ Work unit ${workUnitId} status updated to ${status}`)
        );
        if (result.warnings && result.warnings.length > 0) {
          result.warnings.forEach((warning: string) =>
            console.log(chalk.yellow(`⚠ ${warning}`))
          );
        }
        // Output system reminder (visible to AI, invisible to users)
        if (result.systemReminder) {
          console.log('\n' + result.systemReminder);
        }
      } catch (error: any) {
        console.error(
          chalk.red('✗ Failed to update work unit status:'),
          error.message
        );
        process.exit(1);
      }
    }
  );

// Update work unit estimate command
program
  .command('update-work-unit-estimate')
  .description('Update work unit estimate (Fibonacci: 1,2,3,5,8,13,21)')
  .argument('<workUnitId>', 'Work unit ID')
  .argument('<estimate>', 'Story points estimate (Fibonacci number)')
  .action(async (workUnitId: string, estimate: string) => {
    try {
      await updateWorkUnitEstimate({
        workUnitId,
        estimate: parseInt(estimate, 10),
      });
      console.log(
        chalk.green(`✓ Work unit ${workUnitId} estimate set to ${estimate}`)
      );
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to update estimate:'), error.message);
      process.exit(1);
    }
  });

// Validate work units command
program
  .command('validate-work-units')
  .description('Validate work units data integrity')
  .action(async () => {
    try {
      const result = await validateWorkUnits({});
      if (result.valid) {
        console.log(chalk.green(`✓ All work units are valid`));
      } else {
        console.error(
          chalk.red(`✗ Found ${result.errors.length} validation errors`)
        );
        result.errors.forEach((error: string) =>
          console.error(chalk.red(`  - ${error}`))
        );
        process.exit(1);
      }
    } catch (error: any) {
      console.error(
        chalk.red('✗ Failed to validate work units:'),
        error.message
      );
      process.exit(1);
    }
  });

// Repair work units command
program
  .command('repair-work-units')
  .description('Repair work units data integrity issues')
  .option('--dry-run', 'Show what would be repaired without making changes')
  .action(async (options: { dryRun?: boolean }) => {
    try {
      const result = await repairWorkUnits({
        dryRun: options.dryRun,
      });
      console.log(chalk.green(`✓ Repaired ${result.repaired} issues`));
      if (result.details && result.details.length > 0) {
        result.details.forEach((detail: string) =>
          console.log(chalk.cyan(`  - ${detail}`))
        );
      }
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to repair work units:'), error.message);
      process.exit(1);
    }
  });

// Export work units command
program
  .command('export-work-units')
  .description('Export work units to JSON or CSV')
  .argument('<format>', 'Output format: json or csv')
  .argument('<output>', 'Output file path')
  .option('--status <status>', 'Filter by status')
  .action(
    async (format: string, output: string, options: { status?: string }) => {
      try {
        const result = await exportWorkUnits({
          format: format as 'json' | 'csv',
          output,
          status: options.status as any,
        });
        console.log(
          chalk.green(
            `✓ Exported ${result.count} work units to ${result.outputFile}`
          )
        );
      } catch (error: any) {
        console.error(
          chalk.red('✗ Failed to export work units:'),
          error.message
        );
        process.exit(1);
      }
    }
  );

// ============================================================================
// DEPENDENCY MANAGEMENT COMMANDS
// ============================================================================

// Add dependency command
program
  .command('add-dependency')
  .description('Add a dependency relationship between work units')
  .argument('[workUnitId]', 'Work unit ID')
  .argument(
    '[dependsOnId]',
    'Work unit ID that this depends on (shorthand for --depends-on)'
  )
  .option('--blocks <targetId>', 'Work unit that this blocks')
  .option('--blocked-by <targetId>', 'Work unit that blocks this')
  .option(
    '--depends-on <targetId>',
    'Work unit this depends on (soft dependency)'
  )
  .option('--relates-to <targetId>', 'Related work unit')
  .action(
    async (
      workUnitId: string,
      dependsOnId: string | undefined,
      options: {
        blocks?: string;
        blockedBy?: string;
        dependsOn?: string;
        relatesTo?: string;
      }
    ) => {
      try {
        // If second argument provided, use it as --depends-on (shorthand syntax)
        const finalDependsOn = dependsOnId || options.dependsOn;

        // Check if user provided both shorthand and option (conflict)
        if (
          dependsOnId &&
          options.dependsOn &&
          dependsOnId !== options.dependsOn
        ) {
          throw new Error(
            'Cannot specify dependency both as argument and --depends-on option'
          );
        }

        // Require at least one relationship type
        if (
          !finalDependsOn &&
          !options.blocks &&
          !options.blockedBy &&
          !options.relatesTo
        ) {
          throw new Error(
            'Must specify at least one relationship: <depends-on-id> or --blocks/--blocked-by/--depends-on/--relates-to'
          );
        }

        await addDependency({
          workUnitId,
          blocks: options.blocks,
          blockedBy: options.blockedBy,
          dependsOn: finalDependsOn,
          relatesTo: options.relatesTo,
        });
        console.log(chalk.green(`✓ Dependency added successfully`));
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to add dependency:'), error.message);
        process.exit(1);
      }
    }
  );

// Add dependencies (plural) command
program
  .command('add-dependencies')
  .description('Add multiple dependency relationships at once')
  .argument('<workUnitId>', 'Work unit ID')
  .option('--blocks <ids...>', 'Work unit IDs that this blocks')
  .option('--blocked-by <ids...>', 'Work unit IDs that block this')
  .option('--depends-on <ids...>', 'Work unit IDs this depends on')
  .option('--relates-to <ids...>', 'Related work unit IDs')
  .action(
    async (
      workUnitId: string,
      options: {
        blocks?: string[];
        blockedBy?: string[];
        dependsOn?: string[];
        relatesTo?: string[];
      }
    ) => {
      try {
        const result = await addDependencies({
          workUnitId,
          dependencies: {
            blocks: options.blocks,
            blockedBy: options.blockedBy,
            dependsOn: options.dependsOn,
            relatesTo: options.relatesTo,
          },
        });
        console.log(
          chalk.green(`✓ Added ${result.added} dependencies successfully`)
        );
      } catch (error: any) {
        console.error(
          chalk.red('✗ Failed to add dependencies:'),
          error.message
        );
        process.exit(1);
      }
    }
  );

// Remove dependency command
program
  .command('remove-dependency')
  .description('Remove a dependency relationship between work units')
  .argument('<workUnitId>', 'Work unit ID')
  .argument(
    '[dependsOnId]',
    'Work unit ID to remove from dependsOn (shorthand for --depends-on)'
  )
  .option('--blocks <targetId>', 'Remove blocks relationship')
  .option('--blocked-by <targetId>', 'Remove blockedBy relationship')
  .option('--depends-on <targetId>', 'Remove dependsOn relationship')
  .option('--relates-to <targetId>', 'Remove relatesTo relationship')
  .action(
    async (
      workUnitId: string,
      dependsOnId: string | undefined,
      options: {
        blocks?: string;
        blockedBy?: string;
        dependsOn?: string;
        relatesTo?: string;
      }
    ) => {
      try {
        // If second argument provided, use it as --depends-on (shorthand syntax)
        const finalDependsOn = dependsOnId || options.dependsOn;

        // Check if user provided both shorthand and option (conflict)
        if (
          dependsOnId &&
          options.dependsOn &&
          dependsOnId !== options.dependsOn
        ) {
          throw new Error(
            'Cannot specify dependency both as argument and --depends-on option'
          );
        }

        // Require at least one relationship type
        if (
          !finalDependsOn &&
          !options.blocks &&
          !options.blockedBy &&
          !options.relatesTo
        ) {
          throw new Error(
            'Must specify at least one relationship to remove: <depends-on-id> or --blocks/--blocked-by/--depends-on/--relates-to'
          );
        }

        await removeDependency({
          workUnitId,
          blocks: options.blocks,
          blockedBy: options.blockedBy,
          dependsOn: finalDependsOn,
          relatesTo: options.relatesTo,
        });
        console.log(chalk.green(`✓ Dependency removed successfully`));
      } catch (error: any) {
        console.error(
          chalk.red('✗ Failed to remove dependency:'),
          error.message
        );
        process.exit(1);
      }
    }
  );

// Clear dependencies command
program
  .command('clear-dependencies')
  .description('Remove all dependencies from a work unit')
  .argument('<workUnitId>', 'Work unit ID')
  .option('--confirm', 'Confirm clearing all dependencies')
  .action(async (workUnitId: string, options: { confirm?: boolean }) => {
    try {
      await clearDependencies({
        workUnitId,
        confirm: options.confirm,
      });
      console.log(chalk.green(`✓ All dependencies cleared from ${workUnitId}`));
    } catch (error: any) {
      console.error(
        chalk.red('✗ Failed to clear dependencies:'),
        error.message
      );
      process.exit(1);
    }
  });

// Export dependencies command
program
  .command('export-dependencies')
  .description('Export dependency graph visualization')
  .argument('<format>', 'Output format: mermaid or json')
  .argument('<output>', 'Output file path')
  .action(async (format: string, output: string) => {
    try {
      const result = await exportDependencies({
        format: format as 'mermaid' | 'json',
        output,
      });
      console.log(
        chalk.green(`✓ Dependencies exported to ${result.outputFile}`)
      );
    } catch (error: any) {
      console.error(
        chalk.red('✗ Failed to export dependencies:'),
        error.message
      );
      process.exit(1);
    }
  });

// ============================================================================
// PREFIX AND EPIC MANAGEMENT COMMANDS
// ============================================================================

// Create prefix command
program
  .command('create-prefix')
  .description('Register a new work unit prefix')
  .argument('<prefix>', 'Prefix code (2-6 uppercase letters, e.g., AUTH, DASH)')
  .argument('<description>', 'Prefix description')
  .action(async (prefix: string, description: string) => {
    try {
      await createPrefix({
        prefix,
        description,
      });
      console.log(chalk.green(`✓ Prefix ${prefix} created successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to create prefix:'), error.message);
      process.exit(1);
    }
  });

// Update prefix command
program
  .command('update-prefix')
  .description('Update an existing work unit prefix')
  .argument('<prefix>', 'Prefix code to update')
  .option('-d, --description <description>', 'New description')
  .action(async (prefix: string, options: { description?: string }) => {
    try {
      await updatePrefix({
        prefix,
        description: options.description,
      });
      console.log(chalk.green(`✓ Prefix ${prefix} updated successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to update prefix:'), error.message);
      process.exit(1);
    }
  });

// Delete epic command
program
  .command('delete-epic')
  .description('Delete an epic')
  .argument('<epicId>', 'Epic ID to delete')
  .option('--force', 'Force deletion even if work units are associated')
  .action(async (epicId: string, options: { force?: boolean }) => {
    try {
      await deleteEpic({
        epicId,
        force: options.force,
      });
      console.log(chalk.green(`✓ Epic ${epicId} deleted successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to delete epic:'), error.message);
      process.exit(1);
    }
  });

// ============================================================================
// EXAMPLE MAPPING COMMANDS
// ============================================================================

// Add example command
program
  .command('add-example')
  .description('Add an example to a work unit during specification phase')
  .argument('<workUnitId>', 'Work unit ID')
  .argument('<example>', 'Example description')
  .action(async (workUnitId: string, example: string) => {
    try {
      await addExample({ workUnitId, example });
      console.log(chalk.green(`✓ Example added successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to add example:'), error.message);
      process.exit(1);
    }
  });

// Add question command
program
  .command('add-question')
  .description('Add a question to a work unit during specification phase')
  .argument('<workUnitId>', 'Work unit ID')
  .argument('<question>', 'Question text')
  .action(async (workUnitId: string, question: string) => {
    try {
      await addQuestion({ workUnitId, question });
      console.log(chalk.green(`✓ Question added successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to add question:'), error.message);
      process.exit(1);
    }
  });

// Add rule command
program
  .command('add-rule')
  .description('Add a business rule to a work unit during specification phase')
  .argument('<workUnitId>', 'Work unit ID')
  .argument('<rule>', 'Business rule description')
  .action(async (workUnitId: string, rule: string) => {
    try {
      await addRule({ workUnitId, rule });
      console.log(chalk.green(`✓ Rule added successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to add rule:'), error.message);
      process.exit(1);
    }
  });

// Remove example command
program
  .command('remove-example')
  .description('Remove an example from a work unit by index')
  .argument('<workUnitId>', 'Work unit ID')
  .argument('<index>', 'Example index (0-based)')
  .action(async (workUnitId: string, index: string) => {
    try {
      const result = await removeExample({
        workUnitId,
        index: parseInt(index, 10),
      });
      console.log(chalk.green(`✓ Removed example: "${result.removedExample}"`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to remove example:'), error.message);
      process.exit(1);
    }
  });

// Remove question command
program
  .command('remove-question')
  .description('Remove a question from a work unit by index')
  .argument('<workUnitId>', 'Work unit ID')
  .argument('<index>', 'Question index (0-based)')
  .action(async (workUnitId: string, index: string) => {
    try {
      const result = await removeQuestion({
        workUnitId,
        index: parseInt(index, 10),
      });
      console.log(
        chalk.green(`✓ Removed question: "${result.removedQuestion}"`)
      );
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to remove question:'), error.message);
      process.exit(1);
    }
  });

// Remove rule command
program
  .command('remove-rule')
  .description('Remove a business rule from a work unit by index')
  .argument('<workUnitId>', 'Work unit ID')
  .argument('<index>', 'Rule index (0-based)')
  .action(async (workUnitId: string, index: string) => {
    try {
      const result = await removeRule({
        workUnitId,
        index: parseInt(index, 10),
      });
      console.log(chalk.green(`✓ Removed rule: "${result.removedRule}"`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to remove rule:'), error.message);
      process.exit(1);
    }
  });

// Answer question command
program
  .command('answer-question')
  .description('Answer a question and optionally add to rules or assumptions')
  .argument('<workUnitId>', 'Work unit ID')
  .argument('<index>', 'Question index (0-based)')
  .option('--answer <answer>', 'Answer text')
  .option('--add-to <type>', 'Add answer to: rule, assumption, or none', 'none')
  .action(
    async (
      workUnitId: string,
      index: string,
      options: { answer?: string; addTo?: string }
    ) => {
      try {
        const result = await answerQuestion({
          workUnitId,
          index: parseInt(index, 10),
          answer: options.answer,
          addTo: options.addTo as 'rule' | 'assumption' | 'none',
        });
        console.log(chalk.green(`✓ Answered question: "${result.question}"`));
        if (options.answer) {
          console.log(chalk.dim(`  Answer: "${options.answer}"`));
        }
        if (result.addedTo && result.addedContent) {
          console.log(
            chalk.cyan(`  Added to ${result.addedTo}: "${result.addedContent}"`)
          );
        }
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to answer question:'), error.message);
        process.exit(1);
      }
    }
  );

// Import example map command
program
  .command('import-example-map')
  .description('Import example mapping data from JSON file to work unit')
  .argument('<workUnitId>', 'Work unit ID')
  .argument('<file>', 'Input JSON file path')
  .action(async (workUnitId: string, file: string) => {
    try {
      const result = await importExampleMap({ workUnitId, file });
      const total =
        result.rulesCount +
        result.examplesCount +
        result.questionsCount +
        result.assumptionsCount;
      console.log(
        chalk.green(
          `✓ Imported ${total} items: ${result.rulesCount} rules, ${result.examplesCount} examples, ${result.questionsCount} questions, ${result.assumptionsCount} assumptions`
        )
      );
    } catch (error: any) {
      console.error(
        chalk.red('✗ Failed to import example map:'),
        error.message
      );
      process.exit(1);
    }
  });

// Export example map command
program
  .command('export-example-map')
  .description('Export example mapping data from work unit to JSON file')
  .argument('<workUnitId>', 'Work unit ID')
  .argument('<file>', 'Output JSON file path')
  .action(async (workUnitId: string, file: string) => {
    try {
      const result = await exportExampleMap({ workUnitId, file });
      console.log(chalk.green(`✓ Exported to ${result.outputFile}`));
    } catch (error: any) {
      console.error(
        chalk.red('✗ Failed to export example map:'),
        error.message
      );
      process.exit(1);
    }
  });

// Generate scenarios command
program
  .command('generate-scenarios')
  .description('Generate Gherkin scenarios from example mapping in work unit')
  .argument('<workUnitId>', 'Work unit ID')
  .option(
    '--feature <name>',
    'Feature file name (without .feature extension). Defaults to work unit title in kebab-case.'
  )
  .action(async (workUnitId: string, options: { feature?: string }) => {
    try {
      const result = await generateScenarios({
        workUnitId,
        feature: options.feature,
      });
      console.log(
        chalk.green(
          `✓ Generated ${result.scenariosCount} scenarios in ${result.featureFile}`
        )
      );

      // Display system reminders if any
      if (result.systemReminders && result.systemReminders.length > 0) {
        for (const reminder of result.systemReminders) {
          console.log('\n' + reminder);
        }
      }
    } catch (error: any) {
      console.error(
        chalk.red('✗ Failed to generate scenarios:'),
        error.message
      );
      process.exit(1);
    }
  });

// ============================================================================
// FEATURE DOCUMENTATION COMMANDS
// ============================================================================

// Add assumption command
program
  .command('add-assumption')
  .description('Add assumption to work unit during specification')
  .argument('<work-unit-id>', 'Work unit ID')
  .argument('<assumption>', 'Assumption text')
  .action(async (workUnitId: string, assumption: string) => {
    try {
      await addAssumption({ workUnitId, assumption });
      console.log(chalk.green(`✓ Assumption added successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to add assumption:'), error.message);
      process.exit(1);
    }
  });

// Set user story command
program
  .command('set-user-story')
  .description('Set user story fields for work unit')
  .argument('<work-unit-id>', 'Work unit ID')
  .requiredOption('--role <role>', 'User role (As a...)')
  .requiredOption('--action <action>', 'User action (I want to...)')
  .requiredOption('--benefit <benefit>', 'User benefit (So that...)')
  .action(
    async (
      workUnitId: string,
      options: { role: string; action: string; benefit: string }
    ) => {
      await setUserStoryCommand(workUnitId, options);
    }
  );

// ============================================================================
// WORKFLOW AND AUTOMATION COMMANDS
// ============================================================================

// Auto-advance command
program
  .command('auto-advance')
  .description('Automatically advance work units through workflow states')
  .option('--dry-run', 'Show what would be advanced without making changes')
  .action(async (options: { dryRun?: boolean }) => {
    try {
      const result = await autoAdvance({ dryRun: options.dryRun });
      console.log(chalk.green(`✓ Advanced ${result.advanced} work units`));
      if (result.details && result.details.length > 0) {
        result.details.forEach((detail: string) =>
          console.log(chalk.cyan(`  - ${detail}`))
        );
      }
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to auto-advance:'), error.message);
      process.exit(1);
    }
  });

// Display board command
program
  .command('board')
  .description('Display Kanban board of work units')
  .option('--format <format>', 'Output format: text or json', 'text')
  .option('--limit <limit>', 'Max items per column', '25')
  .action(async (options: { format?: string; limit?: string }) => {
    try {
      const result = await displayBoard({ cwd: process.cwd() });

      if (options.format === 'json') {
        console.log(JSON.stringify(result, null, 2));
      } else {
        // Display text format board using Ink
        const limit = parseInt(options.limit || '25', 10);
        const { render } = await import('ink');
        const React = await import('react');
        const { BoardDisplay } = await import('./components/BoardDisplay.js');

        render(
          React.createElement(BoardDisplay, {
            columns: result.columns,
            board: result.board,
            summary: result.summary,
            limit,
          })
        );
      }
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to display board:'), error.message);
      process.exit(1);
    }
  });

// Note: workflow-automation functionality is internal and used by other commands

// ============================================================================
// QUERY AND METRICS COMMANDS
// ============================================================================

// Note: General query command not implemented yet - use specific query commands below

// Query work units command
program
  .command('query-work-units')
  .description('Query work units with advanced filters')
  .option('--status <status>', 'Filter by status')
  .option('--prefix <prefix>', 'Filter by prefix')
  .option('--epic <epic>', 'Filter by epic')
  .option('--format <format>', 'Output format: text or json', 'text')
  .action(
    async (options: {
      status?: string;
      prefix?: string;
      epic?: string;
      format?: string;
    }) => {
      try {
        const result = await queryWorkUnits({
          status: options.status as any,
          prefix: options.prefix,
          epic: options.epic,
          format: options.format as 'text' | 'json',
        });
        if (options.format === 'json') {
          console.log(JSON.stringify(result, null, 2));
        }
      } catch (error: any) {
        console.error(chalk.red('✗ Query failed:'), error.message);
        process.exit(1);
      }
    }
  );

// Query dependency stats command
program
  .command('query-dependency-stats')
  .description('Show dependency statistics and potential blockers')
  .option('--format <format>', 'Output format: text or json', 'text')
  .action(async (options: { format?: string }) => {
    try {
      const result = await queryDependencyStats({
        format: options.format as 'text' | 'json',
      });
      if (options.format === 'json') {
        console.log(JSON.stringify(result, null, 2));
      }
    } catch (error: any) {
      console.error(chalk.red('✗ Query failed:'), error.message);
      process.exit(1);
    }
  });

// Query example mapping stats command
program
  .command('query-example-mapping-stats')
  .description('Show example mapping coverage statistics')
  .option('--format <format>', 'Output format: text or json', 'text')
  .action(async (options: { format?: string }) => {
    try {
      const result = await queryExampleMappingStats({
        format: options.format as 'text' | 'json',
      });
      if (options.format === 'json') {
        console.log(JSON.stringify(result, null, 2));
      }
    } catch (error: any) {
      console.error(chalk.red('✗ Query failed:'), error.message);
      process.exit(1);
    }
  });

// Query metrics command
program
  .command('query-metrics')
  .description('Query project metrics and statistics')
  .option('--metric <metric>', 'Specific metric to query')
  .option('--format <format>', 'Output format: text or json', 'text')
  .action(async (options: { metric?: string; format?: string }) => {
    try {
      const result = await queryMetrics({
        metric: options.metric,
        format: options.format as 'text' | 'json',
      });
      if (options.format === 'json') {
        console.log(JSON.stringify(result, null, 2));
      }
    } catch (error: any) {
      console.error(chalk.red('✗ Query failed:'), error.message);
      process.exit(1);
    }
  });

// Query estimate accuracy command
program
  .command('query-estimate-accuracy')
  .description('Show estimation accuracy metrics')
  .option('--format <format>', 'Output format: text or json', 'text')
  .action(async (options: { format?: string }) => {
    try {
      const result = await queryEstimateAccuracy({
        format: options.format as 'text' | 'json',
      });
      if (options.format === 'json') {
        console.log(JSON.stringify(result, null, 2));
      }
    } catch (error: any) {
      console.error(chalk.red('✗ Query failed:'), error.message);
      process.exit(1);
    }
  });

// Query estimation guide command
program
  .command('query-estimation-guide')
  .description('Get estimation guidance based on historical data')
  .argument('<workUnitId>', 'Work unit ID')
  .option('--format <format>', 'Output format: text or json', 'text')
  .action(async (workUnitId: string, options: { format?: string }) => {
    try {
      const result = await queryEstimationGuide({
        workUnitId,
        format: options.format as 'text' | 'json',
      });
      if (options.format === 'json') {
        console.log(JSON.stringify(result, null, 2));
      }
    } catch (error: any) {
      console.error(chalk.red('✗ Query failed:'), error.message);
      process.exit(1);
    }
  });

// Record metric command
program
  .command('record-metric')
  .description('Record a project metric')
  .argument('<metric>', 'Metric name')
  .argument('<value>', 'Metric value')
  .option('--unit <unit>', 'Unit of measurement')
  .action(async (metric: string, value: string, options: { unit?: string }) => {
    try {
      await recordMetric({
        metric,
        value: parseFloat(value),
        unit: options.unit,
      });
      console.log(chalk.green(`✓ Metric recorded successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to record metric:'), error.message);
      process.exit(1);
    }
  });

// Record tokens command
program
  .command('record-tokens')
  .description('Record token usage for AI operations')
  .argument('<workUnitId>', 'Work unit ID')
  .argument('<tokens>', 'Number of tokens used')
  .option(
    '--operation <operation>',
    'Operation type (e.g., specification, implementation)'
  )
  .action(
    async (
      workUnitId: string,
      tokens: string,
      options: { operation?: string }
    ) => {
      try {
        await recordTokens({
          workUnitId,
          tokens: parseInt(tokens, 10),
          operation: options.operation,
        });
        console.log(chalk.green(`✓ Token usage recorded successfully`));
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to record tokens:'), error.message);
        process.exit(1);
      }
    }
  );

// Record iteration command
program
  .command('record-iteration')
  .description('Record an iteration or sprint')
  .argument('<name>', 'Iteration name')
  .option('--start <date>', 'Start date')
  .option('--end <date>', 'End date')
  .action(async (name: string, options: { start?: string; end?: string }) => {
    try {
      await recordIteration({
        name,
        start: options.start,
        end: options.end,
      });
      console.log(chalk.green(`✓ Iteration recorded successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to record iteration:'), error.message);
      process.exit(1);
    }
  });

// Note: General estimation command not implemented yet - use update-work-unit-estimate instead

// Generate summary report command
program
  .command('generate-summary-report')
  .description('Generate a comprehensive project summary report')
  .option(
    '--format <format>',
    'Output format: markdown or json',
    'markdown'
  )
  .option('--output <file>', 'Output file path')
  .action(async (options: { format?: string; output?: string }) => {
    try {
      const result = await generateSummaryReport({
        format: options.format as 'markdown' | 'json',
        output: options.output,
      });
      console.log(chalk.green(`✓ Report generated: ${result.outputFile}`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to generate report:'), error.message);
      process.exit(1);
    }
  });

// ============================================================================
// VALIDATION COMMANDS
// ============================================================================

// Validate spec alignment command
program
  .command('validate-spec-alignment')
  .description('Validate alignment between specs, tests, and implementation')
  .option('--fix', 'Attempt to fix alignment issues')
  .action(async (options: { fix?: boolean }) => {
    try {
      const result = await validateSpecAlignment({ fix: options.fix });
      if (result.aligned) {
        console.log(
          chalk.green(`✓ All specs are aligned with tests and implementation`)
        );
      } else {
        console.error(
          chalk.red(`✗ Found ${result.issues.length} alignment issues`)
        );
        result.issues.forEach((issue: string) =>
          console.error(chalk.red(`  - ${issue}`))
        );
        process.exit(1);
      }
    } catch (error: any) {
      console.error(chalk.red('✗ Validation failed:'), error.message);
      process.exit(1);
    }
  });

// ============================================================================
// DEPENDENCY DISPLAY COMMAND
// ============================================================================

// Note: General dependencies display command not implemented yet
// Use query-dependency-stats or show-work-unit to view dependencies

// ============================================================================
// INIT COMMAND
// ============================================================================

program
  .command('init')
  .description('Initialize /fspec and /rspec slash commands for Claude Code')
  .option('--type <type>', 'Installation type: claude-code or custom')
  .option('--path <path>', 'Custom installation path (relative to current directory)')
  .option('--yes', 'Skip confirmation prompts (auto-confirm overwrite)')
  .action(async (options: { type?: string; path?: string; yes?: boolean }) => {
    try {
      let installType: 'claude-code' | 'custom';
      let customPath: string | undefined;

      // Determine install type
      if (options.type) {
        if (options.type !== 'claude-code' && options.type !== 'custom') {
          console.error(
            chalk.red(
              '✗ Invalid type. Must be "claude-code" or "custom"'
            )
          );
          process.exit(1);
        }
        installType = options.type as 'claude-code' | 'custom';

        if (installType === 'custom' && !options.path) {
          console.error(
            chalk.red('✗ --path is required when --type=custom')
          );
          process.exit(1);
        }
        customPath = options.path;
      } else {
        // Interactive mode (default to claude-code for now)
        installType = 'claude-code';
      }

      const result = await init({
        installType,
        customPath,
        confirmOverwrite: options.yes !== false,
      });

      console.log(chalk.green(result.message));
      process.exit(result.exitCode);
    } catch (error: any) {
      console.error(chalk.red('✗ Init failed:'), error.message);
      process.exit(1);
    }
  });

// Main execution with help interceptor
async function main(): Promise<void> {
  // Handle custom help before Commander.js processes arguments
  const { handleCustomHelp } = await import('./utils/help-interceptor');
  const customHelpShown = await handleCustomHelp();

  if (customHelpShown) {
    // Help was displayed and process.exit(0) was called
    return;
  }

  // Normal Commander.js execution
  program.parse();
}

// Run main function when executed directly (not when imported for testing)
// This works for both direct execution (./dist/index.js) and npm link (/usr/local/bin/fspec)
// by checking if the resolved script path matches this file
import { fileURLToPath } from 'url';
import { realpathSync } from 'fs';

const __filename = fileURLToPath(import.meta.url);
const isMainModule = (() => {
  try {
    // Resolve symlinks to get the actual file path
    const realArgv1 = realpathSync(process.argv[1]);
    return realArgv1 === __filename;
  } catch {
    // If we can't resolve, fall back to string comparison
    return process.argv[1]?.includes('index.js');
  }
})();

if (isMainModule) {
  main().catch((error) => {
    console.error(chalk.red('Fatal error:'), error.message);
    process.exit(1);
  });
}
