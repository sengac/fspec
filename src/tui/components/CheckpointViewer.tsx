/**
 * CheckpointViewer Component - three-pane viewer for checkpoints, files, and diffs
 *
 * Coverage:
 * - GIT-004: Interactive checkpoint viewer with diff and commit capabilities
 * - TUI-002: Checkpoint Viewer Three-Pane Layout
 */

import React, { useState, useMemo, useEffect, useRef } from 'react';
import { Box, Text, useInput } from 'ink';
import { VirtualList } from './VirtualList';
import type { FileItem } from './FileDiffViewer';
import { Worker } from 'worker_threads';
import { join } from 'path';
import { parseDiff, DiffLine } from '../../git/diff-parser';
import { useFspecStore } from '../store/fspecStore';
import * as git from 'isomorphic-git';
import fs from 'fs';
import type { Checkpoint as GitCheckpoint } from '../../utils/git-checkpoint';
import { getCheckpointChangedFiles } from '../../utils/git-checkpoint';

export interface Checkpoint {
  name: string;
  workUnitId: string;
  timestamp: string;
  stashRef: string;
  isAutomatic: boolean;
  files: string[];
  fileCount: number;
}

interface CheckpointViewerProps {
  onExit: () => void;
}

export const CheckpointViewer: React.FC<CheckpointViewerProps> = ({
  onExit,
}) => {
  const [focusedPane, setFocusedPane] = useState<'checkpoints' | 'files' | 'diff'>('checkpoints');
  const [selectedCheckpointIndex, setSelectedCheckpointIndex] = useState(0);
  const [selectedFileIndex, setSelectedFileIndex] = useState(0);
  const [diffContent, setDiffContent] = useState<string>('');
  const [isLoadingDiff, setIsLoadingDiff] = useState(false);
  const [checkpoints, setCheckpoints] = useState<Checkpoint[]>([]);
  const [isLoadingCheckpoints, setIsLoadingCheckpoints] = useState(true);

  const cwd = useFspecStore(state => state.cwd);

  // Checkpoint list and file list minimum width in characters (matches FileDiffViewer)
  const leftColumnMinWidth = 30;
  const fileListMinWidth = 30;

  // Worker thread reference
  const workerRef = useRef<Worker | null>(null);
  const pendingRequestId = useRef<string | null>(null);

  // Load all checkpoints from all work units
  useEffect(() => {
    const loadAllCheckpoints = async () => {
      setIsLoadingCheckpoints(true);
      try {
        const indexDir = join(cwd, '.git', 'fspec-checkpoints-index');

        // Check if index directory exists
        if (!fs.existsSync(indexDir)) {
          setCheckpoints([]);
          setIsLoadingCheckpoints(false);
          return;
        }

        // Read all work unit index files
        const indexFiles = fs.readdirSync(indexDir).filter(f => f.endsWith('.json'));
        const allCheckpoints: Checkpoint[] = [];

        for (const indexFile of indexFiles) {
          const workUnitId = indexFile.replace('.json', '');
          const indexPath = join(indexDir, indexFile);

          const content = fs.readFileSync(indexPath, 'utf-8');
          const index = JSON.parse(content) as { checkpoints: { name: string; message: string }[] };

          for (const cp of index.checkpoints) {
            const ref = `refs/fspec-checkpoints/${workUnitId}/${cp.name}`;

            try {
              // Resolve checkpoint ref to get OID
              const checkpointOid = await git.resolveRef({ fs, dir: cwd, ref });

              // Load changed files from checkpoint (not all files)
              const files = await getCheckpointChangedFiles(cwd, checkpointOid);

              // Parse checkpoint message to extract timestamp
              const match = cp.message.match(/^fspec-checkpoint:[^:]+:[^:]+:([^:]+)$/);
              const timestamp = match ? new Date(parseInt(match[1])).toISOString() : new Date().toISOString();

              allCheckpoints.push({
                name: cp.name,
                workUnitId,
                timestamp,
                stashRef: ref,
                isAutomatic: cp.name.includes('-auto-'),
                files,
                fileCount: files.length,
              });
            } catch (error) {
              // Skip checkpoints that can't be loaded
            }
          }
        }

        setCheckpoints(allCheckpoints);
      } catch (error) {
        setCheckpoints([]);
      } finally {
        setIsLoadingCheckpoints(false);
      }
    };

    void loadAllCheckpoints();
  }, [cwd]);

  // Sort checkpoints by timestamp (most recent first) and limit to 200
  const sortedCheckpoints = useMemo(() => {
    const sorted = [...checkpoints].sort((a, b) => {
      return new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime();
    });
    // Limit to 200 most recent checkpoints for performance
    return sorted.slice(0, 200);
  }, [checkpoints]);

  // Get current checkpoint and files
  const currentCheckpoint = sortedCheckpoints[selectedCheckpointIndex];
  const files: FileItem[] = useMemo(() => {
    if (!currentCheckpoint) {
      return [];
    }
    return currentCheckpoint.files.map(f => ({ path: f, status: 'checkpoint' as const }));
  }, [currentCheckpoint, selectedCheckpointIndex]);

  // Initialize worker thread on mount
  useEffect(() => {
    const workerPath = join(process.cwd(), 'dist', 'git', 'diff-worker.js');

    try {
      workerRef.current = new Worker(workerPath);
    } catch (error) {
      // Worker initialization failed
    }

    return () => {
      if (workerRef.current) {
        workerRef.current.terminate();
        workerRef.current = null;
      }
    };
  }, []);

  // Load git diff when selected file changes using worker thread
  useEffect(() => {
    const selectedFile = files[selectedFileIndex];

    if (!selectedFile) {
      setDiffContent('');
      setIsLoadingDiff(false);
      return;
    }

    if (!workerRef.current) {
      setDiffContent('Worker thread not available');
      setIsLoadingDiff(false);
      return;
    }

    setIsLoadingDiff(true);

    const requestId = `${Date.now()}`;
    pendingRequestId.current = requestId;

    const worker = workerRef.current;

    // Set up message handler for this request
    const messageHandler = (response: { id: string; diff?: string; error?: string }) => {
      // Ignore responses from cancelled requests
      if (response.id !== pendingRequestId.current) {
        return;
      }

      if (response.error) {
        setDiffContent('Error loading diff');
      } else {
        const diffLength = response.diff?.length || 0;

        // Truncate large diffs to prevent UX hangs
        const MAX_DIFF_SIZE = 100000; // 100KB max
        let finalDiff = response.diff || 'No changes to display';
        if (diffLength > MAX_DIFF_SIZE) {
          const truncatedDiff = response.diff!.substring(0, MAX_DIFF_SIZE);
          const linesShown = truncatedDiff.split('\n').length;
          const totalLines = response.diff!.split('\n').length;
          finalDiff = truncatedDiff + `\n\n... (diff truncated: showing ${linesShown}/${totalLines} lines, ${MAX_DIFF_SIZE}/${diffLength} chars)`;
        }

        setDiffContent(finalDiff);
      }

      setIsLoadingDiff(false);
      worker.off('message', messageHandler);
    };

    worker.on('message', messageHandler);

    // Send request to worker with checkpointRef to compare checkpoint vs HEAD
    worker.postMessage({
      id: requestId,
      cwd,
      filepath: selectedFile.path,
      checkpointRef: currentCheckpoint.stashRef, // Compare checkpoint file vs HEAD
    });

    // Cleanup function to cancel pending requests
    return () => {
      pendingRequestId.current = null;
      worker.off('message', messageHandler);
    };
  }, [selectedFileIndex, files, cwd, currentCheckpoint]);

  // Parse diff content into structured DiffLine objects
  const diffLines: DiffLine[] = useMemo(() => {
    if (isLoadingDiff) {
      return [{ content: 'Loading diff...', type: 'context', changeGroup: null }];
    }
    if (!diffContent) {
      return [];
    }
    return parseDiff(diffContent);
  }, [diffContent, isLoadingDiff]);

  // Handle keyboard input
  useInput((input, key) => {
    if (key.escape) {
      onExit();
      return;
    }

    // Tab key to cycle through three panes
    if (key.tab) {
      setFocusedPane(prev => {
        if (prev === 'checkpoints') return 'files';
        if (prev === 'files') return 'diff';
        return 'checkpoints';
      });
      return;
    }

    // Arrow key navigation handled by VirtualList when focused
  });

  // Loading state
  if (isLoadingCheckpoints) {
    return (
      <Box flexDirection="column" flexGrow={1} borderStyle="single" borderColor="cyan">
        <Box flexDirection="row" flexGrow={1}>
          <Box flexDirection="column" flexGrow={1} flexBasis={0} borderStyle="single" borderTop={false} borderBottom={false} borderLeft={false}>
            <Box flexDirection="column" flexGrow={1} flexBasis={0} borderStyle="single" borderTop={false} borderLeft={false} borderRight={false}>
              <Text wrap="truncate">Loading checkpoints...</Text>
            </Box>
            <Box flexDirection="column" flexGrow={1} flexBasis={0}>
              <Text wrap="truncate">-</Text>
            </Box>
          </Box>
          <Box flexDirection="column" flexGrow={2} flexBasis={0}>
            <Text wrap="truncate">-</Text>
          </Box>
        </Box>
      </Box>
    );
  }

  // Empty state
  if (sortedCheckpoints.length === 0) {
    return (
      <Box flexDirection="column" flexGrow={1} borderStyle="single" borderColor="cyan">
        <Box flexDirection="row" flexGrow={1}>
          {/* Left column: Checkpoint list (top) + File list (bottom) - 33% width */}
          <Box flexDirection="column" flexGrow={1} flexBasis={0} borderStyle="single" borderTop={false} borderBottom={false} borderLeft={false}>
            {/* Checkpoint list pane (top-left) */}
            <Box flexDirection="column" flexGrow={1} flexBasis={0} borderStyle="single" borderTop={false} borderLeft={false} borderRight={false}>
              <Text wrap="truncate">No checkpoints available</Text>
            </Box>
            {/* File list pane (bottom-left) */}
            <Box flexDirection="column" flexGrow={1} flexBasis={0}>
              <Text wrap="truncate">No files</Text>
            </Box>
          </Box>

          {/* Right side: Diff pane - 67% width */}
          <Box flexDirection="column" flexGrow={2} flexBasis={0}>
            <Text wrap="truncate">Select a checkpoint to view files</Text>
          </Box>
        </Box>
      </Box>
    );
  }

  // Render checkpoint item with compact name
  const renderCheckpointItem = (checkpoint: Checkpoint, index: number, isSelected: boolean): React.ReactNode => {
    const indicator = isSelected ? '>' : ' ';

    // Extract compact name from checkpoint name (e.g., "TUI-001-auto-testing" -> "TUI-001 - TESTING")
    const parts = checkpoint.name.split('-auto-');
    const workUnit = parts[0]; // e.g., "TUI-001"
    const phase = parts[1] ? parts[1].toUpperCase() : 'UNKNOWN'; // e.g., "TESTING"
    const compactName = `${workUnit} - ${phase}`;
    const displayName = `${compactName} (${checkpoint.fileCount} ${checkpoint.fileCount === 1 ? 'file' : 'files'})`;

    return (
      <Box flexGrow={1}>
        <Text color={isSelected ? 'cyan' : 'white'} wrap="wrap">
          {indicator} {displayName}
        </Text>
      </Box>
    );
  };

  // Render diff line with syntax highlighting
  const renderDiffLine = (line: DiffLine, index: number, isSelected: boolean): React.ReactNode => {
    let textColor: 'white' | 'cyan' = 'white';
    let backgroundColor: 'red' | 'green' | undefined;

    // Determine colors based on line type
    if (line.type === 'hunk') {
      textColor = 'cyan';
    } else if (line.type === 'removed') {
      textColor = 'white';
      backgroundColor = 'red';
    } else if (line.type === 'added') {
      textColor = 'white';
      backgroundColor = 'green';
    }

    // Apply selection styling if focused
    const selectionColor = isSelected && focusedPane === 'diff' ? 'cyan' : textColor;
    const selectionInverse = isSelected && focusedPane === 'diff';

    return (
      <Box flexGrow={1}>
        <Text
          color={selectionInverse ? selectionColor : textColor}
          backgroundColor={backgroundColor}
          inverse={selectionInverse}
          wrap="truncate"
        >
          {line.content}
        </Text>
      </Box>
    );
  };

  return (
    <Box flexDirection="column" flexGrow={1}>
      {/* Header */}
      <Box>
        <Text bold>
          Checkpoints: {sortedCheckpoints.length} available
        </Text>
      </Box>

      {/* Three-pane layout: Left column (checkpoint list + file list) + Diff pane (right) */}
      <Box flexDirection="column" flexGrow={1} borderStyle="single" borderColor="cyan">
        <Box flexDirection="row" flexGrow={1}>
          {/* Left column: Checkpoint list (top) + File list (bottom) stacked vertically - 33% width via flexGrow ratio */}
          <Box
            flexDirection="column"
            flexGrow={1}
            flexBasis={0}
            borderStyle="single"
            borderTop={false}
            borderBottom={false}
            borderLeft={false}
          >
            {/* Checkpoint list pane (top-left) - flexGrow 1 with flexBasis 0 for 50/50 vertical split */}
            <Box
              flexDirection="column"
              flexGrow={1}
              flexBasis={0}
              borderStyle="single"
              borderTop={false}
              borderLeft={false}
              borderRight={false}
            >
              <VirtualList
                items={sortedCheckpoints}
                renderItem={renderCheckpointItem}
                showScrollbar={focusedPane === 'checkpoints'}
                isFocused={focusedPane === 'checkpoints'}
                onFocus={(checkpoint, index) => {
                  setSelectedCheckpointIndex(index);
                  setSelectedFileIndex(0); // Reset file selection when checkpoint changes
                }}
              />
            </Box>

            {/* File list pane (bottom-left) - flexGrow 1 with flexBasis 0 for 50/50 vertical split */}
            <Box
              flexDirection="column"
              flexGrow={1}
              flexBasis={0}
            >
              <VirtualList
                items={files}
                renderItem={(file, index, isSelected) => {
                  const indicator = isSelected ? '>' : ' ';
                  return (
                    <Box flexGrow={1}>
                      <Text color={isSelected ? 'cyan' : 'white'} wrap="wrap">
                        {indicator} {file.path}
                      </Text>
                    </Box>
                  );
                }}
                showScrollbar={focusedPane === 'files'}
                isFocused={focusedPane === 'files'}
                onFocus={(file, index) => {
                  setSelectedFileIndex(index);
                }}
              />
            </Box>
          </Box>

          {/* Right side: Diff pane only - 67% width via flexGrow ratio */}
          <Box
            flexDirection="column"
            flexGrow={2}
            flexBasis={0}
          >
            <Box flexGrow={1}>
              <VirtualList
                items={diffLines}
                renderItem={renderDiffLine}
                showScrollbar={focusedPane === 'diff'}
                isFocused={focusedPane === 'diff'}
              />
            </Box>
          </Box>
        </Box>
      </Box>

      {/* Footer */}
      <Box>
        <Text dimColor>
          ESC: Back | Tab: Switch Panes | ↑↓: Navigate | PgUp/PgDn: Scroll
        </Text>
      </Box>
    </Box>
  );
};
