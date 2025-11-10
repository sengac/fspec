import React from 'react';
import { render } from 'ink-testing-library';
import { BoardView } from './src/tui/components/BoardView';
import { useFspecStore } from './src/tui/store/fspecStore';

// Set up minimal state
useFspecStore.setState({
  workUnits: [{
    id: 'TEST-001',
    type: 'story' as const,
    title: 'Test',
    status: 'backlog' as const,
    description: 'Test description',
    epic: 'test',
    estimate: 1,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  }],
});

// Render
try {
  const { frames } = render(<BoardView cwd="/tmp" />);
  console.log('SUCCESS:', frames[frames.length - 1]);
} catch (error) {
  console.error('ERROR:', error);
  console.error('STACK:', error.stack);
}
