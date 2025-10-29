/**
 * CheckpointPanel Component - displays checkpoint counts in header
 *
 * Coverage:
 * - ITF-006: Replace Git Stashes with Checkpoint Component
 */

import React, { useEffect, useState } from 'react';
import { Box, Text } from 'ink';
import chokidar from 'chokidar';
import fs from 'fs';
import path from 'path';

interface CheckpointPanelProps {
  cwd?: string;
}

interface CheckpointCounts {
  manual: number;
  auto: number;
}

export const CheckpointPanel: React.FC<CheckpointPanelProps> = ({ cwd }) => {
  // Count checkpoints by reading all *.json files in the index directory
  const countCheckpoints = (): CheckpointCounts => {
    const workingDir = cwd || process.cwd();
    const indexDir = path.join(workingDir, '.git', 'fspec-checkpoints-index');

    let manual = 0;
    let auto = 0;

    try {
      if (!fs.existsSync(indexDir)) {
        return { manual: 0, auto: 0 };
      }

      const files = fs.readdirSync(indexDir).filter(f => f.endsWith('.json'));

      for (const file of files) {
        const filePath = path.join(indexDir, file);
        const content = fs.readFileSync(filePath, 'utf-8');
        const data = JSON.parse(content) as { checkpoints: Array<{ name: string; message: string }> };

        for (const checkpoint of data.checkpoints || []) {
          // Auto checkpoints are named {workUnitId}-auto-{state}
          if (checkpoint.name.includes('-auto-')) {
            auto++;
          } else {
            manual++;
          }
        }
      }
    } catch (error) {
      // Silent failure - return zero counts if there's an error
    }

    return { manual, auto };
  };

  // Calculate counts on mount and when cwd changes (using state for chokidar updates)
  const [counts, setCounts] = useState<CheckpointCounts>(() => countCheckpoints());

  // Watch checkpoint index directory with chokidar for real-time updates
  useEffect(() => {
    const workingDir = cwd || process.cwd();
    const indexDir = path.join(workingDir, '.git', 'fspec-checkpoints-index');

    // Create directory if it doesn't exist
    if (!fs.existsSync(indexDir)) {
      return;
    }

    // Watch the index directory for changes (cross-platform compatible)
    const watcher = chokidar.watch(indexDir, {
      ignoreInitial: true,
      persistent: false,
    });

    // Update counts immediately when changes are detected (no debouncing)
    watcher.on('add', () => setCounts(countCheckpoints()));
    watcher.on('change', () => setCounts(countCheckpoints()));
    watcher.on('unlink', () => setCounts(countCheckpoints()));

    // Add error handler to prevent silent failures
    watcher.on('error', (error) => {
      console.warn('Checkpoint index watcher error:', error.message);
    });

    return () => {
      void watcher.close();
    };
  }, [cwd]);

  // Format display: "Checkpoints: X Manual, Y Auto" or "Checkpoints: None"
  const displayText = counts.manual === 0 && counts.auto === 0
    ? 'Checkpoints: None'
    : `Checkpoints: ${counts.manual} Manual, ${counts.auto} Auto`;

  return (
    <Box flexDirection="column">
      <Text>{displayText}</Text>
    </Box>
  );
};
