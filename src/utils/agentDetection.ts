/**
 * Agent Detection - Auto-detect installed AI coding agents
 */

import { existsSync } from 'fs';
import { join } from 'path';
import { AGENT_REGISTRY, type AgentConfig } from './agentRegistry';

export interface DetectedAgent {
  agent: AgentConfig;
  detectedPath: string;
}

export function detectAgents(cwd: string): DetectedAgent[] {
  const detected: DetectedAgent[] = [];

  for (const agent of AGENT_REGISTRY) {
    for (const detectionPath of agent.detectionPaths) {
      const fullPath = join(cwd, detectionPath);
      if (existsSync(fullPath)) {
        detected.push({
          agent,
          detectedPath: detectionPath,
        });
        break; // Only record once per agent
      }
    }
  }

  return detected;
}

export function hasAnyAgentInstalled(cwd: string): boolean {
  return detectAgents(cwd).length > 0;
}
