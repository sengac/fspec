/**
 * Shared file system operations for testing.
 *
 * Provides reusable file I/O utilities that follow SOLID/DRY principles.
 * All test file operations should go through these utilities.
 */

import { writeFile, mkdir, readFile } from 'fs/promises';
import { join } from 'path';

/**
 * Write JSON data to a file with consistent formatting.
 */
export async function writeJsonTestFile(
  filePath: string,
  data: unknown
): Promise<void> {
  await writeFile(filePath, JSON.stringify(data, null, 2));
}

/**
 * Write text content to a file.
 */
export async function writeTextFile(
  filePath: string,
  content: string
): Promise<void> {
  await writeFile(filePath, content);
}

/**
 * Read and parse JSON from a test file.
 */
export async function readJsonTestFile<T = unknown>(
  filePath: string
): Promise<T> {
  const content = await readFile(filePath, 'utf-8');
  return JSON.parse(content) as T;
}

/**
 * Ensure a directory exists, creating it recursively if needed.
 */
export async function ensureTestDirectory(dirPath: string): Promise<void> {
  await mkdir(dirPath, { recursive: true });
}

/**
 * Create a file in a directory, ensuring the directory exists first.
 */
export async function createTestFile(
  dirPath: string,
  fileName: string,
  content: string
): Promise<string> {
  await ensureTestDirectory(dirPath);
  const filePath = join(dirPath, fileName);
  await writeFile(filePath, content);
  return filePath;
}

/**
 * Create a JSON file in a directory, ensuring the directory exists first.
 */
export async function createJsonTestFile(
  dirPath: string,
  fileName: string,
  data: unknown
): Promise<string> {
  await ensureTestDirectory(dirPath);
  const filePath = join(dirPath, fileName);
  await writeJsonTestFile(filePath, data);
  return filePath;
}

/**
 * Create multiple test files from a configuration object.
 */
export async function createTestFiles(
  baseDir: string,
  files: Record<string, { content?: string; data?: unknown }>
): Promise<Record<string, string>> {
  const createdFiles: Record<string, string> = {};

  for (const [relativePath, fileConfig] of Object.entries(files)) {
    const fullPath = join(baseDir, relativePath);
    const dirPath = join(fullPath, '..');

    await ensureTestDirectory(dirPath);

    if (fileConfig.content !== undefined) {
      await writeFile(fullPath, fileConfig.content);
    } else if (fileConfig.data !== undefined) {
      await writeJsonTestFile(fullPath, fileConfig.data);
    }

    createdFiles[relativePath] = fullPath;
  }

  return createdFiles;
}
