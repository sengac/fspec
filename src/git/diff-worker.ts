/**
 * Worker thread for git diff operations
 *
 * Coverage:
 * - GIT-007: Diff loading system has rendering bugs and performance issues
 *
 * This worker offloads git diff operations from the main thread to prevent
 * UX hangs and ensure React rendering remains responsive.
 */

import { parentPort } from 'worker_threads';
import { getFileDiff } from './diff.js';

interface DiffRequest {
  id: string;
  cwd: string;
  filepath: string;
}

interface DiffResponse {
  id: string;
  diff?: string;
  error?: string;
}

if (!parentPort) {
  throw new Error('This module must be run as a worker thread');
}

parentPort.on('message', async (request: DiffRequest) => {
  try {
    const diff = await getFileDiff(request.cwd, request.filepath);
    const response: DiffResponse = {
      id: request.id,
      diff: diff || 'No changes to display',
    };
    parentPort!.postMessage(response);
  } catch (error) {
    const response: DiffResponse = {
      id: request.id,
      error: error instanceof Error ? error.message : 'Error loading diff',
    };
    parentPort!.postMessage(response);
  }
});
