/**
 * FileDiffViewer Component - Shared dual-pane viewer for file list and diffs
 *
 * Coverage:
 * - TUI-002: Checkpoint Viewer Three-Pane Layout
 *
 * This component extracts the common file list + diff pane logic from
 * ChangedFilesViewer and CheckpointViewer to eliminate code duplication (DRY).
 */

import React, { useState, useEffect, useMemo, useRef } from 'react';
import { Box, Text } from 'ink';
import { VirtualList } from './VirtualList';
import { useFspecStore } from '../store/fspecStore';
import { logger } from '../../utils/logger';
import { Worker } from 'worker_threads';
import { join } from 'path';
import { parseDiff, DiffLine } from '../../git/diff-parser';

export interface FileItem {
  path: string;
  status: 'staged' | 'unstaged' | 'checkpoint';
}

export interface FileDiffViewerProps {
  files: FileItem[];
  focusedPane: 'files' | 'diff';
  onFocusChange: (pane: 'files' | 'diff') => void;
  onFileSelect: (file: FileItem, index: number) => void;
  selectedFileIndex?: number;
  renderFileItem?: (file: FileItem, index: number, isSelected: boolean) => React.ReactNode;
  diffLines?: DiffLine[]; // Optional: for testing or pre-parsed diffs
}

export const FileDiffViewer: React.FC<FileDiffViewerProps> = ({
  files,
  focusedPane,
  onFocusChange,
  onFileSelect,
  selectedFileIndex = 0,
  renderFileItem: customRenderFileItem,
  diffLines: externalDiffLines,
}) => {
  const [diffContent, setDiffContent] = useState<string>('');
  const [isLoadingDiff, setIsLoadingDiff] = useState(false);

  const cwd = useFspecStore(state => state.cwd);

  // File list minimum width in characters
  const fileListMinWidth = 30;

  // Worker thread reference
  const workerRef = useRef<Worker | null>(null);
  const pendingRequestId = useRef<string | null>(null);
  const componentId = useRef(`FileDiffViewer-${Date.now()}`);

  // Initialize worker thread on mount
  useEffect(() => {
    const workerPath = join(process.cwd(), 'dist', 'git', 'diff-worker.js');
    logger.info(`[${componentId.current}] Initializing worker thread at: ${workerPath}`);

    try {
      workerRef.current = new Worker(workerPath);
      logger.info(`[${componentId.current}] Worker thread initialized successfully`);
    } catch (error) {
      logger.error(`[${componentId.current}] Failed to initialize worker: ${error}`);
    }

    return () => {
      if (workerRef.current) {
        logger.info(`[${componentId.current}] Terminating worker thread`);
        workerRef.current.terminate();
        workerRef.current = null;
      }
    };
  }, []);

  // Load git diff when selected file changes using worker thread
  useEffect(() => {
    const selectedFile = files[selectedFileIndex];

    if (!selectedFile) {
      logger.info(`[${componentId.current}] No selected file, clearing diff`);
      setDiffContent('');
      setIsLoadingDiff(false);
      return;
    }

    if (!workerRef.current) {
      logger.error(`[${componentId.current}] Worker not initialized, cannot load diff`);
      setDiffContent('Worker thread not available');
      setIsLoadingDiff(false);
      return;
    }

    logger.info(`[${componentId.current}] Loading diff for file: ${selectedFile.path}`);
    setIsLoadingDiff(true);

    const startTime = Date.now();
    const requestId = `${Date.now()}`;
    pendingRequestId.current = requestId;

    const worker = workerRef.current;

    // Set up message handler for this request
    const messageHandler = (response: { id: string; diff?: string; error?: string }) => {
      const duration = Date.now() - startTime;

      // Ignore responses from cancelled requests
      if (response.id !== pendingRequestId.current) {
        logger.info(`[${componentId.current}] Received stale response (id=${response.id}), ignoring`);
        return;
      }

      if (response.error) {
        logger.error(`[${componentId.current}] Worker error after ${duration}ms: ${response.error}`);
        setDiffContent('Error loading diff');
      } else {
        const diffLength = response.diff?.length || 0;
        logger.info(`[${componentId.current}] Diff loaded successfully in ${duration}ms (${diffLength} chars)`);

        // Truncate large diffs to prevent UX hangs
        const MAX_DIFF_SIZE = 100000; // 100KB max
        let finalDiff = response.diff || 'No changes to display';
        if (diffLength > MAX_DIFF_SIZE) {
          const truncatedDiff = response.diff!.substring(0, MAX_DIFF_SIZE);
          const linesShown = truncatedDiff.split('\n').length;
          const totalLines = response.diff!.split('\n').length;
          finalDiff = truncatedDiff + `\n\n... (diff truncated: showing ${linesShown}/${totalLines} lines, ${MAX_DIFF_SIZE}/${diffLength} chars)`;
          logger.info(`[${componentId.current}] Diff truncated from ${diffLength} to ${MAX_DIFF_SIZE} chars`);
        }

        setDiffContent(finalDiff);
      }

      setIsLoadingDiff(false);
      worker.off('message', messageHandler);
    };

    worker.on('message', messageHandler);

    // Send request to worker
    logger.info(`[${componentId.current}] Sending request to worker (id=${requestId})`);
    worker.postMessage({
      id: requestId,
      cwd,
      filepath: selectedFile.path,
    });

    // Cleanup function to cancel pending requests
    return () => {
      logger.info(`[${componentId.current}] CLEANUP called (cancelling request ${requestId})`);
      pendingRequestId.current = null;
      worker.off('message', messageHandler);
    };
  }, [selectedFileIndex, files, cwd]);

  // Parse diff content into structured DiffLine objects
  // Use external diff lines if provided (for testing), otherwise load from git
  const diffLines: DiffLine[] = useMemo(() => {
    if (externalDiffLines) {
      return externalDiffLines;
    }
    if (isLoadingDiff) {
      return [{ content: 'Loading diff...', type: 'context', changeGroup: null }];
    }
    if (!diffContent) {
      return [];
    }
    return parseDiff(diffContent);
  }, [externalDiffLines, diffContent, isLoadingDiff]);

  // Empty state
  if (files.length === 0) {
    return (
      <Box flexDirection="column" flexGrow={1} borderStyle="single" borderTop={false} borderLeft={false} borderRight={false} borderBottom={true}>
        <Box flexDirection="row" flexGrow={1}>
          {/* File list pane (33% width via flexGrow ratio) */}
          <Box flexDirection="column" flexGrow={1} borderStyle="single" borderTop={false} borderBottom={false} borderLeft={false}>
            <Text>No files</Text>
          </Box>

          {/* Diff pane (67% width via flexGrow ratio) */}
          <Box flexDirection="column" flexGrow={2}>
            <Text>No changes to display</Text>
          </Box>
        </Box>
      </Box>
    );
  }

  // Default render file item with status indicator
  const defaultRenderFileItem = (
    file: FileItem,
    index: number,
    isSelected: boolean
  ): React.ReactNode => {
    const indicator = isSelected ? '>' : ' ';
    const statusIcon = file.status === 'staged' ? '+' : file.status === 'unstaged' ? 'M' : '';
    const statusColor = file.status === 'staged' ? 'green' : file.status === 'unstaged' ? 'yellow' : 'white';

    return (
      <Box flexGrow={1}>
        <Text color={isSelected ? 'cyan' : 'white'}>
          {indicator} {statusIcon && <Text color={statusColor}>{statusIcon}</Text>} {file.path}
        </Text>
      </Box>
    );
  };

  // Render diff line with syntax highlighting
  const renderDiffLine = (line: DiffLine, index: number, isSelected: boolean): React.ReactNode => {
    let textColor: 'white' | 'cyan' = 'white';
    let backgroundColor: string | undefined;

    // Determine colors based on line type
    if (line.type === 'hunk') {
      textColor = 'cyan';
    } else if (line.type === 'removed') {
      textColor = 'white';
      backgroundColor = '#8B0000'; // Dark red
    } else if (line.type === 'added') {
      textColor = 'white';
      backgroundColor = '#006400'; // Dark green
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

  const renderItem = customRenderFileItem || defaultRenderFileItem;

  return (
    <Box flexDirection="column" flexGrow={1} borderStyle="single" borderTop={false} borderLeft={false} borderRight={false} borderBottom={true}>
      <Box flexDirection="row" flexGrow={1}>
        {/* File list pane (left, 33% width via flexGrow ratio) */}
        <Box
          flexDirection="column"
          flexGrow={1}
          flexBasis={0}
          borderStyle="single"
          borderTop={false}
          borderBottom={false}
          borderLeft={false}
        >
          {/* File list heading */}
          <Box
            backgroundColor={focusedPane === 'files' ? 'green' : undefined}
            borderStyle="single"
            borderTop={false}
            borderLeft={false}
            borderRight={false}
            borderBottom={true}
          >
            <Text
              bold={focusedPane !== 'files'}
              color={focusedPane === 'files' ? 'black' : 'white'}
            >
              Files
            </Text>
          </Box>
          <VirtualList
            items={files}
            renderItem={renderItem}
            showScrollbar={focusedPane === 'files'}
            isFocused={focusedPane === 'files'}
            onFocus={(file, index) => onFileSelect(file, index)}
          />
        </Box>

        {/* Diff pane (right, 67% width via flexGrow ratio) */}
        <Box
          flexDirection="column"
          flexGrow={2}
          flexBasis={0}
        >
          {/* Diff pane heading */}
          <Box
            backgroundColor={focusedPane === 'diff' ? 'green' : undefined}
            borderStyle="single"
            borderTop={false}
            borderLeft={false}
            borderRight={false}
            borderBottom={true}
          >
            <Text
              bold={focusedPane !== 'diff'}
              color={focusedPane === 'diff' ? 'black' : 'white'}
            >
              Diff
            </Text>
          </Box>
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
  );
};
