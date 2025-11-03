#!/usr/bin/env tsx
/**
 * Dev script to test UnifiedBoardLayout directly
 */
import React from 'react';
import { render } from 'ink';
import { UnifiedBoardLayout } from './src/tui/components/UnifiedBoardLayout.js';
import fs from 'fs';
import path from 'path';

const workUnitsPath = path.join(process.cwd(), 'spec', 'work-units.json');
const workUnitsData = JSON.parse(fs.readFileSync(workUnitsPath, 'utf-8'));
const workUnits = Object.values(workUnitsData.workUnits);

const selectedWorkUnit = workUnits.find((wu: any) =>
  wu.id === 'EXMAP-001'
) || workUnits[0];

const App = () => {
  return (
    <UnifiedBoardLayout
      workUnits={workUnits}
      selectedWorkUnit={selectedWorkUnit}
      cwd={process.cwd()}
      terminalWidth={40}
      terminalHeight={24}
    />
  );
};

render(<App />);
