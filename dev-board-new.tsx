#!/usr/bin/env tsx
/**
 * Dev script to test BoardViewNew with UnifiedBoardLayoutNew
 */
import React from 'react';
import { render } from 'ink';
import { BoardViewNew } from './src/tui/components/BoardViewNew.js';

const App = () => {
  return <BoardViewNew />;
};

render(<App />);
