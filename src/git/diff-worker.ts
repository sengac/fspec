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
import { getFileDiff, getCheckpointFileDiff } from './diff';

interface DiffRequest {
  id: string;
  cwd: string;
  filepath: string;
  checkpointRef?: string; // Optional: if provided, compare checkpoint vs HEAD
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
    let diff: string | null;

    // If checkpointRef is provided, compare checkpoint vs HEAD
    if (request.checkpointRef) {
      diff = await getCheckpointFileDiff(
        request.cwd,
        request.filepath,
        request.checkpointRef
      );
    } else {
      // Otherwise, compare working directory vs HEAD
      diff = await getFileDiff(request.cwd, request.filepath);
    }

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
