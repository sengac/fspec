#!/usr/bin/env node

// PERF-001: Clear React 19's performance measure buffer periodically
// React 19's reconciler uses performance.measure() for profiling, which accumulates
// entries over time. After 1,000,000 entries, Node.js emits a warning about potential
// memory leaks. This interval clears the buffer every 30 seconds to prevent the warning.
// NOTE: unref() ensures this interval doesn't keep the process alive - critical for
// CLI commands that should exit after completion (e.g., when run by AI agent tools).
import { performance } from 'perf_hooks';

setInterval(() => {
  performance.clearMeasures();
}, 30000).unref();

// LOG-003: Capture all console methods and redirect to winston logger
// This MUST run before any other imports that might use console to ensure all output is captured
import { initializeConsoleCapture } from './utils/console-capture';
initializeConsoleCapture();

// LOG-004: Wire up Rust tracing logs to TypeScript logger
// This MUST run early to capture all Rust logs (including session navigation)
import { initializeRustLogCapture } from './utils/rust-log-capture';
initializeRustLogCapture();

import { Command } from 'commander';
import chalk from 'chalk';
import { fileURLToPath } from 'url';
import { realpathSync } from 'fs';
import { readFileSync } from 'fs';
import { dirname, join } from 'path';
import { render } from 'ink';
import React from 'react';
import { INK_RENDER_OPTIONS } from './tui/config/inkConfig';

// Read version from package.json
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const packageJson = JSON.parse(
  readFileSync(join(__dirname, '..', 'package.json'), 'utf-8')
);
const version = packageJson.version;

// Command registration functions
import { registerAddAggregateCommand } from './commands/add-aggregate';
import { registerAddArchitectureCommand } from './commands/add-architecture';
import { registerAddArchitectureNoteCommand } from './commands/add-architecture-note';
import { registerAddAttachmentCommand } from './commands/add-attachment';
import { registerAddBoundedContextCommand } from './commands/add-bounded-context';
import { registerAddCommandCommand } from './commands/add-command';
import { registerAddDomainEventCommand } from './commands/add-domain-event';
import { registerAddExternalSystemCommand } from './commands/add-external-system';
import { registerAddHotspotCommand } from './commands/add-hotspot';
import { registerAddPolicyCommand } from './commands/add-policy';
import { registerAddAssumptionCommand } from './commands/add-assumption';
import { registerAddBackgroundCommand } from './commands/add-background';
import { registerAddCapabilityCommand } from './commands/register-add-capability';
import { registerAddPersonaCommand } from './commands/register-add-persona';
import { registerRemoveCapabilityCommand } from './commands/register-remove-capability';
import { registerRemovePersonaCommand } from './commands/register-remove-persona';
import { registerAddHookCommand } from './commands/add-hook';
import { registerCheckpointCommand } from './commands/checkpoint';
import { registerListCheckpointsCommand } from './commands/list-checkpoints';
import { registerRestoreCheckpointCommand } from './commands/restore-checkpoint';
import { registerCleanupCheckpointsCommand } from './commands/cleanup-checkpoints';
import { registerAddVirtualHookCommand } from './commands/add-virtual-hook';
import { registerListVirtualHooksCommand } from './commands/list-virtual-hooks';
import { registerRemoveVirtualHookCommand } from './commands/remove-virtual-hook';
import { registerCopyVirtualHooksCommand } from './commands/copy-virtual-hooks';
import { registerClearVirtualHooksCommand } from './commands/clear-virtual-hooks';
import { registerAddDependenciesCommand } from './commands/add-dependencies';
import { registerAddDependencyCommand } from './commands/add-dependency';
import { registerAddDiagramCommand } from './commands/add-diagram';
import { registerAddExampleCommand } from './commands/add-example';
import { registerAddQuestionCommand } from './commands/add-question';
import { registerAddRuleCommand } from './commands/add-rule';
import { registerAddScenarioCommand } from './commands/add-scenario';
import { registerAddStepCommand } from './commands/add-step';
import { registerAddTagToFeatureCommand } from './commands/add-tag-to-feature';
import { registerAddTagToScenarioCommand } from './commands/add-tag-to-scenario';
import { registerAnswerQuestionCommand } from './commands/answer-question';
import { registerAuditCoverageCommand } from './commands/audit-coverage';
import { registerAutoAdvanceCommand } from './commands/auto-advance';
import { registerBoardCommand } from './commands/display-board';
import { registerBootstrapCommand } from './commands/bootstrap';
import { registerCheckCommand } from './commands/check';
import { registerClearDependenciesCommand } from './commands/clear-dependencies';
import { registerCompareImplementationsCommand } from './commands/compare-implementations';
import { registerConfigureToolsCommand } from './commands/configure-tools';
import { registerCreateEpicCommand } from './commands/create-epic';
import { registerCreateFeatureCommand } from './commands/create-feature';
import { registerCreatePrefixCommand } from './commands/create-prefix';
import { registerCreateStoryCommand } from './commands/create-story';
import { registerCreateBugCommand } from './commands/create-bug';
import { registerCreateTaskCommand } from './commands/create-task';
import { registerDeleteDiagramCommand } from './commands/delete-diagram';
import { registerDeleteEpicCommand } from './commands/delete-epic';
import { registerDependenciesCommand } from './commands/dependencies';
import { registerDeleteFeaturesCommand } from './commands/delete-features-by-tag';
import { registerDeleteScenarioCommand } from './commands/delete-scenario';
import { registerDeleteScenariosCommand } from './commands/delete-scenarios-by-tag';
import { registerDeleteStepCommand } from './commands/delete-step';
import { registerDeleteTagCommand } from './commands/delete-tag';
import { registerDeleteWorkUnitCommand } from './commands/delete-work-unit';
import { registerDiscoverFoundationCommand } from './commands/discover-foundation';
import { registerDiscoverEventStormCommand } from './commands/discover-event-storm';
import { registerExportDependenciesCommand } from './commands/export-dependencies';
import { registerExportExampleMapCommand } from './commands/export-example-map';
import { registerExportWorkUnitsCommand } from './commands/export-work-units';
import { registerFormatCommand } from './commands/format';
import { registerGenerateCoverageCommand } from './commands/generate-coverage';
import { registerGenerateExampleMappingFromEventStormCommand } from './commands/generate-example-mapping-from-event-storm';
import { registerGenerateFoundationMdCommand } from './commands/generate-foundation-md';
import { registerGenerateScenariosCommand } from './commands/generate-scenarios';
import { registerGenerateSummaryReportCommand } from './commands/generate-summary-report';
import { registerGenerateTagsMdCommand } from './commands/generate-tags-md';
import { registerGetScenariosCommand } from './commands/get-scenarios';
import { registerImportExampleMapCommand } from './commands/import-example-map';
import { registerInitCommand } from './commands/init';
import { registerRemoveInitFilesCommand } from './commands/remove-init-files';
import { syncVersion } from './commands/sync-version';
import { registerLinkCoverageCommand } from './commands/link-coverage';
import { registerListAttachmentsCommand } from './commands/list-attachments';
import { registerListEpicsCommand } from './commands/list-epics';
import { registerListFeatureTagsCommand } from './commands/list-feature-tags';
import { registerListFeaturesCommand } from './commands/list-features';
import { registerListHooksCommand } from './commands/list-hooks';
import { registerListPrefixesCommand } from './commands/list-prefixes';
import { registerListScenarioTagsCommand } from './commands/list-scenario-tags';
import { registerListTagsCommand } from './commands/list-tags';
import { registerListWorkUnitsCommand } from './commands/list-work-units';
import { registerPrioritizeWorkUnitCommand } from './commands/prioritize-work-unit';
import { registerQueryBottlenecksCommand } from './commands/query-bottlenecks';
import { registerQueryDependencyStatsCommand } from './commands/query-dependency-stats';
import { registerQueryEstimateAccuracyCommand } from './commands/query-estimate-accuracy';
import { registerQueryOrphansCommand } from './commands/query-orphans';
import { registerQueryEstimationGuideCommand } from './commands/query-estimation-guide';
import { registerQueryExampleMappingStatsCommand } from './commands/query-example-mapping-stats';
import { registerQueryMetricsCommand } from './commands/query-metrics';
import { registerQueryWorkUnitsCommand } from './commands/query-work-units';
import { registerRecordIterationCommand } from './commands/record-iteration';
import { registerRegisterTagCommand } from './commands/register-tag';
import { registerRemoveArchitectureNoteCommand } from './commands/remove-architecture-note';
import { registerRemoveAttachmentCommand } from './commands/remove-attachment';
import { registerRemoveHookCommand } from './commands/remove-hook';
import { registerRemoveDependencyCommand } from './commands/remove-dependency';
import { registerRemoveExampleCommand } from './commands/remove-example';
import { registerRemoveQuestionCommand } from './commands/remove-question';
import { registerRemoveRuleCommand } from './commands/remove-rule';
import { registerRestoreRuleCommand } from './commands/restore-rule';
import { registerRestoreExampleCommand } from './commands/restore-example';
import { registerRestoreQuestionCommand } from './commands/restore-question';
import { registerRestoreArchitectureNoteCommand } from './commands/restore-architecture-note';
import { registerCompactWorkUnitCommand } from './commands/compact-work-unit';
import { registerShowDeletedCommand } from './commands/show-deleted';
import { registerRemoveTagFromFeatureCommand } from './commands/remove-tag-from-feature';
import { registerRemoveTagFromScenarioCommand } from './commands/remove-tag-from-scenario';
import { registerReportBugToGitHubCommand } from './commands/report-bug-to-github';
import { registerRepairWorkUnitsCommand } from './commands/repair-work-units';
import { registerRetagCommand } from './commands/retag';
import { registerReverseCommand } from './commands/reverse';
import { registerResearchCommand } from './commands/research';
import { registerSearchImplementationCommand } from './commands/search-implementation';
import { registerSearchScenariosCommand } from './commands/search-scenarios';
import { registerSetUserStoryCommand } from './commands/set-user-story';
import { registerShowAcceptanceCriteriaCommand } from './commands/show-acceptance-criteria';
import { registerShowCoverageCommand } from './commands/show-coverage';
import { registerShowEpicCommand } from './commands/show-epic';
import { registerShowEventStormCommand } from './commands/show-event-storm';
import { registerShowFeatureCommand } from './commands/show-feature';
import { registerShowFoundationCommand } from './commands/show-foundation';
import { registerShowFoundationEventStormCommand } from './commands/show-foundation-event-storm';
import { registerAddFoundationBoundedContextCommand } from './commands/add-foundation-bounded-context';
import { registerAddAggregateToFoundationCommand } from './commands/add-aggregate-to-foundation';
import { registerAddDomainEventToFoundationCommand } from './commands/add-domain-event-to-foundation';
import { registerAddCommandToFoundationCommand } from './commands/add-command-to-foundation';
import { registerShowTestPatternsCommand } from './commands/show-test-patterns';
import { registerShowWorkUnitCommand } from './commands/show-work-unit';
import { registerReviewCommand } from './commands/review';
import { registerSuggestDependenciesCommand } from './commands/suggest-dependencies';
import { registerTagStatsCommand } from './commands/tag-stats';
import { registerUnlinkCoverageCommand } from './commands/unlink-coverage';
import { registerUpdateFoundationCommand } from './commands/update-foundation';
import { registerUpdatePrefixCommand } from './commands/update-prefix';
import { registerUpdateScenarioCommand } from './commands/update-scenario';
import { registerUpdateStepCommand } from './commands/update-step';
import { registerUpdateTagCommand } from './commands/update-tag';
import { registerUpdateWorkUnitCommand } from './commands/update-work-unit';
import { registerUpdateWorkUnitEstimateCommand } from './commands/update-work-unit-estimate';
import { registerUpdateWorkUnitStatusCommand } from './commands/update-work-unit-status';
import { registerValidateCommand } from './commands/validate';
import { registerValidateFoundationSchemaCommand } from './commands/validate-foundation-schema';
import { registerValidateHooksCommand } from './commands/validate-hooks';
import { registerValidateSpecAlignmentCommand } from './commands/validate-spec-alignment';
import { registerValidateTagsCommand } from './commands/validate-tags';
import { registerValidateWorkUnitsCommand } from './commands/validate-work-units';
import { registerWorkflowAutomationCommand } from './commands/workflow-automation';

// Help functions
import { handleHelpCommand } from './help';
import { handleCustomHelp } from './utils/help-interceptor';

// TUI components
import { BoardView } from './tui/components/BoardView';

const program = new Command();

// Program configuration
program.name('fspec');
program.description('Feature Specification & Project Management for AI Agents');

// Only set version if available
if (version) {
  program.version(version);
}

program
  .configureHelp({
    helpWidth: 100,
  })
  .helpCommand(false)
  .helpOption('-h, --help', 'Display help for command');

// Add custom help command
program
  .command('help')
  .description('Display help for command groups')
  .argument('[group]', 'Help topic: spec, tags, foundation, query, project')
  .action((group?: string) => handleHelpCommand(group, version));

// Register all commands
registerAddAggregateCommand(program);
registerAddArchitectureCommand(program);
registerAddArchitectureNoteCommand(program);
registerAddAttachmentCommand(program);
registerAddBoundedContextCommand(program);
registerBootstrapCommand(program);
registerAddCommandCommand(program);
registerAddDomainEventCommand(program);
registerAddExternalSystemCommand(program);
registerAddHotspotCommand(program);
registerAddPolicyCommand(program);
registerAddAssumptionCommand(program);
registerAddBackgroundCommand(program);
registerAddCapabilityCommand(program);
registerAddPersonaCommand(program);
registerRemoveCapabilityCommand(program);
registerRemovePersonaCommand(program);
registerAddHookCommand(program);
registerCheckpointCommand(program);
registerListCheckpointsCommand(program);
registerRestoreCheckpointCommand(program);
registerCleanupCheckpointsCommand(program);
registerAddVirtualHookCommand(program);
registerListVirtualHooksCommand(program);
registerRemoveVirtualHookCommand(program);
registerCopyVirtualHooksCommand(program);
registerClearVirtualHooksCommand(program);
registerAddDependenciesCommand(program);
registerAddDependencyCommand(program);
registerAddDiagramCommand(program);
registerAddExampleCommand(program);
registerAddQuestionCommand(program);
registerAddRuleCommand(program);
registerAddScenarioCommand(program);
registerAddStepCommand(program);
registerAddTagToFeatureCommand(program);
registerAddTagToScenarioCommand(program);
registerAnswerQuestionCommand(program);
registerAuditCoverageCommand(program);
registerAutoAdvanceCommand(program);
registerBoardCommand(program);
registerCheckCommand(program);
registerClearDependenciesCommand(program);
registerCompareImplementationsCommand(program);
registerConfigureToolsCommand(program);
registerCreateEpicCommand(program);
registerCreateFeatureCommand(program);
registerCreatePrefixCommand(program);
registerCreateStoryCommand(program);
registerCreateBugCommand(program);
registerCreateTaskCommand(program);
registerDeleteDiagramCommand(program);
registerDeleteEpicCommand(program);
registerDependenciesCommand(program);
registerDeleteFeaturesCommand(program);
registerDeleteScenarioCommand(program);
registerDeleteScenariosCommand(program);
registerDeleteStepCommand(program);
registerDeleteTagCommand(program);
registerDeleteWorkUnitCommand(program);
registerDiscoverFoundationCommand(program);
registerDiscoverEventStormCommand(program);
registerExportDependenciesCommand(program);
registerExportExampleMapCommand(program);
registerExportWorkUnitsCommand(program);
registerFormatCommand(program);
registerGenerateCoverageCommand(program);
registerGenerateExampleMappingFromEventStormCommand(program);
registerGenerateFoundationMdCommand(program);
registerGenerateScenariosCommand(program);
registerGenerateSummaryReportCommand(program);
registerGenerateTagsMdCommand(program);
registerGetScenariosCommand(program);
registerImportExampleMapCommand(program);
registerInitCommand(program);
registerRemoveInitFilesCommand(program);
registerLinkCoverageCommand(program);
registerListAttachmentsCommand(program);
registerListEpicsCommand(program);
registerListFeatureTagsCommand(program);
registerListFeaturesCommand(program);
registerListHooksCommand(program);
registerListPrefixesCommand(program);
registerListScenarioTagsCommand(program);
registerListTagsCommand(program);
registerListWorkUnitsCommand(program);
registerPrioritizeWorkUnitCommand(program);
registerQueryBottlenecksCommand(program);
registerQueryDependencyStatsCommand(program);
registerQueryEstimateAccuracyCommand(program);
registerQueryEstimationGuideCommand(program);
registerQueryOrphansCommand(program);
registerQueryExampleMappingStatsCommand(program);
registerQueryMetricsCommand(program);
registerQueryWorkUnitsCommand(program);
registerRecordIterationCommand(program);
registerRegisterTagCommand(program);
registerRemoveArchitectureNoteCommand(program);
registerRemoveAttachmentCommand(program);
registerRemoveHookCommand(program);
registerRemoveDependencyCommand(program);
registerRemoveExampleCommand(program);
registerRemoveQuestionCommand(program);
registerRemoveRuleCommand(program);
registerRestoreRuleCommand(program);
registerRestoreExampleCommand(program);
registerRestoreQuestionCommand(program);
registerRestoreArchitectureNoteCommand(program);
registerCompactWorkUnitCommand(program);
registerShowDeletedCommand(program);
registerRemoveTagFromFeatureCommand(program);
registerRemoveTagFromScenarioCommand(program);
registerReportBugToGitHubCommand(program);
registerRepairWorkUnitsCommand(program);
registerRetagCommand(program);
registerReverseCommand(program);
registerResearchCommand(program);
registerSearchImplementationCommand(program);
registerSearchScenariosCommand(program);
registerSetUserStoryCommand(program);
registerShowAcceptanceCriteriaCommand(program);
registerShowCoverageCommand(program);
registerShowEpicCommand(program);
registerShowEventStormCommand(program);
registerShowFeatureCommand(program);
registerShowFoundationCommand(program);
registerShowFoundationEventStormCommand(program);
registerAddFoundationBoundedContextCommand(program);
registerAddAggregateToFoundationCommand(program);
registerAddDomainEventToFoundationCommand(program);
registerAddCommandToFoundationCommand(program);
registerShowTestPatternsCommand(program);
registerShowWorkUnitCommand(program);
registerReviewCommand(program);
registerSuggestDependenciesCommand(program);
registerTagStatsCommand(program);
registerUnlinkCoverageCommand(program);
registerUpdateFoundationCommand(program);
registerUpdatePrefixCommand(program);
registerUpdateScenarioCommand(program);
registerUpdateStepCommand(program);
registerUpdateTagCommand(program);
registerUpdateWorkUnitCommand(program);
registerUpdateWorkUnitEstimateCommand(program);
registerUpdateWorkUnitStatusCommand(program);
registerValidateCommand(program);
registerValidateFoundationSchemaCommand(program);
registerValidateHooksCommand(program);
registerValidateSpecAlignmentCommand(program);
registerValidateTagsCommand(program);
registerValidateWorkUnitsCommand(program);
registerWorkflowAutomationCommand(program);

async function main(): Promise<void> {
  // Handle --sync-version BEFORE any other processing
  // This must run first to check version and update files if needed
  const syncVersionIndex = process.argv.findIndex(arg =>
    arg.startsWith('--sync-version')
  );
  if (syncVersionIndex !== -1) {
    const versionArg = process.argv[syncVersionIndex];
    const embeddedVersion =
      versionArg.split('=')[1] || process.argv[syncVersionIndex + 1];

    if (embeddedVersion) {
      const exitCode = await syncVersion({ embeddedVersion });
      process.exit(exitCode);
    }
  }

  // Launch interactive TUI when no arguments provided
  // process.argv = ['node', '/path/to/index.js'] when no args
  if (process.argv.length === 2) {
    // Check if stdin supports raw mode (required for Ink)
    // Skip TUI in CI environments or when stdin is not a TTY
    if (!process.stdin.isTTY || process.env.CI === 'true') {
      console.error(
        chalk.yellow('Interactive TUI requires a TTY environment.')
      );
      console.error(
        chalk.yellow('Run with a command or use --help for available commands.')
      );
      process.exit(1);
    }

    const { waitUntilExit } = render(
      React.createElement(BoardView, {
        onExit: () => {
          process.exit(0);
        },
      }),
      {
        // Enable mouse events (trackpad, scroll wheel, clicks)
        stdin: process.stdin,
        stdout: process.stdout,
        // Use shared Ink config to ensure animation timing stays in sync
        ...INK_RENDER_OPTIONS,
      }
    );
    await waitUntilExit();
    return;
  }

  // Handle custom help before Commander.js processes arguments
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
  main().catch(error => {
    console.error(chalk.red('Fatal error:'), error.message);
    process.exit(1);
  });
}
