# AI Coding Agents - October 2025

Comprehensive list of AI coding agents/assistants that fspec should support for multi-agent compatibility.

## Research Date
October 21, 2025

## Top Tier Agents (Most Popular - October 2025)

### 1. **Cline** (formerly Claude Dev)
- **Description**: Autonomous coding agent right in your IDE, capable of creating/editing files, executing commands, using the browser, and more
- **GitHub**: https://github.com/cline/cline
- **Website**: https://cline.bot/
- **Stars**: 51,000+
- **Users**: 3.2 million developers
- **License**: Apache 2.0
- **Key Features**:
  - Plan & Act mode separating strategy from implementation
  - Supports OpenRouter, Anthropic, OpenAI, Google Gemini, AWS Bedrock, Azure, GCP Vertex, Cerebras, Groq
  - MCP (Model Context Protocol) Marketplace (v3.4, Feb 2025)
  - Fully local - code never leaves your machine
- **Status**: Free and open source (you pay for AI model usage only)

### 2. **Cursor**
- **Description**: AI-native code editor with best-in-class predictive completion
- **GitHub**: Not fully open source (proprietary)
- **Website**: https://cursor.sh/
- **Key Features**:
  - Cursor 1.0 launched in 2025 with unified request-based pricing
  - Background Agent capabilities
  - Tab completion model with cross-file change prediction
  - Almost supernatural ability to predict entire functions
- **Status**: Commercial product with free tier

### 3. **Roo Code**
- **Description**: AI dev team with specialized personalities for different work types
- **GitHub**: https://github.com/RooCodeInc/Roo-Code
- **Website**: https://roocode.com/
- **Funding**: $60 million+ raised
- **Users**: Companies like OpenAI, Shopify, Instacart
- **Key Features**:
  - Custom modes for specialized AI personalities (security expert, performance optimizer, documentation writer, QA engineer)
  - Started as Cline fork, evolved into distinct product
  - Multi-agent approach to development
- **Status**: Commercial with investment backing

## CLI-Based Agents

### 4. **Aider**
- **Description**: AI pair programming in your terminal
- **GitHub**: https://github.com/paul-gauthier/aider (now https://github.com/Aider-AI/aider)
- **Stars**: 37,700+
- **Key Features**:
  - CLI-based workflow
  - Automatic commits with sensible messages
  - Multi-language support
  - Deep Git integration
  - Works with Claude 3.7 Sonnet, DeepSeek R1 & Chat V3, OpenAI o1, o3-mini & GPT-4o
  - Local LLM support
  - Codebase mapping for large projects
  - Voice input support
- **Best For**: Terminal-first developers, privacy-conscious teams
- **Status**: Free and open source

### 5. **Claude Code**
- **Description**: Anthropic's official CLI for Claude
- **GitHub**: Part of Anthropic ecosystem
- **Website**: https://claude.com/claude-code
- **Key Features**:
  - Official Anthropic implementation
  - Deep integration with Claude models
  - System-reminder pattern for AI guidance
  - Slash command support
- **Status**: Official Anthropic product
- **spec-kit Support**: ✅ Fully supported

### 6. **Gemini CLI**
- **Description**: Google's command-line interface for Gemini
- **GitHub**: Part of Google Cloud SDK
- **Key Features**:
  - Google's implementation for Gemini models
  - Integration with Google Cloud ecosystem
- **Status**: Official Google product
- **spec-kit Support**: ✅ Fully supported

### 7. **Amazon Q Developer CLI**
- **Description**: Agentic chat experience in your terminal
- **GitHub**: https://github.com/aws/amazon-q-developer-cli
- **Key Features**:
  - Build applications using natural language
  - CLI autocompletions
  - AI chat in terminal (local and SSH)
  - Can read/write files, generate diffs, run shell commands
  - Highest scores on SWE-Bench Leaderboard
  - GitHub integration for PRs and code reviews
- **Limitations**: Does not support custom arguments for slash commands
- **Status**: AWS official product
- **spec-kit Support**: ⚠️ Partial (no custom slash command args)

### 8. **Auggie CLI (Augment Code)**
- **Description**: Agentic CLI for developer workflows
- **GitHub**: https://github.com/augmentcode/auggie
- **Website**: https://www.augmentcode.com/product/CLI
- **Installation**: `npm install -g @augmentcode/auggie`
- **Key Features**:
  - Leading context engine for autonomous codebase understanding
  - Unix-style utility for scripting and automation
  - Built-in `/github-workflow` for GitHub Actions generation
  - CI pipeline integration
  - Automated testing and deployment
- **Status**: Commercial (Enterprise rollout)
- **spec-kit Support**: ✅ Fully supported

### 9. **Codex CLI**
- **Description**: OpenAI-based coding assistant
- **GitHub**: Part of OpenAI ecosystem
- **Key Features**:
  - Based on OpenAI Codex models
  - CLI interface for OpenAI coding capabilities
- **Status**: OpenAI product
- **spec-kit Support**: ✅ Fully supported

## IDE/Editor Integrations

### 10. **GitHub Copilot**
- **Description**: AI pair programmer from GitHub
- **GitHub**: Proprietary (GitHub-owned)
- **Website**: https://github.com/features/copilot
- **Key Features**:
  - VS Code, JetBrains, Neovim integration
  - Code completion and suggestions
  - Chat interface
  - Pull request summaries
  - Deep GitHub integration
- **Status**: Commercial product from Microsoft/GitHub
- **spec-kit Support**: ✅ Fully supported (via VS Code)

### 11. **Windsurf (Codeium)**
- **Description**: First agentic IDE that keeps developers in flow
- **Website**: https://codeium.com/windsurf
- **Platforms**: Mac, Windows, Linux
- **Key Features**:
  - Agentic approach (not just suggestions - plans and executes multi-step changes)
  - Built by Codeium team
  - Collaborative copilot + independent agent modes
  - Full IDE experience
- **Status**: Commercial product
- **spec-kit Support**: ✅ Fully supported

### 12. **Continue.dev**
- **Description**: Open-source continuous AI for IDE, terminal, and CI
- **GitHub**: https://github.com/continuedev/continue
- **Key Features**:
  - Build and run custom agents
  - Works across IDE, terminal, CI environments
  - Local-first and privacy-conscious
  - Open source and customizable
- **Status**: Free and open source

### 13. **Cody (Sourcegraph)**
- **Description**: AI coding assistant with advanced search and codebase context
- **GitHub**: https://github.com/sourcegraph/cody
- **Website**: https://sourcegraph.com/cody
- **Key Features**:
  - VS Code, JetBrains, web support
  - Uses Claude 3.5 Sonnet and GPT-4o
  - Semantic search for context retrieval
  - Chat, autocomplete, commands
  - Free tier available
- **Status**: Freemium (Cody Free + Pro/Enterprise)

## Regional/Specialized Agents

### 14. **Qwen Code (Alibaba)**
- **Description**: Alibaba's AI coding offering
- **GitHub**: Part of Alibaba Cloud
- **Key Features**:
  - Qwen model family for coding tasks
  - Asian market focus
- **Status**: Commercial product
- **spec-kit Support**: ✅ Fully supported

### 15. **opencode**
- **Description**: Open-source AI coding alternative
- **GitHub**: Various implementations (check https://sourceforge.net/software/ai-coding-agents/integrates-with-opencode/)
- **Key Features**:
  - Open-source approach
  - Community-driven
- **Status**: Open source
- **spec-kit Support**: ✅ Fully supported

### 16. **Kilo Code**
- **Description**: Community AI coding project
- **GitHub**: Community maintained
- **Key Features**:
  - Community-driven development
- **Status**: Open source/community
- **spec-kit Support**: ✅ Fully supported

### 17. **CodeBuddy CLI**
- **Description**: CodeBuddy platform CLI tool
- **GitHub**: Part of CodeBuddy ecosystem
- **Key Features**:
  - CodeBuddy platform integration
- **Status**: Commercial
- **spec-kit Support**: ✅ Fully supported

## Summary Statistics

### spec-kit Support Matrix
- **Fully Supported**: 12 agents
- **Partially Supported**: 1 agent (Amazon Q Developer CLI)
- **Additional Popular Agents**: 4 agents (Cline, Continue.dev, Cody, Cursor full features)

### Open Source vs Commercial
- **Open Source**: Cline, Aider, Continue.dev, Cody Free, opencode, Kilo Code
- **Commercial**: Cursor, Roo Code, GitHub Copilot, Windsurf, Amazon Q Developer, Auggie CLI, Claude Code, Gemini CLI, Qwen Code, Codex CLI, CodeBuddy CLI

### By Interface Type
- **CLI/Terminal**: Aider, Claude Code, Gemini CLI, Amazon Q Developer CLI, Auggie CLI, Codex CLI, CodeBuddy CLI
- **IDE Extension**: Cline, Continue.dev, Cody, GitHub Copilot
- **Standalone IDE**: Cursor, Windsurf
- **Platform Hybrid**: Roo Code

## Configuration Approaches (from spec-kit analysis)

### spec-kit Pattern
Users specify their preferred agent during initialization:
```bash
specify init <project-name> --ai <agent-name>
```

The system checks for installed agents and allows bypass with `--ignore-agent-tools` flag for non-standard environments.

### Technology Independence
spec-kit is intentionally "technology independent" - designed to work across diverse AI platforms rather than being tied to a single vendor.

## Recommendations for fspec

1. **Priority Support** (based on popularity/usage):
   - Cline (51k stars, 3.2M users)
   - Aider (37.7k stars)
   - Cursor (market leader)
   - GitHub Copilot (enterprise standard)
   - Claude Code (current default)

2. **Open Source First**:
   - Cline, Aider, Continue.dev, Cody should have first-class support
   - Easier to test and validate
   - Community adoption

3. **CLI Tools** (natural fit for fspec):
   - Aider, Claude Code, Gemini CLI, Amazon Q Developer CLI, Auggie CLI
   - Terminal-based workflows align with fspec's CLI nature

4. **Agent-Specific Templates**:
   - Each agent may need customized:
     - Documentation format (CLAUDE.md → AGENT.md pattern)
     - System-reminder patterns (Claude-specific)
     - Slash command syntax
     - Help output formatting

## References
- spec-kit repository: https://github.com/github/spec-kit
- Cline bot: https://cline.bot/
- Cursor: https://cursor.sh/
- Aider AI: https://github.com/Aider-AI/aider
- Continue.dev: https://github.com/continuedev/continue
- Cody: https://sourcegraph.com/cody
- Amazon Q Developer: https://aws.amazon.com/q/developer/
- Augment Code: https://www.augmentcode.com/
- Windsurf: https://codeium.com/windsurf
- Shakudo AI Coding Assistants (Oct 2025): https://www.shakudo.io/blog/best-ai-coding-assistants
