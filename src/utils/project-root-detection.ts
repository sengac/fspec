import { access, mkdir, stat } from 'fs/promises';
import { join, dirname } from 'path';
import { constants } from 'fs';

/**
 * Project boundary markers that indicate the root of a project.
 * Searched in order from cwd upward.
 */
const BOUNDARY_MARKERS = ['.git', 'package.json', '.gitignore', 'Cargo.toml', 'pyproject.toml'];

/**
 * Maximum number of parent directories to search upward.
 * Prevents excessive filesystem traversal.
 */
const MAX_SEARCH_DEPTH = 10;

/**
 * Finds or creates the spec/ directory at the appropriate project root location.
 *
 * Search algorithm:
 * 1. Check if spec/ directory already exists within project boundary → use it
 * 2. Search upward from cwd for project boundary markers (stops at first match)
 * 3. If boundary marker found → create spec/ at that location
 * 4. If no boundary marker found → create spec/ at cwd (fallback)
 *
 * Safety: If cwd contains 'spec' directory already, use cwd (test isolation)
 *
 * @param cwd - Current working directory to start search from
 * @returns Absolute path to spec/ directory
 */
export async function findOrCreateSpecDirectory(cwd: string): Promise<string> {
  try {
    // Safety check: if spec/ already exists at cwd, use it (test isolation)
    const cwdSpecPath = join(cwd, 'spec');
    try {
      const stats = await stat(cwdSpecPath);
      if (stats.isDirectory()) {
        return cwdSpecPath;
      }
    } catch {
      // spec/ doesn't exist at cwd, continue with normal logic
    }

    // First, search for existing spec/ directory within project boundary
    const existingSpec = await findExistingSpecDirectory(cwd);
    if (existingSpec) {
      return existingSpec;
    }

    // If no existing spec/, find project root and create spec/ there
    const projectRoot = await findProjectRoot(cwd);
    const specPath = join(projectRoot, 'spec');

    // Create spec/ directory if it doesn't exist
    await mkdir(specPath, { recursive: true });

    return specPath;
  } catch (error: unknown) {
    // Graceful fallback: on any error (permission, filesystem, etc.), create spec/ at cwd
    const fallbackSpecPath = join(cwd, 'spec');
    await mkdir(fallbackSpecPath, { recursive: true });
    return fallbackSpecPath;
  }
}

/**
 * Searches upward from cwd for an existing spec/ directory within project boundary.
 *
 * @param cwd - Current working directory to start search from
 * @returns Absolute path to existing spec/ directory, or null if not found
 */
async function findExistingSpecDirectory(cwd: string): Promise<string | null> {
  let currentDir = cwd;
  let depth = 0;

  while (depth < MAX_SEARCH_DEPTH) {
    const specPath = join(currentDir, 'spec');

    try {
      // Check if spec/ directory exists
      const stats = await stat(specPath);
      if (stats.isDirectory()) {
        // Check if we're within a project boundary
        const hasMarker = await hasProjectBoundaryMarker(currentDir);
        if (hasMarker) {
          return specPath;
        }
      }
    } catch {
      // spec/ doesn't exist at this level, continue searching
    }

    // Move up one directory
    const parentDir = dirname(currentDir);

    // Stop if we've reached the filesystem root
    if (parentDir === currentDir) {
      break;
    }

    currentDir = parentDir;
    depth++;
  }

  return null;
}

/**
 * Finds the project root by searching upward for boundary markers.
 *
 * @param cwd - Current working directory to start search from
 * @returns Absolute path to project root (or cwd if no boundary found)
 */
async function findProjectRoot(cwd: string): Promise<string> {
  let currentDir = cwd;
  let depth = 0;

  while (depth < MAX_SEARCH_DEPTH) {
    // Check if current directory has any boundary marker
    const hasMarker = await hasProjectBoundaryMarker(currentDir);
    if (hasMarker) {
      return currentDir;
    }

    // Move up one directory
    const parentDir = dirname(currentDir);

    // Stop if we've reached the filesystem root
    if (parentDir === currentDir) {
      break;
    }

    currentDir = parentDir;
    depth++;
  }

  // No boundary marker found, return cwd as fallback
  return cwd;
}

/**
 * Checks if a directory contains any project boundary marker.
 *
 * @param dir - Directory to check for boundary markers
 * @returns True if any boundary marker exists
 */
async function hasProjectBoundaryMarker(dir: string): Promise<boolean> {
  for (const marker of BOUNDARY_MARKERS) {
    const markerPath = join(dir, marker);
    try {
      await access(markerPath, constants.F_OK);
      return true; // Marker exists
    } catch {
      // Marker doesn't exist, try next one
    }
  }
  return false;
}
