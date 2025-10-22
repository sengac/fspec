/**
 * Project Management Template - Main Entry Point
 *
 * This file combines all template sections into the complete Project Management Guidelines.
 * NO filesystem reads - all content is embedded as TypeScript string literals.
 */

import type { AgentConfig } from './agentRegistry';
import { getIntroSection } from './projectManagementSections/intro';
import { getProjectManagementSection } from './projectManagementSections/projectManagement';
import { getReverseAcddSection } from './projectManagementSections/reverseAcdd';
import { getSpecificationWorkflowSection } from './projectManagementSections/specificationWorkflow';
import { getGherkinRequirementsSection } from './projectManagementSections/gherkinRequirements';
import { getCoverageTrackingSection } from './projectManagementSections/coverageTracking';
import { getFileStructureSection } from './projectManagementSections/fileStructure';
import { getPrefillDetectionSection } from './projectManagementSections/prefillDetection';
import { getTemporalOrderingSection } from './projectManagementSections/temporalOrdering';
import { getEstimationValidationSection } from './projectManagementSections/estimationValidation';
import { getFormattingSection } from './projectManagementSections/formatting';
import { getEnforcementSection } from './projectManagementSections/enforcement';
import { getEffectiveScenariosSection } from './projectManagementSections/effectiveScenarios';
import { getMappingToTestsSection } from './projectManagementSections/mappingToTests';
import { getUpdatingSpecsSection } from './projectManagementSections/updatingSpecs';
import { getDogfoodingSection } from './projectManagementSections/dogfooding';
import { getJsonBackedSystemSection } from './projectManagementSections/jsonBackedSystem';
import { getAttachmentsSection } from './projectManagementSections/attachments';
import { getLifecycleHooksSection } from './projectManagementSections/lifecycleHooks';
import { getVirtualHooksSection } from './projectManagementSections/virtualHooks';
import { getGitCheckpointsSection } from './projectManagementSections/gitCheckpoints';
import { getReferencesSection } from './projectManagementSections/references';

/**
 * Helper function to generate system-reminder examples in the correct format for each agent
 */
export function formatSystemReminder(
  content: string,
  agent: AgentConfig
): string {
  if (agent.supportsSystemReminders) {
    return `<system-reminder>\n${content}\n</system-reminder>`;
  }

  // Transform to visible instruction for non-Claude agents
  const supportsEmoji =
    agent.category === 'ide' || agent.category === 'extension';
  const prefix = supportsEmoji ? '**⚠️ IMPORTANT:**' : '**IMPORTANT:**';

  const lines = content.trim().split('\n');
  const title = lines[0].replace(/^(CRITICAL|WARNING|NOTE|IMPORTANT):?\s*/, '');

  // Extract DO NOT and ALWAYS sections
  const doNotMatch = content.match(/DO NOT (.+?)(?=\n\nALWAYS|\n\nNext|$)/s);
  const alwaysMatch = content.match(/ALWAYS (.+?)(?=\n\nDO NOT|\n\nNext|$)/s);

  let result = `${prefix} ${title}\n\n`;

  // Add body (everything that's not DO NOT/ALWAYS)
  const body = lines
    .slice(1)
    .join('\n')
    .replace(/DO NOT .+/gs, '')
    .replace(/ALWAYS .+/gs, '')
    .trim();

  if (body) {
    result += `${body}\n\n`;
  }

  // Add DO NOT section
  if (doNotMatch) {
    result += `**DO NOT:** ${doNotMatch[1].trim()}\n\n`;
  }

  // Add ALWAYS section
  if (alwaysMatch) {
    result += `**ALWAYS:** ${alwaysMatch[1].trim()}\n\n`;
  }

  return result.trim();
}

/**
 * Get the complete Project Management and Specification Guidelines template.
 * Combines all sections - no filesystem reads required.
 */
export function getProjectManagementTemplate(agent: AgentConfig): string {
  return [
    getIntroSection(),
    getProjectManagementSection(),
    getReverseAcddSection(),
    getSpecificationWorkflowSection(),
    getGherkinRequirementsSection(),
    getCoverageTrackingSection(),
    getFileStructureSection(),
    getPrefillDetectionSection(agent),
    getTemporalOrderingSection(),
    getEstimationValidationSection(agent),
    getFormattingSection(),
    getEnforcementSection(),
    getEffectiveScenariosSection(),
    getMappingToTestsSection(),
    getUpdatingSpecsSection(),
    getDogfoodingSection(),
    getJsonBackedSystemSection(),
    getAttachmentsSection(),
    getLifecycleHooksSection(agent),
    getVirtualHooksSection(agent),
    getGitCheckpointsSection(agent),
    getReferencesSection(),
  ].join('\n\n');
}
