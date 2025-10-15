/**
 * Hook configuration types
 */

export interface HookCondition {
  tags?: string[];
  prefix?: string[];
  epic?: string;
  estimateMin?: number;
  estimateMax?: number;
}

export interface HookDefinition {
  name: string;
  command: string;
  blocking?: boolean;
  timeout?: number;
  condition?: HookCondition;
}

export interface GlobalConfig {
  timeout?: number;
  shell?: string;
}

export interface HookConfig {
  global?: GlobalConfig;
  hooks: Record<string, HookDefinition[]>;
}

export interface HookContext {
  workUnitId?: string;
  event: string;
  timestamp: string;
}

export interface HookExecutionResult {
  hookName: string;
  success: boolean;
  exitCode: number | null;
  stdout: string;
  stderr: string;
  timedOut: boolean;
  duration: number;
}
