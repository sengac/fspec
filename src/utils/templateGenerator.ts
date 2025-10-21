/**
 * Template Generator - Transform base templates for different agents
 */

import { readFile, access } from 'fs/promises';
import { join } from 'path';
import type { AgentConfig } from './agentRegistry';

/**
 * Generate agent-specific documentation from base template
 */
export async function generateAgentDoc(agent: AgentConfig): Promise<string> {
  // Read base template
  const templatePath = await resolveTemplatePath('base/AGENT.md');
  let content = await readFile(templatePath, 'utf-8');

  // Apply transformations
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
    .replace(/\{\{SLASH_COMMAND_PATH\}\}/g, agent.slashCommandPath)
    .replace(/\{\{AGENT_ID\}\}/g, agent.id);
}

/**
 * Resolve template path (production or dev)
 */
async function resolveTemplatePath(relativePath: string): Promise<string> {
  // Try production path first (dist/spec/templates/)
  const prodPath = join(
    __dirname,
    '..',
    '..',
    'spec',
    'templates',
    relativePath
  );

  try {
    await access(prodPath);
    return prodPath;
  } catch {
    // Fall back to dev path (spec/templates/)
    return join(process.cwd(), 'spec', 'templates', relativePath);
  }
}
