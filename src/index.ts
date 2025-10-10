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
  console.log('  - Work unit tags (@AUTH-001, @DASH-012) are validated against spec/work-units.json');
  console.log('  - Work unit tags link features to project management (see ' + chalk.cyan('fspec help project') + ')');
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
  console.log('  ' + chalk.cyan('fspec prioritize-work-unit <id>') + '   Change work unit priority in backlog');
  console.log('    Options:');
  console.log('      --position <position>            Position: top, bottom, or numeric index');
  console.log('      --before <workUnitId>            Place before this work unit');
  console.log('      --after <workUnitId>             Place after this work unit');
  console.log('    Examples:');
  console.log('      fspec prioritize-work-unit AUTH-001 --position=top');
  console.log('      fspec prioritize-work-unit AUTH-002 --before=AUTH-001');
  console.log('      fspec prioritize-work-unit AUTH-003 --position=5');
  console.log('');
  console.log('  ' + chalk.cyan('fspec update-work-unit <id>') + '        Update work unit properties');
  console.log('    Options:');
  console.log('      -t, --title <title>              New title');
  console.log('      -d, --description <desc>         New description');
  console.log('      -e, --epic <epic>                Epic ID');
  console.log('      -p, --parent <parent>            Parent work unit ID');
  console.log('    Examples:');
  console.log('      fspec update-work-unit AUTH-001 -t "New title"');
  console.log('      fspec update-work-unit AUTH-001 -e user-management');
  console.log('');
  console.log('  ' + chalk.cyan('fspec update-work-unit-status <id> <status>') + ' Update status (ACDD workflow)');
  console.log('    Status values: backlog, specifying, testing, implementing, validating, done, blocked');
  console.log('    Options:');
  console.log('      --blocked-reason <reason>        Required when status is blocked');
  console.log('      --reason <reason>                Optional reason for change');
  console.log('    Examples:');
  console.log('      fspec update-work-unit-status AUTH-001 implementing');
  console.log('      fspec update-work-unit-status AUTH-001 blocked --blocked-reason "API not ready"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec update-work-unit-estimate <id> <points>') + ' Set Fibonacci estimate');
  console.log('    Valid estimates: 1, 2, 3, 5, 8, 13, 21 (Fibonacci numbers)');
  console.log('    Examples:');
  console.log('      fspec update-work-unit-estimate AUTH-001 5');
  console.log('      fspec update-work-unit-estimate AUTH-002 13');
  console.log('');
  console.log('  ' + chalk.cyan('fspec delete-work-unit <id>') + '        Delete a work unit');
  console.log('    Options:');
  console.log('      --force                          Force deletion without checks');
  console.log('      --skip-confirmation              Skip confirmation prompt');
  console.log('      --cascade-dependencies           Remove all dependencies first');
  console.log('    Examples:');
  console.log('      fspec delete-work-unit AUTH-001');
  console.log('      fspec delete-work-unit AUTH-001 --force --cascade-dependencies');
  console.log('');
  console.log('  ' + chalk.cyan('fspec validate-work-units') + '          Validate work units data integrity');
  console.log('    Examples:');
  console.log('      fspec validate-work-units');
  console.log('');
  console.log('  ' + chalk.cyan('fspec repair-work-units') + '            Repair data integrity issues');
  console.log('    Options:');
  console.log('      --dry-run                        Show what would be repaired');
  console.log('    Examples:');
  console.log('      fspec repair-work-units --dry-run');
  console.log('      fspec repair-work-units');
  console.log('');
  console.log('  ' + chalk.cyan('fspec export-work-units <format> <output>') + ' Export to JSON or CSV');
  console.log('    Options:');
  console.log('      --status <status>                Filter by status');
  console.log('    Examples:');
  console.log('      fspec export-work-units json work-units.json');
  console.log('      fspec export-work-units csv backlog.csv --status=backlog');
  console.log('');

  console.log(chalk.bold('DEPENDENCY MANAGEMENT'));
  console.log('  ' + chalk.cyan('fspec add-dependency <id>') + '         Add dependency relationship between work units');
  console.log('    Relationship types:');
  console.log('      --blocks <id>                    This work blocks another work unit');
  console.log('      --blocked-by <id>                This work is blocked by another work unit');
  console.log('      --depends-on <id>                Soft dependency (can start but should wait)');
  console.log('      --relates-to <id>                Related work unit (informational)');
  console.log('    Examples:');
  console.log('      fspec add-dependency AUTH-001 --blocks API-001');
  console.log('      fspec add-dependency UI-001 --blocked-by API-001');
  console.log('      fspec add-dependency DASH-001 --depends-on AUTH-001');
  console.log('      fspec add-dependency AUTH-001 --relates-to SEC-001');
  console.log('');
  console.log('  ' + chalk.cyan('fspec add-dependencies <id>') + '        Add multiple dependencies at once');
  console.log('    Options: accepts comma-separated lists for each relationship type');
  console.log('    Examples:');
  console.log('      fspec add-dependencies AUTH-001 --blocks API-001,UI-001 --depends-on DB-001');
  console.log('');
  console.log('  ' + chalk.cyan('fspec remove-dependency <id>') + '      Remove a dependency relationship');
  console.log('    Options: same as add-dependency (--blocks, --blocked-by, --depends-on, --relates-to)');
  console.log('    Examples:');
  console.log('      fspec remove-dependency AUTH-001 --blocks API-001');
  console.log('');
  console.log('  ' + chalk.cyan('fspec clear-dependencies <id>') + '     Remove all dependencies from work unit');
  console.log('    Options:');
  console.log('      --confirm                        Confirm clearing all dependencies');
  console.log('    Examples:');
  console.log('      fspec clear-dependencies AUTH-001 --confirm');
  console.log('');
  console.log('  ' + chalk.cyan('fspec export-dependencies <format> <output>') + ' Export dependency graph');
  console.log('    Formats: mermaid (visualization), json (data)');
  console.log('    Examples:');
  console.log('      fspec export-dependencies mermaid deps.mmd');
  console.log('      fspec export-dependencies json deps.json');
  console.log('');

  console.log(chalk.bold('PREFIX AND EPIC MANAGEMENT'));
  console.log('  ' + chalk.cyan('fspec create-prefix <code> <description>') + ' Register new work unit prefix');
  console.log('    Prefix codes: 2-6 uppercase letters (e.g., AUTH, DASH, API)');
  console.log('    Examples:');
  console.log('      fspec create-prefix AUTH "Authentication features"');
  console.log('      fspec create-prefix PERF "Performance optimization"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec update-prefix <code>') + '        Update prefix description');
  console.log('    Options:');
  console.log('      -d, --description <desc>         New description');
  console.log('    Examples:');
  console.log('      fspec update-prefix AUTH -d "Updated description"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec create-epic <id> <title>') + '      Create new epic');
  console.log('    Epic IDs: lowercase-with-hyphens (e.g., user-management)');
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
  console.log('  ' + chalk.cyan('fspec delete-epic <id>') + '              Delete an epic');
  console.log('    Options:');
  console.log('      --force                          Force deletion even if work units associated');
  console.log('    Examples:');
  console.log('      fspec delete-epic old-epic --force');
  console.log('');

  console.log(chalk.bold('EXAMPLE MAPPING'));
  console.log('  Example Mapping is a collaborative technique for exploring and understanding requirements.');
  console.log('  Add examples, questions, and business rules to scenarios before implementation.');
  console.log('');
  console.log('  ' + chalk.cyan('fspec add-example <feature> <scenario> <example>') + ' Add concrete example');
  console.log('    Examples:');
  console.log('      fspec add-example login "Successful login" "user: alice@example.com, pwd: secret123"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec add-question <feature> <scenario> <question>') + ' Add unanswered question');
  console.log('    Examples:');
  console.log('      fspec add-question login "Login" "What happens if password is expired?"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec add-rule <feature> <scenario> <rule>') + ' Add business rule');
  console.log('    Examples:');
  console.log('      fspec add-rule login "Login" "Password must be at least 8 characters"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec answer-question <feature> <scenario> <question> <answer>') + ' Answer question');
  console.log('    Examples:');
  console.log('      fspec answer-question login "Login" "What about 2FA?" "2FA is required for admins"');
  console.log('');
  console.log('  ' + chalk.cyan('fspec remove-example <feature> <scenario> <example>') + ' Remove example');
  console.log('  ' + chalk.cyan('fspec remove-question <feature> <scenario> <question>') + ' Remove question');
  console.log('  ' + chalk.cyan('fspec remove-rule <feature> <scenario> <rule>') + ' Remove rule');
  console.log('');
  console.log('  ' + chalk.cyan('fspec export-example-map <feature> <output>') + ' Export to JSON');
  console.log('    Examples:');
  console.log('      fspec export-example-map login example-map.json');
  console.log('');
  console.log('  ' + chalk.cyan('fspec import-example-map <feature> <input>') + ' Import from JSON');
  console.log('    Examples:');
  console.log('      fspec import-example-map login example-map.json');
  console.log('');
  console.log('  ' + chalk.cyan('fspec generate-scenarios <feature>') + ' Generate scenarios from examples');
  console.log('    Examples:');
  console.log('      fspec generate-scenarios login');
  console.log('');

  console.log(chalk.bold('WORKFLOW AUTOMATION'));
  console.log('  ' + chalk.cyan('fspec auto-advance') + '                  Automatically advance ready work units');
  console.log('    Options:');
  console.log('      --dry-run                        Show what would be advanced');
  console.log('    Examples:');
  console.log('      fspec auto-advance --dry-run');
  console.log('      fspec auto-advance');
  console.log('');
  console.log('  ' + chalk.cyan('fspec display-board') + '                 Display Kanban board of all work');
  console.log('    Options:');
  console.log('      --format <format>                Output: text or json (default: text)');
  console.log('    Examples:');
  console.log('      fspec display-board');
  console.log('      fspec display-board --format=json');
  console.log('');

  console.log(chalk.bold('QUERY AND REPORTING'));
  console.log('  ' + chalk.cyan('fspec query-work-units') + '             Query work units with filters');
  console.log('    Options:');
  console.log('      --status <status>                Filter by status');
  console.log('      --prefix <prefix>                Filter by prefix');
  console.log('      --epic <epic>                    Filter by epic');
  console.log('      --format <format>                Output: text or json (default: text)');
  console.log('    Examples:');
  console.log('      fspec query-work-units --status implementing --format json');
  console.log('      fspec query-work-units --prefix AUTH --epic user-management');
  console.log('');
  console.log('  ' + chalk.cyan('fspec query-dependency-stats') + '       Show dependency statistics');
  console.log('    Options:');
  console.log('      --format <format>                Output: text or json (default: text)');
  console.log('    Examples:');
  console.log('      fspec query-dependency-stats');
  console.log('');
  console.log('  ' + chalk.cyan('fspec query-example-mapping-stats') + '  Show example mapping coverage');
  console.log('    Examples:');
  console.log('      fspec query-example-mapping-stats');
  console.log('');
  console.log('  ' + chalk.cyan('fspec query-metrics') + '                Query project metrics');
  console.log('    Options:');
  console.log('      --metric <metric>                Specific metric to query');
  console.log('      --format <format>                Output: text or json (default: text)');
  console.log('    Examples:');
  console.log('      fspec query-metrics --format json');
  console.log('');
  console.log('  ' + chalk.cyan('fspec query-estimate-accuracy') + '      Show estimation accuracy');
  console.log('    Examples:');
  console.log('      fspec query-estimate-accuracy');
  console.log('');
  console.log('  ' + chalk.cyan('fspec query-estimation-guide <id>') + '  Get estimation guidance');
  console.log('    Examples:');
  console.log('      fspec query-estimation-guide AUTH-001');
  console.log('');
  console.log('  ' + chalk.cyan('fspec generate-summary-report') + '      Generate comprehensive project report');
  console.log('    Options:');
  console.log('      --format <format>                Output: markdown, json, or html (default: markdown)');
  console.log('      --output <file>                  Output file path');
  console.log('    Examples:');
  console.log('      fspec generate-summary-report --format markdown --output report.md');
  console.log('');

  console.log(chalk.bold('METRICS TRACKING'));
  console.log('  ' + chalk.cyan('fspec record-metric <metric> <value>') + ' Record a project metric');
  console.log('    Options:');
  console.log('      --unit <unit>                    Unit of measurement');
  console.log('    Examples:');
  console.log('      fspec record-metric build-time 45.2 --unit seconds');
  console.log('');
  console.log('  ' + chalk.cyan('fspec record-tokens <id> <tokens>') + '  Record AI token usage');
  console.log('    Options:');
  console.log('      --operation <operation>          Operation type (e.g., specification, implementation)');
  console.log('    Examples:');
  console.log('      fspec record-tokens AUTH-001 45000 --operation implementation');
  console.log('');
  console.log('  ' + chalk.cyan('fspec record-iteration <name>') + '     Record iteration or sprint');
  console.log('    Options:');
  console.log('      --start <date>                   Start date');
  console.log('      --end <date>                     End date');
  console.log('    Examples:');
  console.log('      fspec record-iteration "Sprint 1" --start 2025-01-01 --end 2025-01-14');
  console.log('');

  console.log(chalk.bold('VALIDATION'));
  console.log('  ' + chalk.cyan('fspec validate-spec-alignment') + '     Validate specs align with tests and code');
  console.log('    Options:');
  console.log('      --fix                            Attempt to fix alignment issues');
  console.log('    Examples:');
  console.log('      fspec validate-spec-alignment');
  console.log('      fspec validate-spec-alignment --fix');
  console.log('');

  console.log(chalk.bold('FEATURE DOCUMENTATION'));
  console.log('  ' + chalk.cyan('fspec add-assumption <feature> <assumption>') + ' Add assumption to feature');
  console.log('    Examples:');
  console.log('      fspec add-assumption login "Users have verified email addresses"');
  console.log('');

  console.log(chalk.bold('WORK UNIT LINKING WITH FEATURES'));
  console.log('  Link work units to feature files using tags with pattern: ' + chalk.cyan('@[A-Z]{2,6}-\\d+'));
  console.log('  Examples: @AUTH-001, @DASH-012, @API-123');
  console.log('');
  console.log('  ' + chalk.dim('Add work unit tags to feature files:'));
  console.log('    ' + chalk.cyan('@AUTH-001') + '           # Feature-level tag (applies to all scenarios)');
  console.log('    ' + chalk.cyan('@DASH-012') + '           # Scenario-level tag (overrides feature-level)');
  console.log('');
  console.log('  ' + chalk.dim('Query linked features and scenarios:'));
  console.log('    ' + chalk.cyan('fspec show-feature <file>') + '            # Shows work units linked to feature');
  console.log('    ' + chalk.cyan('fspec show-work-unit AUTH-001') + '        # Shows features/scenarios linked to work unit');
  console.log('    ' + chalk.cyan('fspec get-scenarios --tag=@AUTH-001') + '  # Filter scenarios by work unit');
  console.log('');
  console.log('  ' + chalk.dim('Validation:'));
  console.log('    ' + chalk.cyan('fspec validate-tags') + '                  # Validates work unit tags exist in work-units.json');
  console.log('    All work unit tags are validated against spec/work-units.json');
  console.log('    Invalid work unit IDs or formats will be reported as errors');
  console.log('');

  console.log(chalk.bold('WORKFLOW STATES'));
  console.log('  Work units progress through Kanban states:');
  console.log('  backlog → specifying → testing → implementing → validating → done');
  console.log('  (blocked state can occur at any point)');
  console.log('');

  console.log(chalk.bold('NOTES'));
  console.log('  - Work units are stored in spec/work-units.json');
  console.log('  - Epics are stored in spec/epics.json');
  console.log('  - Work unit tags (@WORK-001) are distinct from regular tags (@phase1)');
  console.log('  - Feature-level work unit tags apply to all scenarios by default');
  console.log('  - Scenario-level work unit tags override feature-level tags');
  console.log('  - All commands follow ACDD (Acceptance Criteria Driven Development)');
  console.log('  - Dependency types: blocks (hard blocker), blocked-by (inverse), depends-on (soft), relates-to (info)');
  console.log('  - Story point estimates use Fibonacci sequence: 1, 2, 3, 5, 8, 13, 21');
  console.log('  - Example Mapping helps explore requirements before writing scenarios');
  console.log('  - AI agents should use query commands to understand project state');
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
  .action(async (workUnitId: string, options: { position?: string; before?: string; after?: string }) => {
    try {
      const parsedPosition = options.position === 'top' ? 'top'
        : options.position === 'bottom' ? 'bottom'
        : options.position ? parseInt(options.position, 10)
        : undefined;

      await prioritizeWorkUnit({
        workUnitId,
        position: parsedPosition as 'top' | 'bottom' | number | undefined,
        before: options.before,
        after: options.after,
      });
      console.log(chalk.green(`✓ Work unit ${workUnitId} prioritized successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to prioritize work unit:'), error.message);
      process.exit(1);
    }
  });

// Update work unit command
program
  .command('update-work-unit')
  .description('Update work unit properties')
  .argument('<workUnitId>', 'Work unit ID to update')
  .option('-t, --title <title>', 'New title')
  .option('-d, --description <description>', 'New description')
  .option('-e, --epic <epic>', 'Epic ID')
  .option('-p, --parent <parent>', 'Parent work unit ID')
  .action(async (workUnitId: string, options: { title?: string; description?: string; epic?: string; parent?: string }) => {
    try {
      await updateWorkUnit({
        workUnitId,
        ...options,
      });
      console.log(chalk.green(`✓ Work unit ${workUnitId} updated successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to update work unit:'), error.message);
      process.exit(1);
    }
  });

// Delete work unit command
program
  .command('delete-work-unit')
  .description('Delete a work unit')
  .argument('<workUnitId>', 'Work unit ID to delete')
  .option('--force', 'Force deletion without checks')
  .option('--skip-confirmation', 'Skip confirmation prompt')
  .option('--cascade-dependencies', 'Remove all dependencies before deleting')
  .action(async (workUnitId: string, options: { force?: boolean; skipConfirmation?: boolean; cascadeDependencies?: boolean }) => {
    try {
      const result = await deleteWorkUnit({
        workUnitId,
        ...options,
      });
      console.log(chalk.green(`✓ Work unit ${workUnitId} deleted successfully`));
      if (result.warnings && result.warnings.length > 0) {
        result.warnings.forEach((warning: string) => console.log(chalk.yellow(`⚠ ${warning}`)));
      }
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to delete work unit:'), error.message);
      process.exit(1);
    }
  });

// Update work unit status command
program
  .command('update-work-unit-status')
  .description('Update work unit status (follows ACDD workflow)')
  .argument('<workUnitId>', 'Work unit ID')
  .argument('<status>', 'New status: backlog, specifying, testing, implementing, validating, done, blocked')
  .option('--blocked-reason <reason>', 'Reason for blocked status (required if status is blocked)')
  .option('--reason <reason>', 'Reason for status change')
  .action(async (workUnitId: string, status: string, options: { blockedReason?: string; reason?: string }) => {
    try {
      const result = await updateWorkUnitStatus({
        workUnitId,
        status: status as 'backlog' | 'specifying' | 'testing' | 'implementing' | 'validating' | 'done' | 'blocked',
        blockedReason: options.blockedReason,
        reason: options.reason,
      });
      console.log(chalk.green(`✓ Work unit ${workUnitId} status updated to ${status}`));
      if (result.warnings && result.warnings.length > 0) {
        result.warnings.forEach((warning: string) => console.log(chalk.yellow(`⚠ ${warning}`)));
      }
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to update work unit status:'), error.message);
      process.exit(1);
    }
  });

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
      console.log(chalk.green(`✓ Work unit ${workUnitId} estimate set to ${estimate}`));
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
        console.error(chalk.red(`✗ Found ${result.errors.length} validation errors`));
        result.errors.forEach((error: string) => console.error(chalk.red(`  - ${error}`)));
        process.exit(1);
      }
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to validate work units:'), error.message);
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
        result.details.forEach((detail: string) => console.log(chalk.cyan(`  - ${detail}`)));
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
  .action(async (format: string, output: string, options: { status?: string }) => {
    try {
      const result = await exportWorkUnits({
        format: format as 'json' | 'csv',
        output,
        status: options.status as any,
      });
      console.log(chalk.green(`✓ Exported ${result.count} work units to ${result.outputFile}`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to export work units:'), error.message);
      process.exit(1);
    }
  });

// ============================================================================
// DEPENDENCY MANAGEMENT COMMANDS
// ============================================================================

// Add dependency command
program
  .command('add-dependency')
  .description('Add a dependency relationship between work units')
  .argument('<workUnitId>', 'Work unit ID')
  .option('--blocks <targetId>', 'Work unit that this blocks')
  .option('--blocked-by <targetId>', 'Work unit that blocks this')
  .option('--depends-on <targetId>', 'Work unit this depends on (soft dependency)')
  .option('--relates-to <targetId>', 'Related work unit')
  .action(async (workUnitId: string, options: { blocks?: string; blockedBy?: string; dependsOn?: string; relatesTo?: string }) => {
    try {
      await addDependency({
        workUnitId,
        blocks: options.blocks,
        blockedBy: options.blockedBy,
        dependsOn: options.dependsOn,
        relatesTo: options.relatesTo,
      });
      console.log(chalk.green(`✓ Dependency added successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to add dependency:'), error.message);
      process.exit(1);
    }
  });

// Add dependencies (plural) command
program
  .command('add-dependencies')
  .description('Add multiple dependency relationships at once')
  .argument('<workUnitId>', 'Work unit ID')
  .option('--blocks <ids...>', 'Work unit IDs that this blocks')
  .option('--blocked-by <ids...>', 'Work unit IDs that block this')
  .option('--depends-on <ids...>', 'Work unit IDs this depends on')
  .option('--relates-to <ids...>', 'Related work unit IDs')
  .action(async (workUnitId: string, options: { blocks?: string[]; blockedBy?: string[]; dependsOn?: string[]; relatesTo?: string[] }) => {
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
      console.log(chalk.green(`✓ Added ${result.added} dependencies successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to add dependencies:'), error.message);
      process.exit(1);
    }
  });

// Remove dependency command
program
  .command('remove-dependency')
  .description('Remove a dependency relationship between work units')
  .argument('<workUnitId>', 'Work unit ID')
  .option('--blocks <targetId>', 'Remove blocks relationship')
  .option('--blocked-by <targetId>', 'Remove blockedBy relationship')
  .option('--depends-on <targetId>', 'Remove dependsOn relationship')
  .option('--relates-to <targetId>', 'Remove relatesTo relationship')
  .action(async (workUnitId: string, options: { blocks?: string; blockedBy?: string; dependsOn?: string; relatesTo?: string }) => {
    try {
      await removeDependency({
        workUnitId,
        blocks: options.blocks,
        blockedBy: options.blockedBy,
        dependsOn: options.dependsOn,
        relatesTo: options.relatesTo,
      });
      console.log(chalk.green(`✓ Dependency removed successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to remove dependency:'), error.message);
      process.exit(1);
    }
  });

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
      console.error(chalk.red('✗ Failed to clear dependencies:'), error.message);
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
      console.log(chalk.green(`✓ Dependencies exported to ${result.outputFile}`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to export dependencies:'), error.message);
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
  .description('Add an example to a scenario for example mapping')
  .argument('<feature>', 'Feature file name or path')
  .argument('<scenario>', 'Scenario name')
  .argument('<example>', 'Example description')
  .action(async (feature: string, scenario: string, example: string) => {
    try {
      await addExample({ feature, scenario, example });
      console.log(chalk.green(`✓ Example added successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to add example:'), error.message);
      process.exit(1);
    }
  });

// Add question command
program
  .command('add-question')
  .description('Add a question to a scenario for example mapping')
  .argument('<feature>', 'Feature file name or path')
  .argument('<scenario>', 'Scenario name')
  .argument('<question>', 'Question text')
  .action(async (feature: string, scenario: string, question: string) => {
    try {
      await addQuestion({ feature, scenario, question });
      console.log(chalk.green(`✓ Question added successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to add question:'), error.message);
      process.exit(1);
    }
  });

// Add rule command
program
  .command('add-rule')
  .description('Add a business rule to a scenario for example mapping')
  .argument('<feature>', 'Feature file name or path')
  .argument('<scenario>', 'Scenario name')
  .argument('<rule>', 'Business rule description')
  .action(async (feature: string, scenario: string, rule: string) => {
    try {
      await addRule({ feature, scenario, rule });
      console.log(chalk.green(`✓ Rule added successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to add rule:'), error.message);
      process.exit(1);
    }
  });

// Remove example command
program
  .command('remove-example')
  .description('Remove an example from a scenario')
  .argument('<feature>', 'Feature file name or path')
  .argument('<scenario>', 'Scenario name')
  .argument('<example>', 'Example description to remove')
  .action(async (feature: string, scenario: string, example: string) => {
    try {
      await removeExample({ feature, scenario, example });
      console.log(chalk.green(`✓ Example removed successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to remove example:'), error.message);
      process.exit(1);
    }
  });

// Remove question command
program
  .command('remove-question')
  .description('Remove a question from a scenario')
  .argument('<feature>', 'Feature file name or path')
  .argument('<scenario>', 'Scenario name')
  .argument('<question>', 'Question text to remove')
  .action(async (feature: string, scenario: string, question: string) => {
    try {
      await removeQuestion({ feature, scenario, question });
      console.log(chalk.green(`✓ Question removed successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to remove question:'), error.message);
      process.exit(1);
    }
  });

// Remove rule command
program
  .command('remove-rule')
  .description('Remove a business rule from a scenario')
  .argument('<feature>', 'Feature file name or path')
  .argument('<scenario>', 'Scenario name')
  .argument('<rule>', 'Rule description to remove')
  .action(async (feature: string, scenario: string, rule: string) => {
    try {
      await removeRule({ feature, scenario, rule });
      console.log(chalk.green(`✓ Rule removed successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to remove rule:'), error.message);
      process.exit(1);
    }
  });

// Answer question command
program
  .command('answer-question')
  .description('Answer a question from example mapping session')
  .argument('<feature>', 'Feature file name or path')
  .argument('<scenario>', 'Scenario name')
  .argument('<question>', 'Question text')
  .argument('<answer>', 'Answer text')
  .action(async (feature: string, scenario: string, question: string, answer: string) => {
    try {
      await answerQuestion({ feature, scenario, question, answer });
      console.log(chalk.green(`✓ Question answered successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to answer question:'), error.message);
      process.exit(1);
    }
  });

// Import example map command
program
  .command('import-example-map')
  .description('Import example mapping data from JSON file')
  .argument('<feature>', 'Feature file name or path')
  .argument('<input>', 'Input JSON file path')
  .action(async (feature: string, input: string) => {
    try {
      const result = await importExampleMap({ feature, input });
      console.log(chalk.green(`✓ Imported ${result.imported} items successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to import example map:'), error.message);
      process.exit(1);
    }
  });

// Export example map command
program
  .command('export-example-map')
  .description('Export example mapping data to JSON file')
  .argument('<feature>', 'Feature file name or path')
  .argument('<output>', 'Output JSON file path')
  .action(async (feature: string, output: string) => {
    try {
      const result = await exportExampleMap({ feature, output });
      console.log(chalk.green(`✓ Exported to ${result.outputFile}`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to export example map:'), error.message);
      process.exit(1);
    }
  });

// Generate scenarios command
program
  .command('generate-scenarios')
  .description('Generate Gherkin scenarios from example mapping')
  .argument('<feature>', 'Feature file name or path')
  .action(async (feature: string) => {
    try {
      const result = await generateScenarios({ feature });
      console.log(chalk.green(`✓ Generated ${result.generated} scenarios`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to generate scenarios:'), error.message);
      process.exit(1);
    }
  });

// ============================================================================
// FEATURE DOCUMENTATION COMMANDS
// ============================================================================

// Add assumption command
program
  .command('add-assumption')
  .description('Add an assumption to a feature file')
  .argument('<feature>', 'Feature file name or path')
  .argument('<assumption>', 'Assumption text')
  .action(async (feature: string, assumption: string) => {
    try {
      await addAssumption({ feature, assumption });
      console.log(chalk.green(`✓ Assumption added successfully`));
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to add assumption:'), error.message);
      process.exit(1);
    }
  });

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
        result.details.forEach((detail: string) => console.log(chalk.cyan(`  - ${detail}`)));
      }
    } catch (error: any) {
      console.error(chalk.red('✗ Failed to auto-advance:'), error.message);
      process.exit(1);
    }
  });

// Display board command
program
  .command('display-board')
  .description('Display Kanban board of work units')
  .option('--format <format>', 'Output format: text or json', 'text')
  .action(async (options: { format?: string }) => {
    try {
      const result = await displayBoard({ format: options.format as 'text' | 'json' });
      if (options.format === 'json') {
        console.log(JSON.stringify(result, null, 2));
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
  .action(async (options: { status?: string; prefix?: string; epic?: string; format?: string }) => {
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
  });

// Query dependency stats command
program
  .command('query-dependency-stats')
  .description('Show dependency statistics and potential blockers')
  .option('--format <format>', 'Output format: text or json', 'text')
  .action(async (options: { format?: string }) => {
    try {
      const result = await queryDependencyStats({ format: options.format as 'text' | 'json' });
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
      const result = await queryExampleMappingStats({ format: options.format as 'text' | 'json' });
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
      const result = await queryEstimateAccuracy({ format: options.format as 'text' | 'json' });
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
  .option('--operation <operation>', 'Operation type (e.g., specification, implementation)')
  .action(async (workUnitId: string, tokens: string, options: { operation?: string }) => {
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
  });

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
  .option('--format <format>', 'Output format: markdown, json, or html', 'markdown')
  .option('--output <file>', 'Output file path')
  .action(async (options: { format?: string; output?: string }) => {
    try {
      const result = await generateSummaryReport({
        format: options.format as 'markdown' | 'json' | 'html',
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
        console.log(chalk.green(`✓ All specs are aligned with tests and implementation`));
      } else {
        console.error(chalk.red(`✗ Found ${result.issues.length} alignment issues`));
        result.issues.forEach((issue: string) => console.error(chalk.red(`  - ${issue}`)));
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

program.parse();
