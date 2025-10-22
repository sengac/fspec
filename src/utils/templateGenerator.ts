/**
 * Template Generator - Transform base templates for different agents
 */

import type { AgentConfig } from './agentRegistry';

// Embedded base template (replaces filesystem read)
const BASE_AGENT_TEMPLATE = `# {{AGENT_NAME}} Development Guidelines for fspec

This document provides guidelines for AI assistants (particularly {{AGENT_NAME}}) working on the **fspec codebase**.

**For using fspec commands and ACDD workflow**: See [spec/{{DOC_TEMPLATE}}](spec/{{DOC_TEMPLATE}})

For complete project context, refer to \`spec/{{DOC_TEMPLATE}}\` for fspec usage.

Slash commands are available at {{SLASH_COMMAND_PATH}}.
`;

/**
 * Generate agent-specific documentation from base template
 */
export async function generateAgentDoc(agent: AgentConfig): Promise<string> {
  // Import template
  const { getProjectManagementTemplate } = await import(
    './projectManagementTemplate'
  );

  // Get full Project Management Guidelines template with agent-specific examples (2069 lines)
  let content = getProjectManagementTemplate(agent);

  // Apply agent-specific transformations
  content = stripSystemReminders(content, agent);
  content = removeMetaCognitivePrompts(content, agent);
  content = replacePlaceholders(content, agent);

  return content;
}

/**
 * Strip <system-reminder> tags and transform to visible instructions
 */
export function stripSystemReminders(
  content: string,
  agent: AgentConfig
): string {
  if (agent.supportsSystemReminders) {
    return content; // Preserve for Claude Code
  }

  // Replace <system-reminder> with visible instruction block
  // Process iteratively to handle nested tags (innermost first)
  let result = content;
  let hasMatch = true;

  while (hasMatch) {
    const prevResult = result;
    result = result.replace(
      /<system-reminder>([\s\S]*?)<\/system-reminder>/,
      (_, inner) => {
        return transformToVisibleInstruction(inner, agent);
      }
    );
    hasMatch = result !== prevResult;
  }

  return result;
}

/**
 * Transform system-reminder content to visible Markdown instruction
 */
function transformToVisibleInstruction(
  content: string,
  agent: AgentConfig
): string {
  const supportsEmoji =
    agent.category === 'ide' || agent.category === 'extension';
  const prefix = supportsEmoji ? '**⚠️ IMPORTANT:**' : '**IMPORTANT:**';

  // Extract title (first line, typically all caps)
  const lines = content.trim().split('\n');
  const title = lines[0].replace(/^(CRITICAL|WARNING|NOTE|IMPORTANT):?\s*/, '');

  // Extract DO NOT and ALWAYS sections
  const doNotMatch = content.match(/DO NOT (.+?)(?=ALWAYS|$)/s);
  const alwaysMatch = content.match(/ALWAYS (.+?)(?=DO NOT|$)/s);

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

  return result;
}

/**
 * Remove meta-cognitive prompts for agents that don't support them
 */
export function removeMetaCognitivePrompts(
  content: string,
  agent: AgentConfig
): string {
  if (agent.supportsMetaCognition) {
    return content; // Preserve for IDE/extension agents
  }

  // Remove meta-cognitive phrases for CLI-only agents
  const patterns = [
    /ultrathink your next steps/gi,
    /ultrathink on/gi,
    /ultrathink/gi,
    /deeply consider/gi,
    /take a moment to reflect/gi,
  ];

  let result = content;
  for (const pattern of patterns) {
    result = result.replace(pattern, '');
  }

  // Clean up extra whitespace
  result = result.replace(/\n{3,}/g, '\n\n');

  return result;
}

/**
 * Replace template placeholders with agent-specific values
 */
export function replacePlaceholders(
  content: string,
  agent: AgentConfig
): string {
  return content
    .replace(/\{\{AGENT_NAME\}\}/g, agent.name)
    .replace(/\{\{DOC_TEMPLATE\}\}/g, agent.docTemplate)
    .replace(/\{\{SLASH_COMMAND_PATH\}\}/g, agent.slashCommandPath)
    .replace(/\{\{AGENT_ID\}\}/g, agent.id);
}
