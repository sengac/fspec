/**
 * WatcherTemplateList - Template-centric watcher management overlay
 *
 * WATCH-023: Watcher Templates and Improved Creation UX
 * Replaces the old instance-centric watcher overlay (WATCH-008).
 * INPUT-001: Uses centralized input handling with CRITICAL priority
 *
 * Features:
 * - Templates as primary list items (not instances)
 * - Collapse/expand for instances (▼/▶)
 * - Type-to-filter search
 * - Arrow key navigation
 * - Actions: Enter (spawn/open), E (edit), D (delete), N (new)
 *
 * @see spec/features/watcher-templates.feature
 */

import React, { useState, useCallback, useEffect, useMemo } from 'react';
import { Box, Text } from 'ink';
import type { WatcherTemplate, WatcherInstance, WatcherListItem } from '../types/watcherTemplate.js';
import { buildFlatWatcherList, filterTemplates, formatTemplateDisplay } from '../utils/watcherTemplateStorage.js';
import { useInputCompat, InputPriority } from '../input/index.js';

interface WatcherTemplateListProps {
  templates: WatcherTemplate[];
  instances: WatcherInstance[];
  terminalWidth: number;
  terminalHeight: number;
  onSpawn: (template: WatcherTemplate) => void;
  onOpen: (instance: WatcherInstance) => void;
  onEdit: (template: WatcherTemplate) => void;
  onDelete: (template: WatcherTemplate, instances: WatcherInstance[]) => void;
  onKillInstance: (instance: WatcherInstance) => void;
  onCreateNew: () => void;
  onClose: () => void;
}

export function WatcherTemplateList({
  templates,
  instances,
  terminalWidth,
  terminalHeight,
  onSpawn,
  onOpen,
  onEdit,
  onDelete,
  onKillInstance,
  onCreateNew,
  onClose,
}: WatcherTemplateListProps): React.ReactElement {
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [expandedTemplates, setExpandedTemplates] = useState<Set<string>>(new Set());
  const [filterQuery, setFilterQuery] = useState('');
  const [scrollOffset, setScrollOffset] = useState(0);

  // Filter templates by search query
  const filteredTemplates = useMemo(() => 
    filterTemplates(templates, filterQuery), 
    [templates, filterQuery]
  );

  // Build flat list for navigation
  const flatList = useMemo(() => 
    buildFlatWatcherList(filteredTemplates, instances, expandedTemplates),
    [filteredTemplates, instances, expandedTemplates]
  );

  // Calculate visible height (leave room for header and footer)
  const visibleHeight = Math.max(1, terminalHeight - 8);

  // Ensure selected index stays in bounds
  useEffect(() => {
    if (selectedIndex >= flatList.length) {
      setSelectedIndex(Math.max(0, flatList.length - 1));
    }
  }, [flatList.length, selectedIndex]);

  // Auto-scroll to keep selection visible
  useEffect(() => {
    if (selectedIndex < scrollOffset) {
      setScrollOffset(selectedIndex);
    } else if (selectedIndex >= scrollOffset + visibleHeight) {
      setScrollOffset(selectedIndex - visibleHeight + 1);
    }
  }, [selectedIndex, scrollOffset, visibleHeight]);

  const toggleExpand = useCallback((templateId: string) => {
    setExpandedTemplates(prev => {
      const next = new Set(prev);
      if (next.has(templateId)) {
        next.delete(templateId);
      } else {
        next.add(templateId);
      }
      return next;
    });
  }, []);

  const handleAction = useCallback(() => {
    const item = flatList[selectedIndex];
    if (!item) return;

    switch (item.type) {
      case 'template':
        onSpawn(item.template);
        break;
      case 'instance':
        onOpen(item.instance);
        break;
      case 'create-new':
        onCreateNew();
        break;
    }
  }, [flatList, selectedIndex, onSpawn, onOpen, onCreateNew]);

  // Handle keyboard input with CRITICAL priority (overlay)
  useInputCompat({
    id: 'watcher-template-list',
    priority: InputPriority.CRITICAL,
    description: 'Watcher template list overlay',
    handler: (input, key) => {
      // Escape closes overlay
      if (key.escape) {
        if (filterQuery) {
          setFilterQuery('');
        } else {
          onClose();
        }
        return true;
      }

      // Enter performs action
      if (key.return) {
        handleAction();
        return true;
      }

      // Navigation
      if (key.upArrow) {
        setSelectedIndex(prev => Math.max(0, prev - 1));
        return true;
      }
      if (key.downArrow) {
        setSelectedIndex(prev => Math.min(flatList.length - 1, prev + 1));
        return true;
      }

      // Expand/collapse
      const item = flatList[selectedIndex];
      if (key.leftArrow && item?.type === 'template') {
        if (expandedTemplates.has(item.template.id)) {
          toggleExpand(item.template.id);
        }
        return true;
      }
      if (key.rightArrow && item?.type === 'template') {
        const templateInstances = instances.filter(i => i.templateId === item.template.id);
        if (templateInstances.length > 0 && !expandedTemplates.has(item.template.id)) {
          toggleExpand(item.template.id);
        }
        return true;
      }

      // Actions
      if (input.toLowerCase() === 'n') {
        onCreateNew();
        return true;
      }

      if (input.toLowerCase() === 'e' && item?.type === 'template') {
        onEdit(item.template);
        return true;
      }

      if (input.toLowerCase() === 'd') {
        if (item?.type === 'template') {
          const templateInstances = instances.filter(i => i.templateId === item.template.id);
          onDelete(item.template, templateInstances);
        } else if (item?.type === 'instance') {
          onKillInstance(item.instance);
        }
        return true;
      }

      // Type-to-filter (printable characters)
      if (key.backspace || key.delete) {
        setFilterQuery(prev => prev.slice(0, -1));
        return true;
      }

      // Accept printable characters for filtering
      const clean = input.split('').filter(ch => {
        const code = ch.charCodeAt(0);
        return code >= 32 && code <= 126;
      }).join('');
      
      if (clean && !key.ctrl && !key.meta) {
        setFilterQuery(prev => prev + clean);
      }

      return true; // Consume all input when overlay is active
    },
  });

  // Render list items
  const visibleItems = flatList.slice(scrollOffset, scrollOffset + visibleHeight);

  return (
    <Box position="absolute" flexDirection="column" width={terminalWidth} height={terminalHeight}>
      <Box flexDirection="column" flexGrow={1} backgroundColor="black">
        <Box flexDirection="column" padding={2} flexGrow={1}>
          {/* Header */}
          <Box marginBottom={1} borderStyle="single" borderBottom borderLeft={false} borderRight={false} borderTop={false}>
            <Text bold color="magenta">Watcher Templates</Text>
            {filterQuery && (
              <Text dimColor> (filter: {filterQuery})</Text>
            )}
          </Box>

          {/* Empty state */}
          {templates.length === 0 && !filterQuery && (
            <Box flexDirection="column" flexGrow={1} justifyContent="center">
              <Text color="yellow">No watcher templates yet.</Text>
              <Text> </Text>
              <Text dimColor>Watchers are AI agents that observe your session and</Text>
              <Text dimColor>can interject with feedback (security issues, test</Text>
              <Text dimColor>coverage, architecture suggestions, etc.)</Text>
              <Text> </Text>
              <Text color="cyan">Press N to create your first template.</Text>
            </Box>
          )}

          {/* No results */}
          {filteredTemplates.length === 0 && filterQuery && (
            <Box flexDirection="column" flexGrow={1}>
              <Text dimColor>No templates match "{filterQuery}"</Text>
            </Box>
          )}

          {/* Template list */}
          {(templates.length > 0 || filterQuery) && filteredTemplates.length > 0 && (
            <Box flexDirection="column" flexGrow={1}>
              {visibleItems.map((item, visibleIdx) => {
                const actualIdx = scrollOffset + visibleIdx;
                const isSelected = actualIdx === selectedIndex;

                if (item.type === 'create-new') {
                  return (
                    <Box key="create-new">
                      <Text
                        backgroundColor={isSelected ? 'magenta' : undefined}
                        color={isSelected ? 'black' : 'cyan'}
                      >
                        {isSelected ? '> ' : '  '}+ Create new template...
                      </Text>
                    </Box>
                  );
                }

                if (item.type === 'template') {
                  const templateInstances = instances.filter(i => i.templateId === item.template.id);
                  const hasInstances = templateInstances.length > 0;
                  const expandIcon = hasInstances 
                    ? (item.isExpanded ? '▼' : '▶') 
                    : ' ';
                  
                  return (
                    <Box key={item.template.id} flexDirection="column">
                      <Box>
                        <Text
                          backgroundColor={isSelected ? 'magenta' : undefined}
                          color={isSelected ? 'black' : 'white'}
                          wrap="truncate"
                        >
                          {isSelected ? '> ' : '  '}{expandIcon} {formatTemplateDisplay(item.template, item.instanceCount)}
                        </Text>
                      </Box>
                      <Box>
                        <Text
                          backgroundColor={isSelected ? 'magenta' : undefined}
                          color={isSelected ? 'black' : 'gray'}
                          dimColor={!isSelected}
                        >
                          {'     '}{item.template.modelId.split('/').pop()}
                        </Text>
                      </Box>
                    </Box>
                  );
                }

                if (item.type === 'instance') {
                  const templateInstances = instances.filter(
                    i => i.templateId === item.template.id
                  );
                  const instanceIdx = templateInstances.indexOf(item.instance) + 1;
                  const isLast = instanceIdx === templateInstances.length;
                  const connector = isLast ? '└─' : '├─';
                  const statusIcon = item.instance.status === 'running' ? '🔄' : '⏸️';
                  
                  return (
                    <Box key={item.instance.sessionId}>
                      <Text
                        backgroundColor={isSelected ? 'magenta' : undefined}
                        color={isSelected ? 'black' : 'gray'}
                      >
                        {isSelected ? '> ' : '  '}   {connector} {statusIcon} #{instanceIdx} {item.instance.status}
                      </Text>
                    </Box>
                  );
                }

                return null;
              })}
            </Box>
          )}

          {/* Footer */}
          <Box marginTop={1} borderStyle="single" borderTop borderBottom={false} borderLeft={false} borderRight={false}>
            <Text dimColor>
              ←→: Collapse/Expand | Enter: Spawn/Open | E: Edit | D: Delete | N: New | Esc: Close
            </Text>
          </Box>
        </Box>
      </Box>
    </Box>
  );
}
