/**
 * Session management for Reverse ACDD strategy planning
 */

import { promises as fs } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { createHash } from 'crypto';
import type {
  ReverseSession,
  SessionPhase,
  StrategyType,
  GapAnalysis,
} from '../types/reverse-session';
import { findProjectRoot } from './project-root-detection';

export async function getSessionPath(cwd: string): Promise<string> {
  // Get project root for deterministic hashing
  const projectRoot = await findProjectRoot(cwd);

  // Create deterministic hash from project root path
  const hash = createHash('sha256')
    .update(projectRoot)
    .digest('hex')
    .substring(0, 12);

  // Store in OS temp directory with project-specific filename
  return join(tmpdir(), `fspec-reverse-${hash}.json`);
}

export async function sessionExists(cwd: string): Promise<boolean> {
  const sessionPath = await getSessionPath(cwd);
  try {
    await fs.access(sessionPath);
    return true;
  } catch {
    return false;
  }
}

export async function loadSession(cwd: string): Promise<ReverseSession | null> {
  const sessionPath = await getSessionPath(cwd);
  try {
    const content = await fs.readFile(sessionPath, 'utf-8');
    return JSON.parse(content) as ReverseSession;
  } catch {
    return null;
  }
}

export async function saveSession(
  cwd: string,
  session: ReverseSession
): Promise<void> {
  const sessionPath = await getSessionPath(cwd);

  // Write session file to OS temp directory
  await fs.writeFile(sessionPath, JSON.stringify(session, null, 2), 'utf-8');
}

export async function deleteSession(cwd: string): Promise<void> {
  const sessionPath = await getSessionPath(cwd);
  try {
    await fs.unlink(sessionPath);
  } catch {
    // Session file doesn't exist - that's fine
  }
}

export function createSession(
  phase: SessionPhase,
  gaps: GapAnalysis,
  strategy?: StrategyType,
  strategyName?: string
): ReverseSession {
  return {
    phase,
    gaps,
    strategy,
    strategyName,
    timestamp: new Date().toISOString(),
  };
}

export function transitionPhase(
  session: ReverseSession,
  newPhase: SessionPhase
): ReverseSession {
  return {
    ...session,
    phase: newPhase,
    timestamp: new Date().toISOString(),
  };
}

export function setStrategy(
  session: ReverseSession,
  strategy: StrategyType,
  strategyName: string,
  totalSteps: number
): ReverseSession {
  return {
    ...session,
    phase: 'executing',
    strategy,
    strategyName,
    currentStep: 1,
    totalSteps,
    timestamp: new Date().toISOString(),
  };
}

export function incrementStep(session: ReverseSession): ReverseSession {
  const currentStep = session.currentStep ?? 1;
  return {
    ...session,
    currentStep: currentStep + 1,
    timestamp: new Date().toISOString(),
  };
}

export function validateCompletion(session: ReverseSession): boolean {
  if (!session.currentStep || !session.totalSteps) {
    return false;
  }
  return session.currentStep >= session.totalSteps;
}
