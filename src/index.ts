#!/usr/bin/env node

import { Command } from 'commander';
import chalk from 'chalk';
import { fileURLToPath } from 'url';
import { realpathSync } from 'fs';
import { readFileSync } from 'fs';
import { dirname, join } from 'path';

// Read version from package.json
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const packageJson = JSON.parse(
  readFileSync(join(__dirname, '..', 'package.json'), 'utf-8')
);
const version = packageJson.version;

// Command registration functions
import { registerAddArchitectureCommand } from './commands/add-architecture';
import { registerAddArchitectureNoteCommand } from './commands/add-architecture-note';
import { registerAddAttachmentCommand } from './commands/add-attachment';
import { registerAddAssumptionCommand } from './commands/add-assumption';
import { registerAddBackgroundCommand } from './commands/add-background';
import { registerAddHookCommand } from './commands/add-hook';
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
import { registerCheckCommand } from './commands/check';
import { registerClearDependenciesCommand } from './commands/clear-dependencies';
import { registerCreateEpicCommand } from './commands/create-epic';
import { registerCreateFeatureCommand } from './commands/create-feature';
import { registerCreatePrefixCommand } from './commands/create-prefix';
import { registerCreateWorkUnitCommand } from './commands/create-work-unit';
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
import { registerExportDependenciesCommand } from './commands/export-dependencies';
import { registerExportExampleMapCommand } from './commands/export-example-map';
import { registerExportWorkUnitsCommand } from './commands/export-work-units';
import { registerFormatCommand } from './commands/format';
import { registerGenerateCoverageCommand } from './commands/generate-coverage';
import { registerGenerateFoundationMdCommand } from './commands/generate-foundation-md';
import { registerGenerateScenariosCommand } from './commands/generate-scenarios';
import { registerGenerateSummaryReportCommand } from './commands/generate-summary-report';
import { registerGenerateTagsMdCommand } from './commands/generate-tags-md';
import { registerGetScenariosCommand } from './commands/get-scenarios';
import { registerImportExampleMapCommand } from './commands/import-example-map';
import { registerInitCommand } from './commands/init';
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
import { registerRecordMetricCommand } from './commands/record-metric';
import { registerRecordTokensCommand } from './commands/record-tokens';
import { registerRegisterTagCommand } from './commands/register-tag';
import { registerRemoveArchitectureNoteCommand } from './commands/remove-architecture-note';
import { registerRemoveAttachmentCommand } from './commands/remove-attachment';
import { registerRemoveHookCommand } from './commands/remove-hook';
import { registerRemoveDependencyCommand } from './commands/remove-dependency';
import { registerRemoveExampleCommand } from './commands/remove-example';
import { registerRemoveQuestionCommand } from './commands/remove-question';
import { registerRemoveRuleCommand } from './commands/remove-rule';
import { registerRemoveTagFromFeatureCommand } from './commands/remove-tag-from-feature';
import { registerRemoveTagFromScenarioCommand } from './commands/remove-tag-from-scenario';
import { registerRepairWorkUnitsCommand } from './commands/repair-work-units';
import { registerRetagCommand } from './commands/retag';
import { registerSetUserStoryCommand } from './commands/set-user-story';
import { registerShowAcceptanceCriteriaCommand } from './commands/show-acceptance-criteria';
import { registerShowCoverageCommand } from './commands/show-coverage';
import { registerShowEpicCommand } from './commands/show-epic';
import { registerShowFeatureCommand } from './commands/show-feature';
import { registerShowFoundationCommand } from './commands/show-foundation';
import { registerShowWorkUnitCommand } from './commands/show-work-unit';
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
import { displayCustomHelpWithNote, handleHelpCommand } from './help';
import { handleCustomHelp } from './utils/help-interceptor';

const program = new Command();

// Program configuration
program
  .name('fspec')
  .description('Feature Specification & Project Management for AI Agents')
  .version(version)
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
  .action(handleHelpCommand);

// Register all commands
registerAddArchitectureCommand(program);
registerAddArchitectureNoteCommand(program);
registerAddAttachmentCommand(program);
registerAddAssumptionCommand(program);
registerAddBackgroundCommand(program);
registerAddHookCommand(program);
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
registerCreateEpicCommand(program);
registerCreateFeatureCommand(program);
registerCreatePrefixCommand(program);
registerCreateWorkUnitCommand(program);
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
registerExportDependenciesCommand(program);
registerExportExampleMapCommand(program);
registerExportWorkUnitsCommand(program);
registerFormatCommand(program);
registerGenerateCoverageCommand(program);
registerGenerateFoundationMdCommand(program);
registerGenerateScenariosCommand(program);
registerGenerateSummaryReportCommand(program);
registerGenerateTagsMdCommand(program);
registerGetScenariosCommand(program);
registerImportExampleMapCommand(program);
registerInitCommand(program);
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
registerRecordMetricCommand(program);
registerRecordTokensCommand(program);
registerRegisterTagCommand(program);
registerRemoveArchitectureNoteCommand(program);
registerRemoveAttachmentCommand(program);
registerRemoveHookCommand(program);
registerRemoveDependencyCommand(program);
registerRemoveExampleCommand(program);
registerRemoveQuestionCommand(program);
registerRemoveRuleCommand(program);
registerRemoveTagFromFeatureCommand(program);
registerRemoveTagFromScenarioCommand(program);
registerRepairWorkUnitsCommand(program);
registerRetagCommand(program);
registerSetUserStoryCommand(program);
registerShowAcceptanceCriteriaCommand(program);
registerShowCoverageCommand(program);
registerShowEpicCommand(program);
registerShowFeatureCommand(program);
registerShowFoundationCommand(program);
registerShowWorkUnitCommand(program);
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
  main().catch((error) => {
    console.error(chalk.red('Fatal error:'), error.message);
    process.exit(1);
  });
}
