/**
 * Slash Command Template - Main Entry Point
 *
 * This file combines all template sections into the complete fspec slash command.
 * NO filesystem reads - all content is embedded as TypeScript string literals.
 */

import {
  getHeaderSection,
  getPersonaIntroSection,
} from './slashCommandSections/header';
import { getAcddConceptSection } from './slashCommandSections/acddConcept';
import { getLoadContextSection } from './slashCommandSections/loadContext';
import { getBootstrapFoundationSection } from './slashCommandSections/bootstrapFoundation';
import { getBigPictureEventStormSection } from './slashCommandSections/bigPictureEventStorm';
import { getEventStormSection } from './slashCommandSections/eventStorm';
import { getExampleMappingSection } from './slashCommandSections/exampleMapping';
import { getEstimationSection } from './slashCommandSections/estimation';
import { getKanbanWorkflowSection } from './slashCommandSections/kanbanWorkflow';
import { getToolConfigurationSection } from './slashCommandSections/toolConfiguration';
import { getCriticalRulesSection } from './slashCommandSections/criticalRules';
import { getAcddWorkflowExampleSection } from './slashCommandSections/acddWorkflowExample';
import { getMonitoringProgressSection } from './slashCommandSections/monitoringProgress';
import { getAcddPrinciplesSection } from './slashCommandSections/acddPrinciples';
import { getCoverageTrackingSection } from './slashCommandSections/coverageTracking';
import { getReadyToStartSection } from './slashCommandSections/readyToStart';

/**
 * Get the minimal fspec slash command template.
 * Returns ONLY the header (title + IMMEDIATELY section + two commands).
 * All other content is generated dynamically by 'fspec bootstrap'.
 */
export function getSlashCommandTemplate(): string {
  return getHeaderSection();
}

/**
 * Get the complete workflow documentation (used by bootstrap command).
 * This is what was previously embedded in the template.
 */
export function getCompleteWorkflowDocumentation(): string {
  return [
    getPersonaIntroSection(),
    getAcddConceptSection(),
    getLoadContextSection(),
    getBootstrapFoundationSection(),
    getBigPictureEventStormSection(),
    getEventStormSection(),
    getExampleMappingSection(),
    getEstimationSection(),
    getKanbanWorkflowSection(),
    getToolConfigurationSection(),
    getCriticalRulesSection(),
    getAcddWorkflowExampleSection(),
    getCoverageTrackingSection(),
    getMonitoringProgressSection(),
    getAcddPrinciplesSection(),
    getReadyToStartSection(),
  ].join('\n');
}
