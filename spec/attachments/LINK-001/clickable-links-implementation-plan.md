# Clickable Links Implementation Plan

## Overview

Plan to add clickable file links and URL opening to fspec's conversation display in both `AgentView.tsx` and `SplitSessionView.tsx` (via `VirtualList.tsx` and `conversationUtils.ts`).

## Message Flow Architecture

| Component | Role | Message Processing |
|-----------|------|-------------------|
| `AgentView.tsx` | Primary conversation view | Uses `messagesToLines()` to convert `ConversationMessage[]` → `ConversationLine[]` |
| `SplitSessionView.tsx` | Split view for watcher sessions | Uses same `messagesToLines()` conversion |
| `VirtualList.tsx` | Virtual scrolling renderer | Renders `ConversationLine[]` via `renderItem` prop |
| `conversationUtils.ts` | Message processing utilities | `wrapMessageToLines()` converts single message to multiple lines |

## Implementation Strategy

### Phase 1: Core Link Detection & Parsing

| Step | Component | Implementation |
|------|-----------|----------------|
| **1.1** | Create `linkUtils.ts` | Regex patterns for file paths and URLs |
| **1.2** | Create `LinkifiedText.tsx` | React component to render clickable text segments |
| **1.3** | Update `ConversationLine` type | Add `linkSegments?: LinkSegment[]` field |
| **1.4** | Update `conversationUtils.ts` | Parse links during line wrapping |

### Phase 2: Link Interaction & Opening

| Step | Component | Implementation |
|------|-----------|----------------|
| **2.1** | Create `linkHandlers.ts` | VS Code integration and URL opening logic |
| **2.2** | Update `VirtualList.tsx` | Handle link clicks within rendered items |
| **2.3** | Update `AgentView.tsx` | Wire up link handlers to conversation display |
| **2.4** | Update `SplitSessionView.tsx` | Wire up link handlers for watcher view |

### Phase 3: Visual Styling & UX

| Step | Component | Implementation |
|------|-----------|----------------|
| **3.1** | Update `LinkifiedText.tsx` | Add hover states, underlines, colors |
| **3.2** | Add keyboard navigation | Tab/Enter support for link selection |
| **3.3** | Add link preview | Show file path/URL on hover |
| **3.4** | Add error handling | Handle invalid paths/unreachable URLs |

## Reusable Patterns from VTCode Analysis

### VTCode Integration Patterns We Can Leverage

| Pattern | Source File | Reusable Code | Adaptation for fspec |
|---------|-------------|---------------|---------------------|
| **VS Code URI Creation** | `extension.ts` | `getPathLabel()`, URI handling | Adapt for file link resolution |
| **Click Event Delegation** | `media/chat-view.js` | DOM event handling pattern | Adapt for Ink.js terminal events |
| **Path Resolution Logic** | `vtcodeConfig.ts` | Relative path handling | Use for file path normalization |
| **Link Type Detection** | Multiple files | File vs URL distinction | Enhance with our regex patterns |

### VTCode Code Patterns to Integrate

#### 1. Path Label Generation (from VTCode `extension.ts`)
```typescript
// VTCode pattern - adapt for our file path resolution
function getPathLabel(uri: vscode.Uri): string {
    if (uri.scheme === "untitled") {
        const segments = uri.path.split("/");
        return segments[segments.length - 1] || "Untitled";
    }
    
    const relative = vscode.workspace.asRelativePath(uri, false);
    if (relative && relative !== uri.toString()) {
        return relative;
    }
    
    if (uri.scheme === "file") {
        return uri.fsPath;
    }
    
    return uri.toString(true);
}

// Adapted for fspec
function resolveDisplayPath(filePath: string, workingDirectory: string): string {
    if (path.isAbsolute(filePath)) {
        // Try to make relative to working directory for better display
        const relative = path.relative(workingDirectory, filePath);
        return relative.length < filePath.length ? relative : filePath;
    }
    return filePath;
}
```

#### 2. Click Event Delegation Pattern (from VTCode `chat-view.js`)
```typescript
// VTCode pattern - adapt for Ink.js
const setupLinkHandlers = (container) => {
    container.addEventListener('click', (e) => {
        const link = e.target.closest('[data-link-type]');
        if (!link) return;
        
        const linkType = link.dataset.linkType;
        const href = link.dataset.href;
        
        switch (linkType) {
            case 'file-absolute':
            case 'file-relative':
                handleFileLink(href);
                break;
            case 'url-http':
            case 'url-https':
                handleUrlLink(href);
                break;
        }
    });
};

// Adapted for fspec Ink.js - will use onPress/onClick props
interface LinkifiedTextProps {
    segments: LinkSegment[];
    onLinkPress: (href: string, linkType: LinkType) => void;
    onLinkHover?: (href: string, linkType: LinkType) => void;
}
```

#### 3. VS Code Integration Methods (from VTCode)
```typescript
// VTCode VS Code URI creation pattern
const createVSCodeUri = (filePath: string, line?: number, column?: number): string => {
    let uri = `vscode://file/${encodeURIComponent(filePath)}`;
    if (line !== undefined) {
        uri += `:${line}`;
        if (column !== undefined) {
            uri += `:${column}`;
        }
    }
    return uri;
};

// Environment detection for VS Code (from VTCode patterns)
const detectVSCodeEnvironment = (): boolean => {
    return !!(
        process.env.VSCODE_PID ||
        process.env.TERM_PROGRAM === 'vscode' ||
        process.env.VSCODE_IPC_HOOK ||
        process.env.VSCODE_IPC_HOOK_CLI
    );
};
```

#### 4. File Path Context Key Creation (from VTCode)
```typescript
// VTCode context key pattern for tracking file references
function createContextKey(
    uri: vscode.Uri | undefined,
    range: vscode.Range | undefined,
    fallback: string
): string {
    const base = uri ? uri.toString() : fallback;
    if (!range) return base;
    
    return `${base}:${range.start.line + 1}:${range.start.character + 1}`;
}

// Adapted for fspec link tracking
function createLinkKey(filePath: string, line?: number, column?: number): string {
    let key = filePath;
    if (line !== undefined) {
        key += `:${line}`;
        if (column !== undefined) {
            key += `:${column}`;
        }
    }
    return key;
}
```

## Detailed Implementation

### 1. Enhanced Link Detection Types (`src/tui/types/links.ts`)

```typescript
export type LinkType = 'file-absolute' | 'file-relative' | 'url-http' | 'url-https' | 'url-vscode';

export interface LinkSegment {
  type: 'text' | 'link';
  content: string;
  linkType?: LinkType;
  href?: string;  // Resolved absolute path or full URL
  displayPath?: string;  // VTCode pattern: shorter display version
  contextKey?: string;   // VTCode pattern: unique identifier for tracking
  start?: number; // Character position in original line
  end?: number;   // Character position in original line
  line?: number;  // Extracted line number (if present)
  column?: number; // Extracted column number (if present)
}

export interface ParsedLine {
  segments: LinkSegment[];
  hasLinks: boolean;
}

// VTCode-inspired environment detection
export interface LinkEnvironment {
  isVSCodeAvailable: boolean;
  workingDirectory: string;
  preferredEditor: 'vscode' | 'system';
}
```

### 2. Enhanced Link Detection Utilities (`src/tui/utils/linkUtils.ts`)

| Function | Purpose | VTCode-Inspired Implementation |
|----------|---------|-------------------------------|
| `parseLinksInText()` | Extract all links from text | Enhanced with VTCode's context tracking |
| `resolveFilePath()` | Convert relative → absolute paths | Uses VTCode's path resolution patterns |
| `validateFilePath()` | Check if file/directory exists | File system validation with caching |
| `isValidUrl()` | Validate HTTP/HTTPS URLs | URL constructor validation |
| `createVSCodeUri()` | Generate `vscode://file/` URI | VTCode's URI creation pattern |
| `detectLinkEnvironment()` | **NEW**: Detect VS Code availability | VTCode's environment detection |
| `createDisplayPath()` | **NEW**: Generate user-friendly paths | VTCode's path label generation |
| `createLinkContextKey()` | **NEW**: Generate unique link identifiers | VTCode's context key pattern |

#### VTCode-Enhanced Implementation Examples

```typescript
// Enhanced with VTCode patterns
export function detectLinkEnvironment(): LinkEnvironment {
    const isVSCodeAvailable = !!(
        process.env.VSCODE_PID ||
        process.env.TERM_PROGRAM === 'vscode' ||
        process.env.VSCODE_IPC_HOOK ||
        process.env.VSCODE_IPC_HOOK_CLI
    );
    
    return {
        isVSCodeAvailable,
        workingDirectory: process.cwd(),
        preferredEditor: isVSCodeAvailable ? 'vscode' : 'system'
    };
}

export function createDisplayPath(filePath: string, workingDir: string): string {
    if (!path.isAbsolute(filePath)) {
        return filePath; // Already relative
    }
    
    // VTCode pattern: prefer relative paths for display
    const relative = path.relative(workingDir, filePath);
    return relative.length < filePath.length ? relative : filePath;
}

export function parseLinksInText(text: string, environment: LinkEnvironment): LinkSegment[] {
    const segments: LinkSegment[] = [];
    // Enhanced regex patterns with VTCode-style context tracking
    // ... implementation with contextKey generation
}
```

### 3. Regex Patterns

| Pattern Type | Regex | Description |
|--------------|-------|-------------|
| **Absolute File Path** | `/^(\/[^\s:*?"<>|]+)(?::(\d+)(?::(\d+))?)?/` | `/path/to/file:line:col` |
| **Relative File Path** | `/^(\.{0,2}\/[^\s:*?"<>|]+)(?::(\d+)(?::(\d+))?)?/` | `./file` or `../file:line:col` |
| **Windows Path** | `/^([A-Z]:[\\\/][^\s:*?"<>|]+)(?::(\d+)(?::(\d+))?)?/` | `C:\path\to\file:line:col` |
| **HTTP/HTTPS URL** | `/https?:\/\/(www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@:%_\+.~#?&//=]*)/` | Standard URL pattern |
| **VS Code URI** | `/vscode:\/\/file\/([^\s]+)(?::(\d+)(?::(\d+))?)?/` | `vscode://file/path:line:col` |

### 4. Enhanced LinkifiedText Component (`src/tui/components/LinkifiedText.tsx`)

```typescript
// Enhanced with VTCode interaction patterns
interface LinkifiedTextProps {
  content: string;
  segments?: LinkSegment[];
  environment?: LinkEnvironment; // VTCode-style environment context
  onLinkClick?: (href: string, linkType: LinkType, contextKey?: string) => void;
  onLinkHover?: (href: string, linkType: LinkType, displayPath?: string) => void;
  onLinkFocus?: (contextKey: string) => void; // VTCode-style focus tracking
}

// VTCode-inspired component with enhanced interaction
export const LinkifiedText: React.FC<LinkifiedTextProps> = ({
  content,
  segments,
  environment,
  onLinkClick,
  onLinkHover,
  onLinkFocus
}) => {
  // Render segments with VTCode-style interaction patterns
  return (
    <Box>
      {segments?.map((segment, index) => {
        if (segment.type === 'link') {
          return (
            <Text
              key={index}
              color="blue"
              underline
              onPress={() => onLinkClick?.(
                segment.href!,
                segment.linkType!,
                segment.contextKey
              )}
              onHover={() => onLinkHover?.(
                segment.href!,
                segment.linkType!,
                segment.displayPath
              )}
              // VTCode-style focus handling for keyboard navigation
              tabIndex={0}
              onFocus={() => onLinkFocus?.(segment.contextKey!)}
            >
              {segment.displayPath || segment.content}
            </Text>
          );
        }
        return <Text key={index}>{segment.content}</Text>;
      })}
    </Box>
  );
};
```

### 5. Enhanced Conversation Types

| Field | Type | Purpose | VTCode Enhancement |
|-------|------|---------|-------------------|
| `ConversationLine.linkSegments?` | `LinkSegment[]` | Parsed link segments for this line | Enhanced with context tracking |
| `ConversationLine.hasLinks?` | `boolean` | Quick check for link presence | Performance optimization |
| `ConversationLine.linkEnvironment?` | `LinkEnvironment` | **NEW**: Environment context | VS Code detection per line |
| `ConversationMessage.linkSegments?` | `LinkSegment[]` | Message-level link cache | Enhanced with display paths |
| `ConversationMessage.linkContextKeys?` | `string[]` | **NEW**: Unique identifiers | VTCode-style tracking |

### 6. Enhanced conversationUtils.ts Updates

| Function | Changes | VTCode Enhancements |
|----------|---------|-------------------|
| `wrapMessageToLines()` | Parse links in message content before line wrapping | Add environment detection per message |
| `messagesToLines()` | Aggregate link parsing across all messages | Context key tracking across messages |
| New: `parseMessageLinks()` | Extract and resolve links from message content | VTCode-style path resolution |
| New: `preserveLinksInWrap()` | Maintain link boundaries when wrapping long lines | Enhanced with display path optimization |
| New: `trackLinkContext()` | **NEW**: Track link interactions | VTCode-inspired context management |

### 7. Enhanced Link Handlers (`src/tui/utils/linkHandlers.ts`)

| Handler | Implementation | VTCode Enhancements | Error Handling |
|---------|----------------|-------------------|----------------|
| `handleFileLink()` | Open with VS Code API or external editor | VTCode environment detection | File not found → show error message |
| `handleHttpLink()` | Open with system default browser | Enhanced URL validation | Network error → show error message |
| `handleVSCodeLink()` | Parse and redirect to VS Code | VTCode URI pattern implementation | Invalid format → show error message |
| `createVSCodeCommand()` | Generate VS Code open command | VTCode extension integration patterns | Fallback to file system open |
| New: `handleLinkWithContext()` | **NEW**: Context-aware link handling | VTCode context key tracking | Enhanced error context |
| New: `validateLinkEnvironment()` | **NEW**: Validate environment before opening | VTCode environment checks | Graceful degradation |

#### VTCode-Enhanced Handler Implementation

```typescript
// Enhanced with VTCode patterns
export async function handleFileLink(
    href: string,
    environment: LinkEnvironment,
    contextKey?: string,
    line?: number,
    column?: number
): Promise<void> {
    try {
        // VTCode pattern: prefer VS Code if available
        if (environment.isVSCodeAvailable) {
            const vscodeUri = createVSCodeUri(href, line, column);
            await openWithVSCode(vscodeUri);
            return;
        }
        
        // Fallback to system default
        await openWithSystemDefault(href);
        
    } catch (error) {
        // Enhanced error handling with context
        showErrorWithContext(error, contextKey, href);
    }
}

export function createVSCodeCommand(filePath: string, line?: number, column?: number): string[] {
    // VTCode pattern: command line integration
    const args = ['code'];
    
    if (line !== undefined) {
        args.push('--goto', `${filePath}:${line}:${column || 1}`);
    } else {
        args.push(filePath);
    }
    
    return args;
}
```

### 8. Enhanced VS Code Integration

| Method | Implementation | VTCode Patterns | Fallback |
|--------|----------------|----------------|----------|
| **Environment Detection** | Multi-variable VS Code detection | `VSCODE_PID`, `TERM_PROGRAM`, `VSCODE_IPC_HOOK` checks | Generic file opening |
| **VS Code URI Generation** | Line/column-aware URI creation | `vscode://file/path:line:col` with encoding | File system open |
| **Command Line Integration** | Direct `code` command execution | `code --goto path:line:col` pattern | System default editor |
| **Context Key Tracking** | Link interaction tracking | VTCode's context key patterns | Basic link tracking |
| **Path Label Display** | User-friendly path display | VTCode's relative path preferences | Full path display |

#### VTCode Integration Implementation

```typescript
// Enhanced VS Code integration based on VTCode patterns
export class VSCodeIntegration {
    private environment: LinkEnvironment;
    
    constructor() {
        this.environment = detectLinkEnvironment();
    }
    
    // VTCode pattern: environment detection
    private detectVSCode(): boolean {
        return !!(
            process.env.VSCODE_PID ||
            process.env.TERM_PROGRAM === 'vscode' ||
            process.env.VSCODE_IPC_HOOK ||
            process.env.VSCODE_IPC_HOOK_CLI
        );
    }
    
    // VTCode pattern: URI creation with encoding
    createUri(filePath: string, line?: number, column?: number): string {
        let uri = `vscode://file/${encodeURIComponent(path.resolve(filePath))}`;
        if (line !== undefined) {
            uri += `:${line}`;
            if (column !== undefined) {
                uri += `:${column}`;
            }
        }
        return uri;
    }
    
    // VTCode pattern: command line fallback
    async openFile(filePath: string, line?: number, column?: number): Promise<void> {
        if (this.environment.isVSCodeAvailable) {
            const command = line !== undefined 
                ? ['code', '--goto', `${filePath}:${line}:${column || 1}`]
                : ['code', filePath];
            
            await execCommand(command);
        } else {
            // System default fallback
            await openWithSystemDefault(filePath);
        }
    }
}

### 9. Enhanced VirtualList.tsx Integration

| Change | Implementation | VTCode Enhancements |
|--------|----------------|-------------------|
| **Link Click Handling** | Add `onLinkClick` prop to `VirtualListProps` | Context-aware click handling |
| **Render Item Update** | Pass link handlers to rendered items | Enhanced with environment detection |
| **Keyboard Navigation** | Support Tab navigation between links | VTCode-style focus management |
| **Focus Management** | Maintain focus state for link selection | Context key-based tracking |
| **Link State Tracking** | **NEW**: Track link interaction state | VTCode-inspired state management |

#### VTCode-Enhanced VirtualList Props

```typescript
interface VirtualListProps<T> {
    // ... existing props
    
    // Enhanced link handling with VTCode patterns
    linkEnvironment?: LinkEnvironment;
    onLinkClick?: (href: string, linkType: LinkType, contextKey?: string) => void;
    onLinkHover?: (href: string, linkType: LinkType, displayPath?: string) => void;
    onLinkFocus?: (contextKey: string) => void;
    linkFocusState?: Map<string, boolean>; // VTCode-style focus tracking
}
```

## Enhanced Implementation Order & Effort Estimates

### Phase 1: Foundation with VTCode Patterns (1-2 weeks)
| Task | Effort | Dependencies | VTCode Enhancements |
|------|--------|--------------|-------------------|
| Create enhanced link types | 3 hours | None | Add VTCode environment and context tracking |
| Implement VTCode-enhanced regex patterns | 6 hours | Link types | Add VTCode path resolution patterns |
| Create `linkUtils.ts` with VTCode integration | 12 hours | Regex patterns | VS Code detection and path handling |
| Update `ConversationLine` type with context | 2 hours | Link types | Add VTCode context tracking fields |

### Phase 2: Core Functionality with VTCode Integration (2-3 weeks)
| Task | Effort | Dependencies | VTCode Enhancements |
|------|--------|--------------|-------------------|
| Create enhanced `LinkifiedText.tsx` | 16 hours | Link types, utils | VTCode interaction patterns |
| Update `conversationUtils.ts` | 10 hours | Link utils, LinkifiedText | Context key tracking |
| Implement VTCode-enhanced link handlers | 10 hours | VS Code research | Full VTCode integration patterns |
| Update `VirtualList.tsx` with context tracking | 6 hours | LinkifiedText component | VTCode focus management |

### Phase 3: Advanced Integration (1-2 weeks)
| Task | Effort | Dependencies | VTCode Enhancements |
|------|--------|--------------|-------------------|
| Wire up `AgentView.tsx` with context | 6 hours | All core components | Environment-aware integration |
| Wire up `SplitSessionView.tsx` with context | 6 hours | All core components | Context sharing between views |
| Add VTCode-style keyboard navigation | 8 hours | VirtualList integration | Enhanced focus management |
| Enhanced error handling & fallbacks | 6 hours | Link handlers | VTCode-style error contexts |

### Phase 4: VTCode-Level Polish (1 week)
| Task | Effort | Dependencies | VTCode Enhancements |
|------|--------|--------------|-------------------|
| VTCode-style visual enhancements | 6 hours | LinkifiedText | Enhanced hover states and indicators |
| Context-aware hover states and previews | 6 hours | Link handlers | VTCode-style path previews |
| Performance optimization with caching | 6 hours | All components | VTCode-style link caching |
| Comprehensive testing & integration | 10 hours | Full integration | VTCode interaction testing |

## Testing Strategy

| Test Type | Coverage |
|-----------|----------|
| **Unit Tests** | Regex patterns, link resolution, path validation |
| **Integration Tests** | ConversationLine parsing, VirtualList rendering |
| **E2E Tests** | File opening, URL launching, VS Code integration |
| **Performance Tests** | Large conversations with many links |

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| **Performance Impact** | Lazy parsing, caching, debounced link detection |
| **VS Code Detection** | Multiple fallback methods, environment variable checking |
| **File Path Resolution** | Robust path validation, cross-platform support |
| **URL Validation** | Safe URL parsing, malicious URL protection |
| **Link Boundary Issues** | Careful wrapping logic, link preservation across lines |

## Enhanced Configuration Options

| Setting | Default | Purpose | VTCode Enhancement |
|---------|---------|---------|------------------|
| `links.enabled` | `true` | Enable/disable link parsing | Environment-aware defaults |
| `links.maxLinksPerMessage` | `50` | Performance limit | Adaptive based on message length |
| `links.validateFiles` | `true` | Check file existence | Cached validation with VTCode patterns |
| `links.editor` | `"auto"` | Preferred editor for file links | Auto-detect VS Code availability |
| `links.showLineNumbers` | `true` | Include line numbers in file links | VTCode-style display preferences |
| `links.useRelativePaths` | `true` | **NEW**: Prefer relative path display | VTCode path label patterns |
| `links.contextTracking` | `true` | **NEW**: Enable context key tracking | VTCode-style interaction tracking |
| `links.vscodeIntegration` | `"auto"` | **NEW**: VS Code integration mode | Enhanced with VTCode detection |

### VTCode-Enhanced Configuration Examples

```typescript
// Enhanced configuration with VTCode patterns
interface LinkConfig {
    enabled: boolean;
    maxLinksPerMessage: number;
    validateFiles: boolean;
    editor: 'auto' | 'vscode' | 'system';
    showLineNumbers: boolean;
    useRelativePaths: boolean;
    contextTracking: boolean;
    vscodeIntegration: 'auto' | 'enabled' | 'disabled';
    
    // VTCode-inspired environment adaptation
    environmentDetection: {
        checkVSCodePid: boolean;
        checkTermProgram: boolean;
        checkIpcHook: boolean;
    };
    
    // VTCode-style caching
    caching: {
        pathResolution: boolean;
        fileValidation: boolean;
        contextKeys: boolean;
    };
}
```

This implementation provides a comprehensive solution for adding clickable file and URL links to fspec's conversation interface while maintaining performance and providing robust fallbacks. The VTCode enhancements add professional-grade VS Code integration, context tracking, and user experience patterns proven in a mature codebase.