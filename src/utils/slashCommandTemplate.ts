/**
 * Slash Command Template - Main Entry Point
 *
 * This file combines all template sections into the complete fspec slash command.
 * NO filesystem reads - all content is embedded as TypeScript string literals.
 */

import { getHeaderSection } from './slashCommandSections/header';
import { getAcddConceptSection } from './slashCommandSections/acddConcept';
import { getLoadContextSection } from './slashCommandSections/loadContext';
import { getBootstrapFoundationSection } from './slashCommandSections/bootstrapFoundation';
import { getExampleMappingSection } from './slashCommandSections/exampleMapping';
import { getEstimationSection } from './slashCommandSections/estimation';
import { getKanbanWorkflowSection } from './slashCommandSections/kanbanWorkflow';
import { getCriticalRulesSection } from './slashCommandSections/criticalRules';
import { getAcddWorkflowExampleSection } from './slashCommandSections/acddWorkflowExample';
import { getMonitoringProgressSection } from './slashCommandSections/monitoringProgress';
import { getAcddPrinciplesSection } from './slashCommandSections/acddPrinciples';
import { getCoverageTrackingSection } from './slashCommandSections/coverageTracking';
import { getReadyToStartSection } from './slashCommandSections/readyToStart';

/**
 * Get the complete fspec slash command template.
 * Combines all sections - no filesystem reads required.
 */
export function getSlashCommandTemplate(): string {
  return [
    getHeaderSection(),
    getAcddConceptSection(),
    getLoadContextSection(),
    getBootstrapFoundationSection(),
    getExampleMappingSection(),
    getEstimationSection(),
    getKanbanWorkflowSection(),
    getCriticalRulesSection(),
    getAcddWorkflowExampleSection(),
    getMonitoringProgressSection(),
    getAcddPrinciplesSection(),
    getCoverageTrackingSection(),
    getReadyToStartSection(),
  ].join('\n');
}
