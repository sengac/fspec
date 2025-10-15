import chalk from 'chalk';

// Help functions (preserved from original)
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
  console.log(
    '  ' +
      chalk.cyan('hooks') +
      '     - Lifecycle hooks for workflow automation (validate, add, remove, list)'
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
      '        - Configuration and setup commands'
  );
  console.log(
    '  ' +
      chalk.cyan('fspec help hooks') +
      '        - Lifecycle hook commands\n'
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

  console.log(chalk.bold('COVERAGE TRACKING (Scenario-to-Test-to-Implementation)'));
  console.log(
    '  ' +
      chalk.cyan('fspec link-coverage <feature> --scenario <name>') +
      ' Link tests/implementation'
  );
  console.log('    Options:');
  console.log(
    '      --test-file <path>               Test file path (e.g., src/__tests__/auth.test.ts)'
  );
  console.log(
    '      --test-lines <range>             Test line range (e.g., "45-62")'
  );
  console.log(
    '      --impl-file <path>               Implementation file path'
  );
  console.log(
    '      --impl-lines <lines>             Implementation lines (e.g., "10,11,12,23")'
  );
  console.log(
    '      --skip-validation                Skip file validation (reverse ACDD)'
  );
  console.log('    Examples:');
  console.log(
    '      fspec link-coverage user-auth --scenario "Login" --test-file src/__tests__/auth.test.ts --test-lines 45-62'
  );
  console.log('');
  console.log(
    '  ' +
      chalk.cyan('fspec show-coverage [feature]') +
      '    Show coverage report (gaps)'
  );
  console.log('    Options:');
  console.log('      --format <format>                Output: text or json');
  console.log('      --output <file>                  Write to file');
  console.log('    Examples:');
  console.log('      fspec show-coverage user-authentication');
  console.log('      fspec show-coverage              # Project-wide');
  console.log('');
  console.log(
    '  ' +
      chalk.cyan('fspec audit-coverage <feature>') +
      '     Verify file paths exist'
  );
  console.log('    Options:');
  console.log(
    '      --fix                            Remove broken mappings'
  );
  console.log('    Examples:');
  console.log('      fspec audit-coverage user-authentication --fix');
  console.log('');
  console.log(
    '  ' +
      chalk.cyan('fspec unlink-coverage <feature>') +
      '    Remove coverage mappings'
  );
  console.log('    Options:');
  console.log('      --scenario <name>                Scenario to unlink');
  console.log('      --test-file <path>               Test file to remove');
  console.log('      --impl-file <path>               Implementation to remove');
  console.log('      --all                            Remove all mappings');
  console.log('    Examples:');
  console.log(
    '      fspec unlink-coverage user-auth --scenario "Login" --all'
  );
  console.log('');
  console.log(
    '  ' +
      chalk.cyan('fspec generate-coverage') +
      '           Generate .coverage files for existing features'
  );
  console.log('    Options:');
  console.log('      --dry-run                        Preview without creating files');
  console.log('    Examples:');
  console.log('      fspec generate-coverage');
  console.log('      fspec generate-coverage --dry-run');
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
  console.log('  • Capture architecture notes and non-functional requirements');
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

  console.log(chalk.bold('ARCHITECTURE NOTES'));
  console.log(
    '  ' + chalk.cyan('fspec add-architecture-note <work-unit-id> <note>')
  );
  console.log(
    '    Description: Capture architecture decisions, NFRs, dependencies'
  );
  console.log('    Examples:');
  console.log(
    '      fspec add-architecture-note AUTH-001 "Uses @cucumber/gherkin parser"'
  );
  console.log(
    '      fspec add-architecture-note AUTH-001 "Performance: Must complete within 2 seconds"'
  );
  console.log('');
  console.log(
    '  ' + chalk.cyan('fspec remove-architecture-note <work-unit-id> <index>')
  );
  console.log('    Description: Remove architecture note by index (0-based)');
  console.log('    Examples:');
  console.log('      fspec remove-architecture-note AUTH-001 0');
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

// ===== HOOKS HELP =====
function displayHooksHelp(): void {
  console.log(chalk.bold('\nLIFECYCLE HOOKS'));
  console.log(chalk.dim('Workflow automation with lifecycle hooks\n'));

  console.log('Use this when you need to:');
  console.log('  • Execute custom scripts at command lifecycle events');
  console.log('  • Add quality gates with blocking pre-hooks');
  console.log('  • Automate testing with post-hooks');
  console.log('  • Send notifications on workflow events');
  console.log('  • Validate hook configurations');
  console.log('  • List and manage configured hooks\n');

  console.log(chalk.bold('HOOK MANAGEMENT'));
  console.log('  ' + chalk.cyan('fspec list-hooks'));
  console.log('    Description: List all configured hooks');
  console.log('    Examples:');
  console.log('      fspec list-hooks');
  console.log('');
  console.log('  ' + chalk.cyan('fspec validate-hooks'));
  console.log('    Description: Validate hook configuration and script paths');
  console.log('    Examples:');
  console.log('      fspec validate-hooks');
  console.log('');
  console.log('  ' + chalk.cyan('fspec add-hook <event> <name>'));
  console.log('    Options:');
  console.log('      --command <path>                 Path to hook script');
  console.log('      --blocking                       Make hook blocking');
  console.log('      --timeout <seconds>              Timeout in seconds');
  console.log('    Examples:');
  console.log(
    '      fspec add-hook pre-implementing lint --command spec/hooks/lint.sh --blocking'
  );
  console.log(
    '      fspec add-hook post-implementing test --command spec/hooks/test.sh'
  );
  console.log('');
  console.log('  ' + chalk.cyan('fspec remove-hook <event> <name>'));
  console.log('    Examples:');
  console.log('      fspec remove-hook pre-implementing lint');
  console.log('      fspec remove-hook post-implementing test');
  console.log('');

  console.log(chalk.bold('HOOK CONFIGURATION'));
  console.log('  Configuration file: ' + chalk.cyan('spec/fspec-hooks.json'));
  console.log('');
  console.log('  Event naming pattern:');
  console.log('    pre-<command>   - Before command execution');
  console.log('    post-<command>  - After command execution');
  console.log('');
  console.log('  Hook properties:');
  console.log('    name            - Unique identifier');
  console.log('    command         - Script path (relative to project root)');
  console.log('    blocking        - If true, failure prevents execution');
  console.log('    timeout         - Timeout in seconds (default: 60)');
  console.log('    condition       - Optional filters (tags, prefix, epic, estimate)');
  console.log('');

  console.log(chalk.bold('COMMON USE CASES'));
  console.log('  Quality Gates (blocking pre-hooks):');
  console.log(
    '    fspec add-hook pre-implementing validate --command spec/hooks/lint.sh --blocking'
  );
  console.log('');
  console.log('  Automated Testing (post-hooks):');
  console.log(
    '    fspec add-hook post-implementing test --command spec/hooks/test.sh'
  );
  console.log('');
  console.log('  Notifications (non-blocking post-hooks):');
  console.log(
    '    fspec add-hook post-validating notify --command spec/hooks/notify.sh'
  );
  console.log('');

  console.log(chalk.bold('DOCUMENTATION'));
  console.log('  Configuration: ' + chalk.cyan('docs/hooks/configuration.md'));
  console.log(
    '  Troubleshooting: ' + chalk.cyan('docs/hooks/troubleshooting.md')
  );
  console.log('  Examples: ' + chalk.cyan('examples/hooks/'));
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
export function handleHelpCommand(group?: string): void {
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
    case 'hooks':
    case 'hook':
    case 'lifecycle':
    case 'automation':
      displayHooksHelp();
      break;
    default:
      console.log(chalk.red(`Unknown help topic: ${group}`));
      console.log('Valid topics: specs, work, discovery, metrics, setup, hooks');
      console.log('Use ' + chalk.cyan('fspec --help') + ' for main help\n');
  }
}
