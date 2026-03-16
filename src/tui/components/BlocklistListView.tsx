/**
 * BlocklistListView - Full-screen overlay for viewing and toggling blocklist rules
 *
 * BLOCK-004: Blocklist TUI - List/Form Views
 * INPUT-001: Uses centralized input handling with CRITICAL priority
 *
 * Features:
 * - Full-screen overlay (follows SupervisorTemplateList pattern)
 * - Shows all rules from system and project configs
 * - Session-level rule toggling (disable/enable rules for current session)
 * - Keyboard navigation (j/k or arrows, Enter to toggle, Escape to close)
 *
 * @see spec/features/blocklist-tui-list-form-views.feature
 */

import React, { useState, useCallback, useEffect, useMemo } from 'react';
import { Box, Text } from 'ink';
import { useInputCompat, InputPriority } from '../input/index';

/** Blocklist rule structure matching NAPI bindings */
export interface BlocklistRule {
  /** Unique identifier for the rule */
  id: string;
  /** Regex pattern to match against commands */
  pattern: string;
  /** Action: "block", "allow", or "prompt" */
  action: string;
  /** Reason for blocking (shown to AI) */
  reason: string;
  /** Guidance on what to do instead (educational) */
  guidance?: string;
  /** Source of the rule: "system" or "project" */
  source?: 'system' | 'project';
}

interface BlocklistListViewProps {
  /** All blocklist rules (merged from system and project) */
  rules: BlocklistRule[];
  /** Set of rule IDs that are disabled for this session */
  disabledRules: Set<string>;
  /** Terminal dimensions */
  terminalWidth: number;
  terminalHeight: number;
  /** Callback when user toggles a rule's session state */
  onToggleRule: (ruleId: string) => void;
  /** Callback when overlay is closed */
  onClose: () => void;
}

export function BlocklistListView({
  rules,
  disabledRules,
  terminalWidth,
  terminalHeight,
  onToggleRule,
  onClose,
}: BlocklistListViewProps): React.ReactElement {
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [scrollOffset, setScrollOffset] = useState(0);

  // Calculate visible height (leave room for header and footer)
  const visibleHeight = Math.max(1, terminalHeight - 10);

  // Ensure selected index stays in bounds
  useEffect(() => {
    if (rules.length > 0 && selectedIndex >= rules.length) {
      setSelectedIndex(Math.max(0, rules.length - 1));
    }
  }, [rules.length, selectedIndex]);

  // Auto-scroll to keep selection visible
  useEffect(() => {
    if (selectedIndex < scrollOffset) {
      setScrollOffset(selectedIndex);
    } else if (selectedIndex >= scrollOffset + visibleHeight) {
      setScrollOffset(selectedIndex - visibleHeight + 1);
    }
  }, [selectedIndex, scrollOffset, visibleHeight]);

  const handleToggle = useCallback(() => {
    const rule = rules[selectedIndex];
    if (rule) {
      onToggleRule(rule.id);
    }
  }, [rules, selectedIndex, onToggleRule]);

  // Handle keyboard input with CRITICAL priority (overlay)
  useInputCompat({
    id: 'blocklist-list-view',
    priority: InputPriority.CRITICAL,
    description: 'Blocklist list overlay',
    handler: (input, key) => {
      // Escape closes overlay
      if (key.escape) {
        onClose();
        return true;
      }

      // Enter or Space toggles rule
      if (key.return || input === ' ') {
        handleToggle();
        return true;
      }

      // Navigation: j/down and k/up
      if (key.downArrow || input.toLowerCase() === 'j') {
        setSelectedIndex(prev => Math.min(rules.length - 1, prev + 1));
        return true;
      }
      if (key.upArrow || input.toLowerCase() === 'k') {
        setSelectedIndex(prev => Math.max(0, prev - 1));
        return true;
      }

      return true; // Consume all input when overlay is active
    },
  });

  // Get selected rule for details panel
  const selectedRule = rules[selectedIndex];

  // Render visible items
  const visibleRules = rules.slice(scrollOffset, scrollOffset + visibleHeight);

  // Get action color
  const getActionColor = (action: string): string => {
    switch (action) {
      case 'block':
        return 'red';
      case 'allow':
        return 'green';
      case 'prompt':
        return 'yellow';
      default:
        return 'white';
    }
  };

  return (
    <Box position="absolute" flexDirection="column" width={terminalWidth} height={terminalHeight}>
      <Box flexDirection="column" flexGrow={1} backgroundColor="black">
        <Box flexDirection="column" padding={2} flexGrow={1}>
          {/* Header */}
          <Box marginBottom={1} borderStyle="single" borderBottom borderLeft={false} borderRight={false} borderTop={false}>
            <Text bold color="cyan">Blocklist Rules</Text>
            <Text dimColor> ({rules.length} rules)</Text>
          </Box>

          {/* Empty state */}
          {rules.length === 0 && (
            <Box flexDirection="column" flexGrow={1} justifyContent="center">
              <Text color="yellow">No blocklist rules configured.</Text>
              <Text> </Text>
              <Text dimColor>Blocklist rules prevent dangerous commands and guide</Text>
              <Text dimColor>AI agents to use proper tools and patterns.</Text>
              <Text> </Text>
              <Text dimColor>System config: ~/.fspec/blocklist.json</Text>
              <Text dimColor>Project config: .fspec/blocklist.json</Text>
            </Box>
          )}

          {/* Rule list */}
          {rules.length > 0 && (
            <Box flexDirection="row" flexGrow={1}>
              {/* Left panel: Rule list */}
              <Box flexDirection="column" width="50%">
                {visibleRules.map((rule, visibleIdx) => {
                  const actualIdx = scrollOffset + visibleIdx;
                  const isSelected = actualIdx === selectedIndex;
                  const isDisabled = disabledRules.has(rule.id);

                  return (
                    <Box key={rule.id} flexDirection="column">
                      <Box>
                        <Text
                          backgroundColor={isSelected ? 'cyan' : undefined}
                          color={isSelected ? 'black' : isDisabled ? 'gray' : 'white'}
                          wrap="truncate"
                        >
                          {isSelected ? '> ' : '  '}
                          {isDisabled ? '○' : '●'} {rule.id}
                        </Text>
                      </Box>
                      <Box>
                        <Text
                          backgroundColor={isSelected ? 'cyan' : undefined}
                          color={isSelected ? 'black' : getActionColor(rule.action)}
                          dimColor={!isSelected && isDisabled}
                        >
                          {'    '}[{rule.action}]
                          {isDisabled && ' (disabled)'}
                        </Text>
                      </Box>
                    </Box>
                  );
                })}
              </Box>

              {/* Right panel: Rule details */}
              <Box flexDirection="column" width="50%" paddingLeft={2} borderStyle="single" borderLeft borderRight={false} borderTop={false} borderBottom={false}>
                {selectedRule && (
                  <>
                    <Box marginBottom={1}>
                      <Text bold color="cyan">Rule Details</Text>
                    </Box>
                    <Box>
                      <Text color="white">ID: </Text>
                      <Text>{selectedRule.id}</Text>
                    </Box>
                    <Box>
                      <Text color="white">Action: </Text>
                      <Text color={getActionColor(selectedRule.action)}>{selectedRule.action}</Text>
                    </Box>
                    <Box>
                      <Text color="white">Source: </Text>
                      <Text dimColor>{selectedRule.source || 'unknown'}</Text>
                    </Box>
                    <Box marginTop={1}>
                      <Text color="white">Pattern:</Text>
                    </Box>
                    <Box>
                      <Text dimColor wrap="wrap">{selectedRule.pattern}</Text>
                    </Box>
                    {selectedRule.reason && (
                      <>
                        <Box marginTop={1}>
                          <Text color="white">Reason:</Text>
                        </Box>
                        <Box>
                          <Text wrap="wrap">{selectedRule.reason}</Text>
                        </Box>
                      </>
                    )}
                    {selectedRule.guidance && (
                      <>
                        <Box marginTop={1}>
                          <Text color="white">Guidance:</Text>
                        </Box>
                        <Box>
                          <Text color="green" wrap="wrap">{selectedRule.guidance}</Text>
                        </Box>
                      </>
                    )}
                    <Box marginTop={1}>
                      <Text color="white">Session Status: </Text>
                      <Text color={disabledRules.has(selectedRule.id) ? 'yellow' : 'green'}>
                        {disabledRules.has(selectedRule.id) ? 'disabled (session)' : 'enabled'}
                      </Text>
                    </Box>
                  </>
                )}
              </Box>
            </Box>
          )}

          {/* Scroll indicator */}
          {rules.length > visibleHeight && (
            <Box marginTop={1}>
              <Text dimColor>
                Showing {scrollOffset + 1}-{Math.min(scrollOffset + visibleHeight, rules.length)} of {rules.length}
              </Text>
            </Box>
          )}

          {/* Footer */}
          <Box marginTop={1} borderStyle="single" borderTop borderBottom={false} borderLeft={false} borderRight={false}>
            <Text dimColor>
              ↑↓/jk: Navigate | Enter/Space: Toggle Rule | Esc: Close
            </Text>
          </Box>
        </Box>
      </Box>
    </Box>
  );
}
