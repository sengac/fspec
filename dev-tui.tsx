#!/usr/bin/env tsx
/**
 * Dev script to test TUI rendering with real data
 */
import React from 'react';
import { render } from 'ink';
import { UnifiedBoardLayout } from './src/tui/components/UnifiedBoardLayout.js';
import fs from 'fs';
import path from 'path';

// Load real work units from spec/work-units.json
const workUnitsPath = path.join(process.cwd(), 'spec', 'work-units.json');
const workUnitsData = JSON.parse(fs.readFileSync(workUnitsPath, 'utf-8'));
const workUnits = Object.values(workUnitsData.workUnits);

// Find a work unit in implementing state with a description
const selectedWorkUnit = workUnits.find((wu: any) =>
  wu.status === 'implementing' && wu.description
) || workUnits[0];

const App = () => {
  return (
    <UnifiedBoardLayout
      workUnits={workUnits}
      selectedWorkUnit={selectedWorkUnit}
      cwd={process.cwd()}
    />
  );
};

render(<App />);
