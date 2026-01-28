/**
 * AgentSelector - Interactive CLI component for selecting AI coding agent
 *
 * Used during `fspec init` to let users choose which agent they want to use.
 * Renders standalone (outside main TUI), so uses useInputCompat's fallback mode.
 *
 * INPUT-001: Uses centralized input handling (falls back to useInput when no InputManager)
 */

import React, { useState, useEffect } from 'react';
import { Box, Text, useApp } from 'ink';
import type { AgentConfig } from '../utils/agentRegistry';
import { useInputCompat, InputPriority } from '../tui/input/index';

interface AgentSelectorProps {
  agents: AgentConfig[];
  preSelected: string[];
  onSubmit: (selected: string) => void;
}

export const AgentSelector: React.FC<AgentSelectorProps> = ({
  agents,
  preSelected,
  onSubmit,
}) => {
  // Start cursor on first pre-selected agent, or first agent if none detected
  const initialCursor = preSelected.length > 0
    ? agents.findIndex(a => a.id === preSelected[0])
    : 0;
  const [cursor, setCursor] = useState(initialCursor >= 0 ? initialCursor : 0);
  const [selectedAgent, setSelectedAgent] = useState<string | null>(null);
  const { exit } = useApp();

  useInputCompat({
    id: 'agent-selector-nav',
    priority: InputPriority.MEDIUM,
    isActive: !selectedAgent,
    handler: (_input, key) => {
      if (selectedAgent) {
        return false;
      }

      if (key.upArrow) {
        setCursor(Math.max(0, cursor - 1));
        return true;
      } else if (key.downArrow) {
        setCursor(Math.min(agents.length - 1, cursor + 1));
        return true;
      } else if (key.return) {
        // Select the agent
        setSelectedAgent(agents[cursor].id);
        return true;
      }
      return false;
    },
  });

  useEffect(() => {
    if (selectedAgent) {
      onSubmit(selectedAgent);
      // Exit after a brief delay to show the success message
      setTimeout(() => {
        exit();
      }, 100);
    }
  }, [selectedAgent, onSubmit, exit]);

  if (selectedAgent) {
    // Exit silently - success message will be displayed by init.ts action handler
    // This prevents duplicate success messages
    return null;
  }

  return (
    <Box flexDirection="column">
      <Text bold>Select your AI coding agent:</Text>
      <Text dimColor>(Use ↑↓ to navigate, ENTER to select)</Text>
      <Text> </Text>
      {agents.map((agent, index) => {
        const isCursor = index === cursor;
        const marker = isCursor ? '▶' : ' ';

        return (
          <Text key={agent.id} color={isCursor ? 'cyan' : undefined}>
            {marker} {agent.name}
            {preSelected.includes(agent.id) && (
              <Text dimColor> (detected)</Text>
            )}
          </Text>
        );
      })}
    </Box>
  );
};
