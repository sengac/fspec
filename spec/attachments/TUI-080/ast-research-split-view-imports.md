# AST Research: Split View Import Usage

## SplitSessionView import
Only imported in AgentView.tsx:
- `src/tui/components/AgentView.tsx:31` — `import { SplitSessionView } from './SplitSessionView'`

## correlationMapping imports
Only imported by SplitSessionView and its test:
- `src/tui/components/SplitSessionView.tsx:39` — `import { buildCorrelationMaps } from '../utils/correlationMapping'`
- `src/tui/components/__tests__/cross-pane-correlation.test.tsx:38` — `import { buildCorrelationMaps, getHighlightedTurns } from '../../utils/correlationMapping'`

## Conclusion
Both files are safe to delete — no other consumers.
