# AST Research: Link Detection Integration Points

## Research Objective
Identify integration points in the fspec TUI codebase for implementing clickable file links and URL detection.

## Key Files Identified

### 1. conversationUtils.ts (Primary Integration Point)
- **Location**: `src/tui/utils/conversationUtils.ts`
- **Key Function**: `wrapMessageToLines()`
- **Purpose**: Converts `ConversationMessage` objects into `ConversationLine` objects for rendering
- **Integration Strategy**: Parse links before line wrapping, add `linkSegments` field to `ConversationLine`

```typescript
// Current signature
export const wrapMessageToLines = (
  msg: ConversationMessage,
  msgIndex: number,
  maxWidth: number,
  addSeparator: boolean = true
): ConversationLine[] => { ... }
```

### 2. ConversationLine Type (Type Extension Needed)
- **Location**: `src/tui/types/conversation.ts`
- **Required Changes**: Add optional `linkSegments` field for storing parsed link information

### 3. VirtualList.tsx (Rendering Point)
- **Location**: `src/tui/components/VirtualList.tsx`
- **Purpose**: Virtual scrolling renderer for conversation lines
- **Integration Strategy**: Enhance `renderItem` to use `LinkifiedText` component when `linkSegments` present

### 4. AgentView.tsx and SplitSessionView.tsx (Feature Consumers)
- **Locations**: 
  - `src/tui/components/AgentView.tsx`
  - `src/tui/components/SplitSessionView.tsx`
- **Purpose**: Both use `messagesToLines()` to convert messages for display
- **Integration Strategy**: Wire up link click handlers through props

## New Files to Create

### 1. linkUtils.ts
- **Location**: `src/tui/utils/linkUtils.ts`
- **Purpose**: Regex patterns for link detection, link parsing utilities
- **Key Exports**:
  - `parseLinksInText(text: string): LinkSegment[]`
  - `resolveFilePath(path: string, cwd: string): string`
  - `detectVSCodeEnvironment(): boolean`
  - `createVSCodeUri(path: string, line?: number, col?: number): string`

### 2. LinkifiedText.tsx
- **Location**: `src/tui/components/LinkifiedText.tsx`
- **Purpose**: Ink component to render text with clickable link segments
- **Props**: `segments: LinkSegment[]`, `onLinkClick: (href: string, type: LinkType) => void`

### 3. linkHandlers.ts
- **Location**: `src/tui/utils/linkHandlers.ts`
- **Purpose**: Link opening logic with VS Code integration
- **Key Exports**:
  - `handleFileLink(path: string, line?: number, col?: number): Promise<void>`
  - `handleUrlLink(url: string): Promise<void>`

## Type Definitions

### LinkSegment Interface
```typescript
interface LinkSegment {
  type: 'text' | 'link';
  content: string;
  linkType?: 'file-absolute' | 'file-relative' | 'url-http' | 'url-https';
  href?: string;        // Resolved absolute path or full URL
  line?: number;        // Line number for file:line:col notation
  column?: number;      // Column number for file:line:col notation
}
```

## Regex Patterns Needed

| Pattern Type | Regex | Example |
|-------------|-------|---------|
| Absolute Path | `/^(\/[^\s:*?"<>|]+)(?::(\d+)(?::(\d+))?)?/` | `/path/file.ts:42:10` |
| Relative Path | `/^(\.{0,2}\/[^\s:*?"<>|]+)(?::(\d+)(?::(\d+))?)?/` | `./file.ts:42` |
| HTTP/HTTPS URL | `/https?:\/\/[^\s<>]+/` | `https://github.com/user/repo` |

## Environment Detection

VS Code integration requires detecting when running in VS Code terminal:
```typescript
const isVSCodeTerminal = () => {
  return !!(
    process.env.VSCODE_PID ||
    process.env.TERM_PROGRAM === 'vscode' ||
    process.env.VSCODE_IPC_HOOK ||
    process.env.VSCODE_IPC_HOOK_CLI
  );
};
```

## Dependencies

- **open** package: For cross-platform URL/file opening
- **Ink.js**: For `onPress` handlers in LinkifiedText component
- No additional dependencies required for regex parsing

## Testing Strategy

1. **Unit Tests**: `linkUtils.test.ts` - Test regex patterns and link parsing
2. **Component Tests**: `LinkifiedText.test.tsx` - Test rendering and click handling
3. **Integration Tests**: Test VirtualList with link-enabled lines
4. **E2E Tests**: Test actual file opening in VS Code environment

---
*Research conducted for LINK-001: Implement Clickable File Links and URL Opening*
