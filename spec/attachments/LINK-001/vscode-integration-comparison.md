# VS Code Integration Feature Analysis

## Executive Summary

After examining `/tmp/opencode`, `/tmp/vtcode`, and `/tmp/pi-mono`, I found significant differences in their IDE integration approaches. **VTCode** has the most mature VS Code integration, **OpenCode** has a solid foundation with ACP/LSP support, and **Pi-Mono** appears to be primarily terminal-focused with no obvious IDE integration.

## VS Code Integration Feature Comparison

| Feature Category | OpenCode | VTCode | Pi-Mono |
|------------------|----------|---------|---------|
| **VS Code Extension** | ✅ `sst-dev.opencode` | ✅ `vtcode-companion` (rich features) | ❌ None |
| **Language Server Protocol** | ✅ Full LSP implementation | ✅ Tree-sitter integration | ❌ None |
| **Agent Communication Protocol** | ✅ ACP v1 compliant | ✅ ACP support | ❌ None |
| **File Linking/Navigation** | ✅ LSP-based navigation | ✅ File linking + navigation | ❌ None |
| **Chat Integration** | ❌ Terminal only | ✅ Webview chat interface | ❌ None |
| **Context Menu Integration** | ✅ Basic | ✅ "Ask About Selection" | ❌ None |
| **Status Bar Integration** | ✅ CLI detection | ✅ Rich status indicators | ❌ None |
| **Activity Bar** | ❌ None | ✅ Custom activity bar view | ❌ None |
| **Command Palette** | ✅ Basic commands | ✅ Extensive command set | ❌ None |

## IDE Integration Architectures

### OpenCode Architecture
| Component | Purpose | Implementation |
|-----------|---------|----------------|
| **LSP Server** | Code intelligence | Full LSP v3.17 with diagnostics, hover, definition, references |
| **ACP Agent** | IDE communication | JSON-RPC over stdio using `@agentclientprotocol/sdk` |
| **VS Code Extension** | Frontend | Lightweight wrapper that launches CLI in terminal |
| **IDE Detection** | Auto-setup | Uses `TERM_PROGRAM` and `GIT_ASKPASS` environment variables |

### VTCode Architecture  
| Component | Purpose | Implementation |
|-----------|---------|----------------|
| **VS Code Extension** | Rich integration | Full-featured extension with webviews, tree views, commands |
| **ACP Support** | Editor integration | Agent Client Protocol for Zed integration |
| **Tree-sitter** | Code intelligence | Semantic code understanding with multiple language support |
| **Chat Interface** | User interaction | Embedded webview chat within VS Code |
| **MCP Integration** | Tool ecosystem | Model Context Protocol for extensibility |

### Pi-Mono Architecture
| Component | Purpose | Implementation |
|-----------|---------|----------------|
| **Terminal UI** | Primary interface | Rich TUI using custom `@mariozechner/pi-tui` library |
| **SDK** | Programmatic access | Embeddable agent runtime for custom applications |
| **Extensions** | Customization | TypeScript-based extension system |

## Key Integration Features

### File Navigation & Linking
| Feature | OpenCode | VTCode | Pi-Mono |
|---------|----------|---------|---------|
| **Go-to-Definition** | ✅ LSP implementation | ✅ Tree-sitter + LSP | ❌ |
| **Find References** | ✅ LSP implementation | ✅ Tree-sitter + LSP | ❌ |
| **Symbol Search** | ✅ Workspace symbols | ✅ Document/workspace symbols | ❌ |
| **Hover Information** | ✅ LSP hover | ✅ LSP hover | ❌ |
| **Clickable File Links** | ✅ Via LSP locations | ✅ Via LSP + file protocols | ❌ |

### User Experience Features
| Feature | OpenCode | VTCode | Pi-Mono |
|---------|----------|---------|---------|
| **Workspace Trust** | ✅ Security model | ✅ VS Code trust integration | ❌ |
| **Configuration UI** | ❌ CLI/TOML only | ✅ Settings integration | ✅ Interactive CLI |
| **Progress Indicators** | ❌ Terminal only | ✅ VS Code progress API | ✅ Terminal spinners |
| **Error Handling** | ✅ LSP diagnostics | ✅ Rich error display | ✅ Terminal formatting |
| **Keyboard Shortcuts** | ✅ Basic | ✅ Extensive bindings | ✅ Terminal bindings |

## Communication Protocols

### Agent Communication Protocol (ACP)
| Aspect | OpenCode | VTCode | Pi-Mono |
|--------|----------|---------|---------|
| **Version** | v1 compliant | v1 compliant | ❌ Not supported |
| **Transport** | JSON-RPC over stdio | JSON-RPC over stdio | ❌ |
| **Session Management** | ✅ `session/new`, `session/load` | ✅ Full session support | ❌ |
| **Streaming** | ❌ Complete responses only | ✅ Real-time streaming | ❌ |
| **Tool Reporting** | ❌ Limited | ✅ Tool execution progress | ❌ |

### Language Server Protocol (LSP)
| Aspect | OpenCode | VTCode | Pi-Mono |
|--------|----------|---------|---------|
| **Server Implementation** | ✅ Full featured | ✅ Tree-sitter enhanced | ❌ |
| **Client Implementation** | ✅ VS Code JSON-RPC | ✅ Native VS Code LSP | ❌ |
| **Multi-language Support** | ✅ Configurable servers | ✅ Rust, Python, JS/TS, Go, etc. | ❌ |
| **Diagnostics** | ✅ Real-time | ✅ Real-time | ❌ |

## Technical Implementation Details

### OpenCode - LSP Implementation
- **File**: `/packages/opencode/src/lsp/index.ts`
- **Features**: Workspace symbols, document symbols, hover, definition, references
- **Client**: Uses `vscode-jsonrpc` for VS Code communication
- **Server Management**: Dynamic server spawning based on file extensions
- **Diagnostics**: Real-time error reporting with severity levels

### OpenCode - ACP Implementation  
- **File**: `/packages/opencode/src/acp/agent.ts`
- **Protocol**: Agent Client Protocol v1 compliant
- **Transport**: JSON-RPC over stdio using `@agentclientprotocol/sdk`
- **Sessions**: Maps ACP sessions to internal OpenCode sessions
- **Tools**: File operations, terminal support, permission requests

### VTCode - VS Code Extension
- **File**: `/vscode-extension/src/extension.ts` (110,000+ lines)
- **Features**: Comprehensive VS Code integration with webviews, tree views
- **Chat Interface**: Embedded chat panel with rich UI components
- **Commands**: Extensive command palette integration (40+ commands)
- **Language Features**: Syntax highlighting for `vtcode.toml` configuration files

### VTCode - Language Features
- **File**: `/vscode-extension/src/languageFeatures.ts`
- **Configuration**: Intelligent completion for TOML configuration files
- **Validation**: Schema validation for configuration sections
- **Documentation**: Inline documentation for configuration options

## Implementation Requirements for fspec

### Minimal VS Code Integration (OpenCode approach)
| Requirement | Effort Level | Description |
|-------------|--------------|-------------|
| **VS Code Extension** | Medium | Basic extension with command palette integration |
| **Terminal Integration** | Low | Launch fspec in VS Code integrated terminal |
| **File Protocol Support** | Low | Support `vscode://file/` URLs for navigation |
| **Basic Commands** | Low | Commands for common fspec operations |

### Rich VS Code Integration (VTCode approach)  
| Requirement | Effort Level | Description |
|-------------|--------------|-------------|
| **Webview Chat Interface** | High | Embedded chat panel within VS Code |
| **Tree View Providers** | Medium | Custom tree views for project structure |
| **LSP Integration** | High | Language server for Gherkin/feature files |
| **Status Bar Integration** | Low | Show fspec status and quick actions |
| **Context Menu Actions** | Medium | Right-click actions on files/selections |

### Protocol Integration Options
| Protocol | Complexity | Benefits | Use Case |
|----------|------------|----------|----------|
| **ACP** | Medium | Standard protocol, works with multiple editors | Multi-editor support |
| **LSP** | High | Rich language features, standard protocol | Code intelligence |
| **Custom Extension** | Medium | Full control, VS Code specific | Rich VS Code experience |
| **Terminal + File URLs** | Low | Simple, reliable | Basic file navigation |

## Recommended Implementation Strategy

### Phase 1: Basic Integration (2-4 weeks)
1. **VS Code Extension**: Basic extension with command palette integration
2. **Terminal Integration**: Launch fspec in VS Code terminal with proper working directory
3. **File Navigation**: Support clickable file paths using `vscode://file/` protocol
4. **Basic Commands**: Add common fspec commands to command palette

**Key Files to Create:**
```
vscode-extension/
├── package.json         # Extension manifest
├── src/
│   ├── extension.ts     # Main extension entry point
│   ├── commands.ts      # Command implementations
│   └── terminal.ts      # Terminal integration
└── README.md           # Extension documentation
```

### Phase 2: Enhanced Integration (4-8 weeks)
1. **LSP for Gherkin**: Language server for .feature files with syntax highlighting
2. **Tree View**: Custom tree view showing work units and their status
3. **Status Bar**: Show current work unit status and quick actions
4. **Settings Integration**: VS Code settings for fspec configuration

**Additional Files:**
```
lsp/
├── server.ts           # LSP server implementation
├── gherkin-parser.ts   # Gherkin syntax analysis
└── diagnostics.ts      # Error reporting

vscode-extension/src/
├── treeview.ts         # Work unit tree view
├── statusbar.ts        # Status bar integration
└── settings.ts         # VS Code settings
```

### Phase 3: Advanced Integration (8-12 weeks)
1. **Webview Chat**: Embedded chat interface for interacting with fspec
2. **ACP Support**: Implement Agent Communication Protocol for broader editor support
3. **Advanced Navigation**: Go-to-definition for scenario steps, feature linking
4. **Workflow Automation**: Automated work unit creation from VS Code actions

**Advanced Files:**
```
acp/
├── agent.ts            # ACP agent implementation
├── session.ts          # Session management
└── protocol.ts         # ACP protocol handlers

vscode-extension/src/
├── chatview.ts         # Webview chat interface
├── navigation.ts       # Advanced navigation features
└── automation.ts       # Workflow automation
```

## Key Learnings from Analysis

### OpenCode Strengths
- Clean separation between CLI and IDE integration
- Robust LSP implementation with multi-language support
- ACP compliance for broad editor compatibility
- Automatic IDE detection and extension installation

### VTCode Strengths
- Rich VS Code integration with comprehensive UI components
- Excellent user experience with context menus, tree views
- Advanced configuration management with intellisense
- Embedded chat interface for seamless interaction

### Pi-Mono Strengths
- Extensible architecture with TypeScript-based extensions
- Clean SDK for programmatic integration
- Rich terminal UI as primary interface
- Focus on terminal workflows rather than IDE integration

### Implementation Recommendations for fspec

1. **Start with OpenCode's approach**: Basic extension + terminal integration
2. **Add VTCode's UX patterns**: Tree views for work units, status indicators
3. **Consider ACP for multi-editor support**: Future-proof for Zed, Cursor, etc.
4. **Implement Gherkin LSP**: Rich language support for .feature files
5. **Gradual enhancement**: Build incrementally from basic to advanced features

This analysis provides a comprehensive foundation for implementing VS Code integration features in the fspec codebase, with clear implementation paths based on proven patterns from these mature projects.