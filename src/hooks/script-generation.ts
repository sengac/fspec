/**
 * Virtual hook script file generation
 *
 * Implements Rule 10: Complex commands generate script files in spec/hooks/.virtual/
 * Implements Rule 15: Git context hooks use scripts to process file lists
 */

import { writeFile, unlink, mkdir, chmod } from 'fs/promises';
import { join } from 'path';

export interface GenerateScriptOptions {
  workUnitId: string;
  hookName: string;
  command: string;
  gitContext: boolean;
  projectRoot: string;
}

export interface CleanupScriptOptions {
  workUnitId: string;
  hookName: string;
  projectRoot: string;
}

export interface GetScriptPathOptions {
  workUnitId: string;
  hookName: string;
  projectRoot: string;
}

/**
 * Get the path for a virtual hook script file
 */
export function getVirtualHookScriptPath(
  options: GetScriptPathOptions
): string {
  const { workUnitId, hookName, projectRoot } = options;
  const filename = `${workUnitId}-${hookName}.sh`;
  return join(projectRoot, 'spec', 'hooks', '.virtual', filename);
}

/**
 * Generate a script file for a virtual hook
 */
export async function generateVirtualHookScript(
  options: GenerateScriptOptions
): Promise<string> {
  const { workUnitId, hookName, command, gitContext, projectRoot } = options;

  // Create directory if it doesn't exist
  const virtualHooksDir = join(projectRoot, 'spec', 'hooks', '.virtual');
  await mkdir(virtualHooksDir, { recursive: true });

  // Generate script path
  const scriptPath = getVirtualHookScriptPath({
    workUnitId,
    hookName,
    projectRoot,
  });

  // Generate script content
  let scriptContent: string;

  if (gitContext) {
    // Script with git context - reads JSON from stdin and extracts file lists
    scriptContent = `#!/bin/bash
set -e

# Read context JSON from stdin
CONTEXT=$(cat)

# Extract staged and unstaged files from context
STAGED_FILES=$(echo "$CONTEXT" | jq -r '.stagedFiles[]? // empty' 2>/dev/null | tr '\\n' ' ')
UNSTAGED_FILES=$(echo "$CONTEXT" | jq -r '.unstagedFiles[]? // empty' 2>/dev/null | tr '\\n' ' ')

# Combine all changed files
ALL_FILES="$STAGED_FILES $UNSTAGED_FILES"

# Exit if no files to process
if [ -z "$ALL_FILES" ]; then
  echo "No changed files to process"
  exit 0
fi

# Run command with changed files
${command} $ALL_FILES
`;
  } else {
    // Simple script - just execute command
    scriptContent = `#!/bin/bash
set -e

# Execute command
${command}
`;
  }

  // Write script file
  await writeFile(scriptPath, scriptContent, { mode: 0o755 });

  // Make executable (redundant but explicit)
  await chmod(scriptPath, 0o755);

  return scriptPath;
}

/**
 * Clean up (delete) a virtual hook script file
 */
export async function cleanupVirtualHookScript(
  options: CleanupScriptOptions
): Promise<void> {
  const scriptPath = getVirtualHookScriptPath(options);

  try {
    await unlink(scriptPath);
  } catch (error: unknown) {
    // Ignore if file doesn't exist
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') {
      throw error;
    }
  }
}
