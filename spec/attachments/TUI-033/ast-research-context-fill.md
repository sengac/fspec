{
  "matches": [
    {
      "type": "arrow_function",
      "line": 45,
      "column": 30,
      "text": "({\n  value,\n  onChange,\n  onSubmit,\n  placeholder = '',\n  isActive = true,\n  onHistoryPrev,\n  onHistoryNext,\n}) => {\n  // Use ref to avoid stale closure issues with rapid typing\n  const valueRef = useRef(value);\n  valueRef.current = value;\n\n  useInput(\n    (input, key) => {\n      // Ignore mouse escape sequences\n      if (key.mouse || input.includes('[M') || input.includes('[<')) {\n        return;\n      }\n\n      if (key.return) {\n        onSubmit();\n        return;\n      }\n\n      if (key.backspace || key.delete) {\n        const newValue = valueRef.current.slice(0, -1);\n        valueRef.current = newValue; // Update immediately to handle rapid keystrokes\n        onChange(newValue);\n        return;\n      }\n\n\n      // NAPI-006: Shift+Arrow for history navigation (check before ignoring arrow keys)\n\n      // Check raw escape sequences first (most reliable for Shift+Arrow)\n      if (input.includes('[1;2A') || input.includes('\\x1b[1;2A')) {\n        onHistoryPrev?.();\n        return;\n      }\n      if (input.includes('[1;2B') || input.includes('\\x1b[1;2B')) {\n        onHistoryNext?.();\n        return;\n      }\n      // ink may set key.shift when shift is held\n      if (key.shift && key.upArrow) {\n        onHistoryPrev?.();\n        return;\n      }\n      if (key.shift && key.downArrow) {\n        onHistoryNext?.();\n        return;\n      }\n\n      // Ignore navigation keys (handled by other components)\n      if (\n        key.escape ||\n        key.tab ||\n        key.upArrow ||\n        key.downArrow ||\n        key.pageUp ||\n        key.pageDown\n      ) {\n        return;\n      }\n\n      // Filter to only printable characters, removing any escape sequence remnants\n      const clean = input\n        .split('')\n        .filter((ch) => {\n          const code = ch.charCodeAt(0);\n          // Only allow printable ASCII (space through tilde)\n          return code >= 32 && code <= 126;\n        })\n        .join('');\n\n      if (clean) {\n        const newValue = valueRef.current + clean;\n        valueRef.current = newValue; // Update immediately to handle rapid keystrokes\n        onChange(newValue);\n      }\n    },\n    { isActive }\n  );\n\n  return (\n    <Text>\n      {value || <Text dimColor>{placeholder}</Text>}\n      <Text inverse> </Text>\n    </Text>\n  );\n}"
    },
    {
      "type": "arrow_function",
      "line": 59,
      "column": 4,
      "text": "(input, key) => {\n      // Ignore mouse escape sequences\n      if (key.mouse || input.includes('[M') || input.includes('[<')) {\n        return;\n      }\n\n      if (key.return) {\n        onSubmit();\n        return;\n      }\n\n      if (key.backspace || key.delete) {\n        const newValue = valueRef.current.slice(0, -1);\n        valueRef.current = newValue; // Update immediately to handle rapid keystrokes\n        onChange(newValue);\n        return;\n      }\n\n\n      // NAPI-006: Shift+Arrow for history navigation (check before ignoring arrow keys)\n\n      // Check raw escape sequences first (most reliable for Shift+Arrow)\n      if (input.includes('[1;2A') || input.includes('\\x1b[1;2A')) {\n        onHistoryPrev?.();\n        return;\n      }\n      if (input.includes('[1;2B') || input.includes('\\x1b[1;2B')) {\n        onHistoryNext?.();\n        return;\n      }\n      // ink may set key.shift when shift is held\n      if (key.shift && key.upArrow) {\n        onHistoryPrev?.();\n        return;\n      }\n      if (key.shift && key.downArrow) {\n        onHistoryNext?.();\n        return;\n      }\n\n      // Ignore navigation keys (handled by other components)\n      if (\n        key.escape ||\n        key.tab ||\n        key.upArrow ||\n        key.downArrow ||\n        key.pageUp ||\n        key.pageDown\n      ) {\n        return;\n      }\n\n      // Filter to only printable characters, removing any escape sequence remnants\n      const clean = input\n        .split('')\n        .filter((ch) => {\n          const code = ch.charCodeAt(0);\n          // Only allow printable ASCII (space through tilde)\n          return code >= 32 && code <= 126;\n        })\n        .join('');\n\n      if (clean) {\n        const newValue = valueRef.current + clean;\n        valueRef.current = newValue; // Update immediately to handle rapid keystrokes\n        onChange(newValue);\n      }\n    }"
    },
    {
      "type": "arrow_function",
      "line": 114,
      "column": 16,
      "text": "(ch) => {\n          const code = ch.charCodeAt(0);\n          // Only allow printable ASCII (space through tilde)\n          return code >= 32 && code <= 126;\n        }"
    },
    {
      "type": "arrow_function",
      "line": 288,
      "column": 12,
      "text": "() => {\n    if (!isLoading || lastChunkTime === null) return;\n    const timeout = setTimeout(() => {\n      setDisplayedTokPerSec(null);\n    }, 10000);\n    return () => clearTimeout(timeout);\n  }"
    },
    {
      "type": "arrow_function",
      "line": 290,
      "column": 31,
      "text": "() => {\n      setDisplayedTokPerSec(null);\n    }"
    },
    {
      "type": "arrow_function",
      "line": 293,
      "column": 11,
      "text": "() => clearTimeout(timeout)"
    },
    {
      "type": "arrow_function",
      "line": 297,
      "column": 12,
      "text": "() => {\n    if (!isOpen) {\n      // Reset state when modal closes (fresh session each time)\n      setSession(null);\n      setConversation([]);\n      setTokenUsage({ inputTokens: 0, outputTokens: 0 });\n      setError(null);\n      setInputValue('');\n      setIsDebugEnabled(false); // AGENT-021: Reset debug state on modal close\n      // TUI-031: Reset tok/s tracking\n      streamingStartTimeRef.current = null;\n      setDisplayedTokPerSec(null);\n      setLastChunkTime(null);\n      lastChunkTimeRef.current = null;\n      rateSamplesRef.current = [];\n      sessionRef.current = null;\n      // NAPI-006: Reset history and search state\n      setHistoryEntries([]);\n      setHistoryIndex(-1);\n      setSavedInput('');\n      setIsSearchMode(false);\n      setSearchQuery('');\n      setSearchResults([]);\n      setSearchResultIndex(0);\n      setCurrentSessionId(null);\n      // NAPI-003: Reset resume mode state\n      setIsResumeMode(false);\n      setAvailableSessions([]);\n      setResumeSessionIndex(0);\n      return;\n    }\n\n    const initSession = async () => {\n      try {\n        // Dynamic import to handle ESM\n        const codeletNapi = await import('@sengac/codelet-napi');\n        const { CodeletSession, persistenceSetDataDirectory, persistenceGetHistory } = codeletNapi;\n\n        // NAPI-006: Set up persistence data directory\n        const fspecDir = getFspecUserDir();\n        try {\n          persistenceSetDataDirectory(fspecDir);\n        } catch {\n          // Ignore if already set\n        }\n\n        // Default to Claude as the primary AI provider\n        const newSession = new CodeletSession('claude');\n        setSession(newSession);\n        sessionRef.current = newSession;\n        setCurrentProvider(newSession.currentProviderName);\n        setAvailableProviders(newSession.availableProviders);\n        setTokenUsage(newSession.tokenTracker);\n\n        // NAPI-006: Session creation is deferred until first message is sent\n        // This prevents empty sessions from being persisted when user opens\n        // the modal but doesn't send any messages. See handleSubmit() for\n        // the actual session creation logic.\n\n        // NAPI-006: Load history for current project\n        try {\n\n          const history = persistenceGetHistory(currentProjectRef.current, 100);\n\n          // Convert NAPI history entries (camelCase from NAPI-RS) to our interface\n          const entries: HistoryEntry[] = history.map((h: { display: string; timestamp: string; project: string; sessionId: string; hasPastedContent?: boolean }) => ({\n            display: h.display,\n            timestamp: h.timestamp,\n            project: h.project,\n            sessionId: h.sessionId,\n            hasPastedContent: h.hasPastedContent ?? false,\n          }));\n          setHistoryEntries(entries);\n        } catch (err) {\n          logger.error(`Failed to load history: ${err instanceof Error ? err.message : String(err)}`);\n        }\n\n        setError(null);\n      } catch (err) {\n        const errorMessage =\n          err instanceof Error\n            ? err.message\n            : 'Failed to initialize AI session';\n        setError(errorMessage);\n        setSession(null);\n        sessionRef.current = null;\n      }\n    };\n\n    void initSession();\n  }"
    },
    {
      "type": "arrow_function",
      "line": 329,
      "column": 24,
      "text": "async () => {\n      try {\n        // Dynamic import to handle ESM\n        const codeletNapi = await import('@sengac/codelet-napi');\n        const { CodeletSession, persistenceSetDataDirectory, persistenceGetHistory } = codeletNapi;\n\n        // NAPI-006: Set up persistence data directory\n        const fspecDir = getFspecUserDir();\n        try {\n          persistenceSetDataDirectory(fspecDir);\n        } catch {\n          // Ignore if already set\n        }\n\n        // Default to Claude as the primary AI provider\n        const newSession = new CodeletSession('claude');\n        setSession(newSession);\n        sessionRef.current = newSession;\n        setCurrentProvider(newSession.currentProviderName);\n        setAvailableProviders(newSession.availableProviders);\n        setTokenUsage(newSession.tokenTracker);\n\n        // NAPI-006: Session creation is deferred until first message is sent\n        // This prevents empty sessions from being persisted when user opens\n        // the modal but doesn't send any messages. See handleSubmit() for\n        // the actual session creation logic.\n\n        // NAPI-006: Load history for current project\n        try {\n\n          const history = persistenceGetHistory(currentProjectRef.current, 100);\n\n          // Convert NAPI history entries (camelCase from NAPI-RS) to our interface\n          const entries: HistoryEntry[] = history.map((h: { display: string; timestamp: string; project: string; sessionId: string; hasPastedContent?: boolean }) => ({\n            display: h.display,\n            timestamp: h.timestamp,\n            project: h.project,\n            sessionId: h.sessionId,\n            hasPastedContent: h.hasPastedContent ?? false,\n          }));\n          setHistoryEntries(entries);\n        } catch (err) {\n          logger.error(`Failed to load history: ${err instanceof Error ? err.message : String(err)}`);\n        }\n\n        setError(null);\n      } catch (err) {\n        const errorMessage =\n          err instanceof Error\n            ? err.message\n            : 'Failed to initialize AI session';\n        setError(errorMessage);\n        setSession(null);\n        sessionRef.current = null;\n      }\n    }"
    },
    {
      "type": "arrow_function",
      "line": 362,
      "column": 54,
      "text": "(h: { display: string; timestamp: string; project: string; sessionId: string; hasPastedContent?: boolean }) => ({\n            display: h.display,\n            timestamp: h.timestamp,\n            project: h.project,\n            sessionId: h.sessionId,\n            hasPastedContent: h.hasPastedContent ?? false,\n          })"
    },
    {
      "type": "arrow_function",
      "line": 390,
      "column": 35,
      "text": "async () => {\n    if (!sessionRef.current || !inputValue.trim() || isLoading) return;\n\n    const userMessage = inputValue.trim();\n\n    // AGENT-021: Handle /debug command - toggle debug capture mode\n    if (userMessage === '/debug') {\n      setInputValue('');\n      try {\n        // Pass ~/.fspec as the debug directory\n        const result = sessionRef.current.toggleDebug(getFspecUserDir());\n        setIsDebugEnabled(result.enabled);\n        // Add the result message to conversation\n        setConversation(prev => [\n          ...prev,\n          { role: 'tool', content: result.message },\n        ]);\n      } catch (err) {\n        const errorMessage = err instanceof Error ? err.message : 'Failed to toggle debug mode';\n        setError(errorMessage);\n      }\n      return;\n    }\n\n    // NAPI-006: Handle /search command - enter history search mode\n    if (userMessage === '/search') {\n      setInputValue('');\n      handleSearchMode();\n      return;\n    }\n\n    // AGENT-003: Handle /clear command - clear context and reset session\n    if (userMessage === '/clear') {\n      setInputValue('');\n      try {\n        // Clear history in the Rust session (includes reinjecting context reminders)\n        sessionRef.current.clearHistory();\n        // Reset React state\n        setConversation([]);\n        setTokenUsage({ inputTokens: 0, outputTokens: 0 });\n        // Note: currentProvider, isDebugEnabled, and historyEntries are preserved\n      } catch (err) {\n        const errorMessage = err instanceof Error ? err.message : 'Failed to clear session';\n        setError(errorMessage);\n      }\n      return;\n    }\n\n    // NAPI-006: Handle /history command - show command history\n    if (userMessage === '/history' || userMessage.startsWith('/history ')) {\n      setInputValue('');\n      const allProjects = userMessage.includes('--all-projects');\n      try {\n        const { persistenceGetHistory } = await import('@sengac/codelet-napi');\n        const history = persistenceGetHistory(allProjects ? null : currentProjectRef.current, 20);\n        if (history.length === 0) {\n          setConversation(prev => [\n            ...prev,\n            { role: 'tool', content: 'No history entries found' },\n          ]);\n        } else {\n          const historyList = history.map((h: { display: string; timestamp: string }) =>\n            `- ${h.display}`\n          ).join('\\n');\n          setConversation(prev => [\n            ...prev,\n            { role: 'tool', content: `Command history:\\n${historyList}` },\n          ]);\n        }\n      } catch (err) {\n        const errorMessage = err instanceof Error ? err.message : 'Failed to get history';\n        setConversation(prev => [\n          ...prev,\n          { role: 'tool', content: `History failed: ${errorMessage}` },\n        ]);\n      }\n      return;\n    }\n\n    // NAPI-003: Handle /resume command - show session selection overlay\n    if (userMessage === '/resume') {\n      setInputValue('');\n      void handleResumeMode();\n      return;\n    }\n\n    // NAPI-006: Handle /sessions command - list all sessions\n    if (userMessage === '/sessions') {\n      setInputValue('');\n      try {\n        const { persistenceListSessions } = await import('@sengac/codelet-napi');\n        const sessions = persistenceListSessions(currentProjectRef.current);\n        if (sessions.length === 0) {\n          setConversation(prev => [\n            ...prev,\n            { role: 'tool', content: 'No sessions found for this project' },\n          ]);\n        } else {\n          const sessionList = sessions.map((s: SessionManifest) =>\n            `- ${s.name} (${s.messageCount} messages, ${s.id.slice(0, 8)}...)`\n          ).join('\\n');\n          setConversation(prev => [\n            ...prev,\n            { role: 'tool', content: `Sessions:\\n${sessionList}` },\n          ]);\n        }\n      } catch (err) {\n        const errorMessage = err instanceof Error ? err.message : 'Failed to list sessions';\n        setConversation(prev => [\n          ...prev,\n          { role: 'tool', content: `List sessions failed: ${errorMessage}` },\n        ]);\n      }\n      return;\n    }\n\n    // NAPI-006: Handle /switch <name> command - switch to another session\n    if (userMessage.startsWith('/switch ')) {\n      setInputValue('');\n      const targetName = userMessage.slice(8).trim();\n      try {\n        const { persistenceListSessions, persistenceLoadSession } = await import('@sengac/codelet-napi');\n        const sessions = persistenceListSessions(currentProjectRef.current);\n        const target = sessions.find((s: SessionManifest) => s.name === targetName);\n        if (target) {\n          setCurrentSessionId(target.id);\n          setConversation(prev => [\n            ...prev,\n            { role: 'tool', content: `Switched to session: \"${target.name}\"` },\n          ]);\n        } else {\n          setConversation(prev => [\n            ...prev,\n            { role: 'tool', content: `Session not found: \"${targetName}\"` },\n          ]);\n        }\n      } catch (err) {\n        const errorMessage = err instanceof Error ? err.message : 'Failed to switch session';\n        setConversation(prev => [\n          ...prev,\n          { role: 'tool', content: `Switch failed: ${errorMessage}` },\n        ]);\n      }\n      return;\n    }\n\n    // NAPI-006: Handle /rename <new-name> command - rename current session\n    if (userMessage.startsWith('/rename ')) {\n      setInputValue('');\n      const newName = userMessage.slice(8).trim();\n      if (!currentSessionId) {\n        setConversation(prev => [\n          ...prev,\n          { role: 'tool', content: 'No active session to rename' },\n        ]);\n        return;\n      }\n      try {\n        const { persistenceRenameSession } = await import('@sengac/codelet-napi');\n        persistenceRenameSession(currentSessionId, newName);\n        setConversation(prev => [\n          ...prev,\n          { role: 'tool', content: `Session renamed to: \"${newName}\"` },\n        ]);\n      } catch (err) {\n        const errorMessage = err instanceof Error ? err.message : 'Failed to rename session';\n        setConversation(prev => [\n          ...prev,\n          { role: 'tool', content: `Rename failed: ${errorMessage}` },\n        ]);\n      }\n      return;\n    }\n\n    // NAPI-006: Handle /fork <index> <name> command - fork session at index\n    if (userMessage.startsWith('/fork ')) {\n      setInputValue('');\n      const parts = userMessage.slice(6).trim().split(/\\s+/);\n      const index = parseInt(parts[0], 10);\n      const name = parts.slice(1).join(' ');\n      if (!currentSessionId) {\n        setConversation(prev => [\n          ...prev,\n          { role: 'tool', content: 'No active session to fork' },\n        ]);\n        return;\n      }\n      if (isNaN(index) || !name) {\n        setConversation(prev => [\n          ...prev,\n          { role: 'tool', content: 'Usage: /fork <index> <name>' },\n        ]);\n        return;\n      }\n      try {\n        const { persistenceForkSession } = await import('@sengac/codelet-napi');\n        const forkedSession = persistenceForkSession(currentSessionId, index, name);\n        setCurrentSessionId(forkedSession.id);\n        setConversation(prev => [\n          ...prev,\n          { role: 'tool', content: `Session forked at index ${index}: \"${name}\"` },\n        ]);\n      } catch (err) {\n        const errorMessage = err instanceof Error ? err.message : 'Failed to fork session';\n        setConversation(prev => [\n          ...prev,\n          { role: 'tool', content: `Fork failed: ${errorMessage}` },\n        ]);\n      }\n      return;\n    }\n\n    // NAPI-006: Handle /merge <session> <indices> command - merge messages from another session\n    if (userMessage.startsWith('/merge ')) {\n      setInputValue('');\n      const parts = userMessage.slice(7).trim().split(/\\s+/);\n      const sourceName = parts[0];\n      const indicesStr = parts[1];\n      if (!currentSessionId) {\n        setConversation(prev => [\n          ...prev,\n          { role: 'tool', content: 'No active session to merge into' },\n        ]);\n        return;\n      }\n      if (!sourceName || !indicesStr) {\n        setConversation(prev => [\n          ...prev,\n          { role: 'tool', content: 'Usage: /merge <session-name> <indices> (e.g., /merge session-b 3,4)' },\n        ]);\n        return;\n      }\n      try {\n        const { persistenceListSessions, persistenceMergeMessages } = await import('@sengac/codelet-napi');\n        const sessions = persistenceListSessions(currentProjectRef.current);\n        const source = sessions.find((s: SessionManifest) => s.name === sourceName || s.id === sourceName);\n        if (!source) {\n          setConversation(prev => [\n            ...prev,\n            { role: 'tool', content: `Source session not found: \"${sourceName}\"` },\n          ]);\n          return;\n        }\n        const indices = indicesStr.split(',').map((s: string) => parseInt(s.trim(), 10));\n        const result = persistenceMergeMessages(currentSessionId, source.id, indices);\n        setConversation(prev => [\n          ...prev,\n          { role: 'tool', content: `Merged ${indices.length} messages from \"${source.name}\"` },\n        ]);\n      } catch (err) {\n        const errorMessage = err instanceof Error ? err.message : 'Failed to merge messages';\n        setConversation(prev => [\n          ...prev,\n          { role: 'tool', content: `Merge failed: ${errorMessage}` },\n        ]);\n      }\n      return;\n    }\n\n    // NAPI-006: Handle /cherry-pick <session> <index> --context <n> command\n    if (userMessage.startsWith('/cherry-pick ')) {\n      setInputValue('');\n      const args = userMessage.slice(13).trim();\n      const contextMatch = args.match(/--context\\s+(\\d+)/);\n      const context = contextMatch ? parseInt(contextMatch[1], 10) : 0;\n      const cleanArgs = args.replace(/--context\\s+\\d+/, '').trim();\n      const parts = cleanArgs.split(/\\s+/);\n      const sourceName = parts[0];\n      const index = parseInt(parts[1], 10);\n      if (!currentSessionId) {\n        setConversation(prev => [\n          ...prev,\n          { role: 'tool', content: 'No active session for cherry-pick' },\n        ]);\n        return;\n      }\n      if (!sourceName || isNaN(index)) {\n        setConversation(prev => [\n          ...prev,\n          { role: 'tool', content: 'Usage: /cherry-pick <session> <index> [--context N]' },\n        ]);\n        return;\n      }\n      try {\n        const { persistenceListSessions, persistenceCherryPick } = await import('@sengac/codelet-napi');\n        const sessions = persistenceListSessions(currentProjectRef.current);\n        const source = sessions.find((s: SessionManifest) => s.name === sourceName || s.id === sourceName);\n        if (!source) {\n          setConversation(prev => [\n            ...prev,\n            { role: 'tool', content: `Source session not found: \"${sourceName}\"` },\n          ]);\n          return;\n        }\n        const result = persistenceCherryPick(currentSessionId, source.id, index, context);\n        setConversation(prev => [\n          ...prev,\n          { role: 'tool', content: `Cherry-picked message ${index} with ${context} context messages from \"${source.name}\"` },\n        ]);\n      } catch (err) {\n        const errorMessage = err instanceof Error ? err.message : 'Failed to cherry-pick';\n        setConversation(prev => [\n          ...prev,\n          { role: 'tool', content: `Cherry-pick failed: ${errorMessage}` },\n        ]);\n      }\n      return;\n    }\n\n    // NAPI-005: Handle /compact command - manual context compaction\n    if (userMessage === '/compact') {\n      setInputValue('');\n\n      // Check if there's anything to compact - use session's messages, not React state\n      if (sessionRef.current.messages.length === 0) {\n        setConversation(prev => [\n          ...prev,\n          { role: 'tool', content: 'Nothing to compact - no messages yet' },\n        ]);\n        return;\n      }\n\n      // Show compacting message\n      setConversation(prev => [\n        ...prev,\n        { role: 'tool', content: '[Compacting context...]' },\n      ]);\n\n      try {\n        const result = await sessionRef.current.compact();\n        // Show success message with metrics\n        const compressionPct = result.compressionRatio.toFixed(0);\n        const message = `[Context compacted: ${result.originalTokens}→${result.compactedTokens} tokens, ${compressionPct}% compression]\\n[Summarized ${result.turnsSummarized} turns, kept ${result.turnsKept} turns]`;\n        setConversation(prev => [\n          ...prev,\n          { role: 'tool', content: message },\n        ]);\n        // Update token tracker to reflect reduced context\n        const finalTokens = sessionRef.current.tokenTracker;\n        setTokenUsage(finalTokens);\n\n        // Persist compaction state and token usage\n        if (currentSessionId) {\n          try {\n            const { persistenceSetCompactionState, persistenceSetSessionTokens } = await import('@sengac/codelet-napi');\n            // Create summary for persistence (includes key metrics)\n            const summary = `Compacted ${result.turnsSummarized} turns (${result.originalTokens}→${result.compactedTokens} tokens, ${compressionPct}% compression)`;\n            // compacted_before_index = turnsSummarized (messages 0 to turnsSummarized-1 were compacted)\n            persistenceSetCompactionState(currentSessionId, summary, result.turnsSummarized);\n            persistenceSetSessionTokens(currentSessionId, finalTokens.inputTokens, finalTokens.outputTokens, 0, 0);\n          } catch {\n            // Compaction state persistence failed - continue\n          }\n        }\n      } catch (err) {\n        const errorMessage = err instanceof Error ? err.message : 'Failed to compact context';\n        setConversation(prev => [\n          ...prev,\n          { role: 'tool', content: `Compaction failed: ${errorMessage}` },\n        ]);\n      }\n      return;\n    }\n\n    setInputValue('');\n    setHistoryIndex(-1); // Reset history navigation\n    setSavedInput('');\n    setIsLoading(true);\n    // TUI-031: Reset tok/s tracking for new prompt\n    streamingStartTimeRef.current = Date.now();\n    setDisplayedTokPerSec(null);\n    setLastChunkTime(null);\n    lastChunkTimeRef.current = null;\n    rateSamplesRef.current = [];\n\n    // NAPI-006: Deferred session creation - only create session on first message\n    // This prevents empty sessions from being persisted when user opens modal\n    // but doesn't send any messages\n    let activeSessionId = currentSessionId;\n    if (!activeSessionId && isFirstMessageRef.current) {\n      try {\n        const { persistenceCreateSessionWithProvider } = await import('@sengac/codelet-napi');\n        const project = currentProjectRef.current;\n        // Use first message as session name (truncated to 50 chars)\n        const sessionName = userMessage.slice(0, 50) + (userMessage.length > 50 ? '...' : '');\n\n        const persistedSession = persistenceCreateSessionWithProvider(\n          sessionName,\n          project,\n          currentProvider\n        );\n\n        activeSessionId = persistedSession.id;\n        setCurrentSessionId(activeSessionId);\n        // Mark first message as processed (session already named with message content)\n        isFirstMessageRef.current = false;\n      } catch {\n        // Session creation failed - continue without persistence\n      }\n    }\n\n    // NAPI-006: Save command to history\n    if (activeSessionId) {\n      try {\n        const { persistenceAddHistory } = await import('@sengac/codelet-napi');\n        persistenceAddHistory(userMessage, currentProjectRef.current, activeSessionId);\n        // Update local history entries\n        setHistoryEntries(prev => [{\n          display: userMessage,\n          timestamp: new Date().toISOString(),\n          project: currentProjectRef.current,\n          sessionId: activeSessionId,\n          hasPastedContent: false,\n        }, ...prev]);\n      } catch (err) {\n        logger.error(`Failed to save history: ${err instanceof Error ? err.message : String(err)}`);\n      }\n    } else {\n      logger.warn('No activeSessionId - history will not be saved');\n    }\n\n    // Add user message to conversation\n    setConversation(prev => [...prev, { role: 'user', content: userMessage }]);\n\n    // Persist user message as full envelope\n    if (activeSessionId) {\n      try {\n        const { persistenceStoreMessageEnvelope } = await import('@sengac/codelet-napi');\n\n        // Create proper user message envelope\n        // Note: \"type\" field matches Rust's #[serde(rename = \"type\")] for message_type\n        const userEnvelope = {\n          uuid: crypto.randomUUID(),\n          timestamp: new Date().toISOString(),\n          type: 'user',\n          provider: currentProvider,\n          message: {\n            role: 'user',\n            content: [{ type: 'text', text: userMessage }],\n          },\n        };\n        const envelopeJson = JSON.stringify(userEnvelope);\n        persistenceStoreMessageEnvelope(activeSessionId, envelopeJson);\n\n        // Note: Session naming now happens at creation time (deferred session creation above)\n        // so we don't need to rename here\n      } catch {\n        // User message persistence failed - continue\n      }\n    }\n\n    // Add streaming assistant message placeholder\n    setConversation(prev => [\n      ...prev,\n      { role: 'assistant', content: '', isStreaming: true },\n    ]);\n\n    try {\n      // Track current text segment (resets after tool calls)\n      let currentSegment = '';\n      // Track full assistant response for persistence (includes ALL content blocks)\n      let fullAssistantResponse = '';\n      // Track assistant message content blocks for envelope storage\n      const assistantContentBlocks: Array<{ type: string; text?: string; id?: string; name?: string; input?: unknown }> = [];\n      // Track pending tool results to store as user message\n      const pendingToolResults: Array<{ type: string; tool_use_id: string; content: string; is_error?: boolean }> = [];\n\n      await sessionRef.current.prompt(userMessage, (chunk: StreamChunk) => {\n        if (!chunk) return;\n\n        if (chunk.type === 'Text' && chunk.text) {\n          // TUI-031: Calculate tok/s on each text chunk\n          const now = Date.now();\n          const chunkTokens = Math.ceil(chunk.text.length / 4); // ~4 chars per token\n\n          if (lastChunkTimeRef.current !== null && chunkTokens > 0) {\n            const deltaTime = (now - lastChunkTimeRef.current) / 1000;\n            if (deltaTime > 0.01) { // At least 10ms between samples\n              const instantRate = chunkTokens / deltaTime;\n              // Add to samples, keep last N for smoothing\n              rateSamplesRef.current.push(instantRate);\n              if (rateSamplesRef.current.length > MAX_RATE_SAMPLES) {\n                rateSamplesRef.current.shift();\n              }\n              // Display average of samples\n              const avgRate = rateSamplesRef.current.reduce((a, b) => a + b, 0) / rateSamplesRef.current.length;\n              setDisplayedTokPerSec(avgRate);\n              setLastChunkTime(now);\n            }\n          }\n          lastChunkTimeRef.current = now;\n\n          // Text chunks are now batched in Rust, so we receive fewer, larger updates\n          currentSegment += chunk.text;\n          fullAssistantResponse += chunk.text; // Accumulate for display persistence\n          // Add to content blocks for envelope storage\n          const lastBlock = assistantContentBlocks[assistantContentBlocks.length - 1];\n          if (lastBlock && lastBlock.type === 'text') {\n            lastBlock.text = (lastBlock.text || '') + chunk.text;\n          } else {\n            assistantContentBlocks.push({ type: 'text', text: chunk.text });\n          }\n          const segmentSnapshot = currentSegment;\n          setConversation(prev => {\n            const updated = [...prev];\n            const streamingIdx = updated.findLastIndex(m => m.isStreaming);\n            if (streamingIdx >= 0) {\n              updated[streamingIdx] = {\n                ...updated[streamingIdx],\n                content: segmentSnapshot,\n              };\n            }\n            return updated;\n          });\n        } else if (chunk.type === 'ToolCall' && chunk.toolCall) {\n          // Finalize current streaming message and add tool call (match CLI format)\n          const toolCall = chunk.toolCall;\n\n          // Add tool_use block to content blocks for envelope storage\n          let parsedInput: unknown;\n          try {\n            parsedInput = JSON.parse(toolCall.input);\n          } catch {\n            parsedInput = toolCall.input;\n          }\n          assistantContentBlocks.push({\n            type: 'tool_use',\n            id: toolCall.id,\n            name: toolCall.name,\n            input: parsedInput,\n          });\n\n          let toolContent = `[Planning to use tool: ${toolCall.name}]`;\n          // Parse and display arguments\n          if (typeof parsedInput === 'object' && parsedInput !== null) {\n            for (const [key, value] of Object.entries(parsedInput as Record<string, unknown>)) {\n              const displayValue =\n                typeof value === 'string' ? value : JSON.stringify(value);\n              toolContent += `\\n  ${key}: ${displayValue}`;\n            }\n          } else if (toolCall.input) {\n            toolContent += `\\n  ${toolCall.input}`;\n          }\n          const toolContentSnapshot = toolContent;\n          setConversation(prev => {\n            const updated = [...prev];\n            const streamingIdx = updated.findLastIndex(m => m.isStreaming);\n            if (streamingIdx >= 0) {\n              // Mark current segment as complete\n              updated[streamingIdx] = {\n                ...updated[streamingIdx],\n                isStreaming: false,\n              };\n            }\n            // Add tool call message\n            updated.push({\n              role: 'tool',\n              content: toolContentSnapshot,\n            });\n            return updated;\n          });\n        } else if (chunk.type === 'ToolResult' && chunk.toolResult) {\n          // Show tool result in CLI format, then start new streaming message\n          const result = chunk.toolResult;\n\n          // Add tool_result to pending list (will be stored as user message)\n          pendingToolResults.push({\n            type: 'tool_result',\n            tool_use_id: result.toolCallId,\n            content: result.content,\n            is_error: result.isError,\n          });\n\n          // Sanitize content: replace tabs with spaces (Ink can't render tabs)\n          const sanitizedContent = result.content.replace(/\\t/g, '  ');\n          const preview = sanitizedContent.slice(0, 500);\n          const truncated = sanitizedContent.length > 500;\n          // Format like CLI: indented with separators\n          const indentedPreview = preview\n            .split('\\n')\n            .map(line => `  ${line}`)\n            .join('\\n');\n          const toolResultContent = `[Tool result preview]\\n-------\\n${indentedPreview}${truncated ? '...' : ''}\\n-------`;\n          currentSegment = ''; // Reset for next text segment\n          setConversation(prev => [\n            ...prev,\n            { role: 'tool' as const, content: toolResultContent },\n            // Add new streaming placeholder for AI continuation\n            { role: 'assistant' as const, content: '', isStreaming: true },\n          ]);\n        } else if (chunk.type === 'Done') {\n          // Mark streaming complete and remove empty trailing assistant messages\n          setConversation(prev => {\n            const updated = [...prev];\n            // Remove empty streaming assistant messages at the end\n            while (\n              updated.length > 0 &&\n              updated[updated.length - 1].role === 'assistant' &&\n              updated[updated.length - 1].isStreaming &&\n              !updated[updated.length - 1].content\n            ) {\n              updated.pop();\n            }\n            // Mark any remaining streaming message as complete\n            const lastAssistantIdx = updated.findLastIndex(\n              m => m.role === 'assistant' && m.isStreaming\n            );\n            if (lastAssistantIdx >= 0) {\n              updated[lastAssistantIdx] = {\n                ...updated[lastAssistantIdx],\n                isStreaming: false,\n              };\n            }\n            return updated;\n          });\n        } else if (chunk.type === 'Status' && chunk.status) {\n          const statusMessage = chunk.status;\n          // Status messages (e.g., compaction notifications)\n          setConversation(prev => [\n            ...prev,\n            {\n              role: 'tool',\n              content: statusMessage,\n            },\n          ]);\n        } else if (chunk.type === 'Interrupted') {\n          // Agent was interrupted by user\n          // Use ⚠ (U+26A0) without emoji selector - width 1 in both string-width and terminal\n          setConversation(prev => {\n            const updated = [\n              ...prev,\n              { role: 'tool' as const, content: '⚠ Agent interrupted' },\n            ];\n            // Mark any streaming message as complete\n            const lastAssistantIdx = updated.findLastIndex(\n              m => m.role === 'assistant' && m.isStreaming\n            );\n            if (lastAssistantIdx >= 0) {\n              updated[lastAssistantIdx] = {\n                ...updated[lastAssistantIdx],\n                isStreaming: false,\n              };\n            }\n            return updated;\n          });\n        } else if (chunk.type === 'TokenUpdate' && chunk.tokens) {\n          // Update token usage display (tok/s is now calculated from Text chunks)\n          setTokenUsage(chunk.tokens);\n        } else if (chunk.type === 'Error' && chunk.error) {\n          setError(chunk.error);\n        }\n      });\n\n      // Persist full envelopes to session (includes tool calls and results)\n      if (activeSessionId) {\n        try {\n          const { persistenceStoreMessageEnvelope } = await import('@sengac/codelet-napi');\n\n          // Store assistant message with ALL content blocks (text + tool_use)\n          // Note: \"type\" field matches Rust's #[serde(rename = \"type\")] for message_type\n          if (assistantContentBlocks.length > 0) {\n            const assistantEnvelope = {\n              uuid: crypto.randomUUID(),\n              timestamp: new Date().toISOString(),\n              type: 'assistant',\n              provider: currentProvider,\n              message: {\n                role: 'assistant',\n                content: assistantContentBlocks,\n              },\n            };\n            const assistantJson = JSON.stringify(assistantEnvelope);\n            persistenceStoreMessageEnvelope(activeSessionId, assistantJson);\n          }\n\n          // Store tool results as user message (if any)\n          if (pendingToolResults.length > 0) {\n            const toolResultEnvelope = {\n              uuid: crypto.randomUUID(),\n              timestamp: new Date().toISOString(),\n              type: 'user',\n              provider: currentProvider,\n              message: {\n                role: 'user',\n                content: pendingToolResults,\n              },\n            };\n            const toolResultJson = JSON.stringify(toolResultEnvelope);\n            persistenceStoreMessageEnvelope(activeSessionId, toolResultJson);\n          }\n        } catch {\n          // Message persistence failed - continue\n        }\n      }\n\n      // Update token usage after prompt completes (safe to access now - session unlocked)\n      if (sessionRef.current) {\n        const finalTokens = sessionRef.current.tokenTracker;\n        setTokenUsage(finalTokens);\n\n        // Persist token usage to session manifest (for restore)\n        if (activeSessionId) {\n          try {\n            const { persistenceSetSessionTokens } = await import('@sengac/codelet-napi');\n            persistenceSetSessionTokens(\n              activeSessionId,\n              finalTokens.inputTokens,\n              finalTokens.outputTokens,\n              0, // cache read - not tracked separately yet\n              0  // cache create - not tracked separately yet\n            );\n          } catch {\n            // Token usage persistence failed - continue\n          }\n        }\n      }\n    } catch (err) {\n      const errorMessage =\n        err instanceof Error ? err.message : 'Failed to send prompt';\n      setError(errorMessage);\n    } finally {\n      setIsLoading(false);\n    }\n  }"
    },
    {
      "type": "arrow_function",
      "line": 403,
      "column": 24,
      "text": "prev => [\n          ...prev,\n          { role: 'tool', content: result.message },\n        ]"
    },
    {
      "type": "arrow_function",
      "line": 446,
      "column": 26,
      "text": "prev => [\n            ...prev,\n            { role: 'tool', content: 'No history entries found' },\n          ]"
    },
    {
      "type": "arrow_function",
      "line": 451,
      "column": 42,
      "text": "(h: { display: string; timestamp: string }) =>\n            `- ${h.display}`"
    },
    {
      "type": "arrow_function",
      "line": 454,
      "column": 26,
      "text": "prev => [\n            ...prev,\n            { role: 'tool', content: `Command history:\\n${historyList}` },\n          ]"
    },
    {
      "type": "arrow_function",
      "line": 461,
      "column": 24,
      "text": "prev => [\n          ...prev,\n          { role: 'tool', content: `History failed: ${errorMessage}` },\n        ]"
    },
    {
      "type": "arrow_function",
      "line": 483,
      "column": 26,
      "text": "prev => [\n            ...prev,\n            { role: 'tool', content: 'No sessions found for this project' },\n          ]"
    },
    {
      "type": "arrow_function",
      "line": 488,
      "column": 43,
      "text": "(s: SessionManifest) =>\n            `- ${s.name} (${s.messageCount} messages, ${s.id.slice(0, 8)}...)`"
    },
    {
      "type": "arrow_function",
      "line": 491,
      "column": 26,
      "text": "prev => [\n            ...prev,\n            { role: 'tool', content: `Sessions:\\n${sessionList}` },\n          ]"
    },
    {
      "type": "arrow_function",
      "line": 498,
      "column": 24,
      "text": "prev => [\n          ...prev,\n          { role: 'tool', content: `List sessions failed: ${errorMessage}` },\n        ]"
    },
    {
      "type": "arrow_function",
      "line": 513,
      "column": 37,
      "text": "(s: SessionManifest) => s.name === targetName"
    },
    {
      "type": "arrow_function",
      "line": 516,
      "column": 26,
      "text": "prev => [\n            ...prev,\n            { role: 'tool', content: `Switched to session: \"${target.name}\"` },\n          ]"
    },
    {
      "type": "arrow_function",
      "line": 521,
      "column": 26,
      "text": "prev => [\n            ...prev,\n            { role: 'tool', content: `Session not found: \"${targetName}\"` },\n          ]"
    },
    {
      "type": "arrow_function",
      "line": 528,
      "column": 24,
      "text": "prev => [\n          ...prev,\n          { role: 'tool', content: `Switch failed: ${errorMessage}` },\n        ]"
    },
    {
      "type": "arrow_function",
      "line": 541,
      "column": 24,
      "text": "prev => [\n          ...prev,\n          { role: 'tool', content: 'No active session to rename' },\n        ]"
    },
    {
      "type": "arrow_function",
      "line": 550,
      "column": 24,
      "text": "prev => [\n          ...prev,\n          { role: 'tool', content: `Session renamed to: \"${newName}\"` },\n        ]"
    },
    {
      "type": "arrow_function",
      "line": 556,
      "column": 24,
      "text": "prev => [\n          ...prev,\n          { role: 'tool', content: `Rename failed: ${errorMessage}` },\n        ]"
    },
    {
      "type": "arrow_function",
      "line": 571,
      "column": 24,
      "text": "prev => [\n          ...prev,\n          { role: 'tool', content: 'No active session to fork' },\n        ]"
    },
    {
      "type": "arrow_function",
      "line": 578,
      "column": 24,
      "text": "prev => [\n          ...prev,\n          { role: 'tool', content: 'Usage: /fork <index> <name>' },\n        ]"
    },
    {
      "type": "arrow_function",
      "line": 588,
      "column": 24,
      "text": "prev => [\n          ...prev,\n          { role: 'tool', content: `Session forked at index ${index}: \"${name}\"` },\n        ]"
    },
    {
      "type": "arrow_function",
      "line": 594,
      "column": 24,
      "text": "prev => [\n          ...prev,\n          { role: 'tool', content: `Fork failed: ${errorMessage}` },\n        ]"
    },
    {
      "type": "arrow_function",
      "line": 609,
      "column": 24,
      "text": "prev => [\n          ...prev,\n          { role: 'tool', content: 'No active session to merge into' },\n        ]"
    },
    {
      "type": "arrow_function",
      "line": 616,
      "column": 24,
      "text": "prev => [\n          ...prev,\n          { role: 'tool', content: 'Usage: /merge <session-name> <indices> (e.g., /merge session-b 3,4)' },\n        ]"
    },
    {
      "type": "arrow_function",
      "line": 625,
      "column": 37,
      "text": "(s: SessionManifest) => s.name === sourceName || s.id === sourceName"
    },
    {
      "type": "arrow_function",
      "line": 627,
      "column": 26,
      "text": "prev => [\n            ...prev,\n            { role: 'tool', content: `Source session not found: \"${sourceName}\"` },\n          ]"
    },
    {
      "type": "arrow_function",
      "line": 633,
      "column": 50,
      "text": "(s: string) => parseInt(s.trim(), 10)"
    },
    {
      "type": "arrow_function",
      "line": 635,
      "column": 24,
      "text": "prev => [\n          ...prev,\n          { role: 'tool', content: `Merged ${indices.length} messages from \"${source.name}\"` },\n        ]"
    },
    {
      "type": "arrow_function",
      "line": 641,
      "column": 24,
      "text": "prev => [\n          ...prev,\n          { role: 'tool', content: `Merge failed: ${errorMessage}` },\n        ]"
    },
    {
      "type": "arrow_function",
      "line": 660,
      "column": 24,
      "text": "prev => [\n          ...prev,\n          { role: 'tool', content: 'No active session for cherry-pick' },\n        ]"
    },
    {
      "type": "arrow_function",
      "line": 667,
      "column": 24,
      "text": "prev => [\n          ...prev,\n          { role: 'tool', content: 'Usage: /cherry-pick <session> <index> [--context N]' },\n        ]"
    },
    {
      "type": "arrow_function",
      "line": 676,
      "column": 37,
      "text": "(s: SessionManifest) => s.name === sourceName || s.id === sourceName"
    },
    {
      "type": "arrow_function",
      "line": 678,
      "column": 26,
      "text": "prev => [\n            ...prev,\n            { role: 'tool', content: `Source session not found: \"${sourceName}\"` },\n          ]"
    },
    {
      "type": "arrow_function",
      "line": 685,
      "column": 24,
      "text": "prev => [\n          ...prev,\n          { role: 'tool', content: `Cherry-picked message ${index} with ${context} context messages from \"${source.name}\"` },\n        ]"
    },
    {
      "type": "arrow_function",
      "line": 691,
      "column": 24,
      "text": "prev => [\n          ...prev,\n          { role: 'tool', content: `Cherry-pick failed: ${errorMessage}` },\n        ]"
    },
    {
      "type": "arrow_function",
      "line": 705,
      "column": 24,
      "text": "prev => [\n          ...prev,\n          { role: 'tool', content: 'Nothing to compact - no messages yet' },\n        ]"
    },
    {
      "type": "arrow_function",
      "line": 713,
      "column": 22,
      "text": "prev => [\n        ...prev,\n        { role: 'tool', content: '[Compacting context...]' },\n      ]"
    },
    {
      "type": "arrow_function",
      "line": 723,
      "column": 24,
      "text": "prev => [\n          ...prev,\n          { role: 'tool', content: message },\n        ]"
    },
    {
      "type": "arrow_function",
      "line": 746,
      "column": 24,
      "text": "prev => [\n          ...prev,\n          { role: 'tool', content: `Compaction failed: ${errorMessage}` },\n        ]"
    },
    {
      "type": "arrow_function",
      "line": 797,
      "column": 26,
      "text": "prev => [{\n          display: userMessage,\n          timestamp: new Date().toISOString(),\n          project: currentProjectRef.current,\n          sessionId: activeSessionId,\n          hasPastedContent: false,\n        }, ...prev]"
    },
    {
      "type": "arrow_function",
      "line": 812,
      "column": 20,
      "text": "prev => [...prev, { role: 'user', content: userMessage }]"
    },
    {
      "type": "arrow_function",
      "line": 842,
      "column": 20,
      "text": "prev => [\n      ...prev,\n      { role: 'assistant', content: '', isStreaming: true },\n    ]"
    },
    {
      "type": "arrow_function",
      "line": 857,
      "column": 51,
      "text": "(chunk: StreamChunk) => {\n        if (!chunk) return;\n\n        if (chunk.type === 'Text' && chunk.text) {\n          // TUI-031: Calculate tok/s on each text chunk\n          const now = Date.now();\n          const chunkTokens = Math.ceil(chunk.text.length / 4); // ~4 chars per token\n\n          if (lastChunkTimeRef.current !== null && chunkTokens > 0) {\n            const deltaTime = (now - lastChunkTimeRef.current) / 1000;\n            if (deltaTime > 0.01) { // At least 10ms between samples\n              const instantRate = chunkTokens / deltaTime;\n              // Add to samples, keep last N for smoothing\n              rateSamplesRef.current.push(instantRate);\n              if (rateSamplesRef.current.length > MAX_RATE_SAMPLES) {\n                rateSamplesRef.current.shift();\n              }\n              // Display average of samples\n              const avgRate = rateSamplesRef.current.reduce((a, b) => a + b, 0) / rateSamplesRef.current.length;\n              setDisplayedTokPerSec(avgRate);\n              setLastChunkTime(now);\n            }\n          }\n          lastChunkTimeRef.current = now;\n\n          // Text chunks are now batched in Rust, so we receive fewer, larger updates\n          currentSegment += chunk.text;\n          fullAssistantResponse += chunk.text; // Accumulate for display persistence\n          // Add to content blocks for envelope storage\n          const lastBlock = assistantContentBlocks[assistantContentBlocks.length - 1];\n          if (lastBlock && lastBlock.type === 'text') {\n            lastBlock.text = (lastBlock.text || '') + chunk.text;\n          } else {\n            assistantContentBlocks.push({ type: 'text', text: chunk.text });\n          }\n          const segmentSnapshot = currentSegment;\n          setConversation(prev => {\n            const updated = [...prev];\n            const streamingIdx = updated.findLastIndex(m => m.isStreaming);\n            if (streamingIdx >= 0) {\n              updated[streamingIdx] = {\n                ...updated[streamingIdx],\n                content: segmentSnapshot,\n              };\n            }\n            return updated;\n          });\n        } else if (chunk.type === 'ToolCall' && chunk.toolCall) {\n          // Finalize current streaming message and add tool call (match CLI format)\n          const toolCall = chunk.toolCall;\n\n          // Add tool_use block to content blocks for envelope storage\n          let parsedInput: unknown;\n          try {\n            parsedInput = JSON.parse(toolCall.input);\n          } catch {\n            parsedInput = toolCall.input;\n          }\n          assistantContentBlocks.push({\n            type: 'tool_use',\n            id: toolCall.id,\n            name: toolCall.name,\n            input: parsedInput,\n          });\n\n          let toolContent = `[Planning to use tool: ${toolCall.name}]`;\n          // Parse and display arguments\n          if (typeof parsedInput === 'object' && parsedInput !== null) {\n            for (const [key, value] of Object.entries(parsedInput as Record<string, unknown>)) {\n              const displayValue =\n                typeof value === 'string' ? value : JSON.stringify(value);\n              toolContent += `\\n  ${key}: ${displayValue}`;\n            }\n          } else if (toolCall.input) {\n            toolContent += `\\n  ${toolCall.input}`;\n          }\n          const toolContentSnapshot = toolContent;\n          setConversation(prev => {\n            const updated = [...prev];\n            const streamingIdx = updated.findLastIndex(m => m.isStreaming);\n            if (streamingIdx >= 0) {\n              // Mark current segment as complete\n              updated[streamingIdx] = {\n                ...updated[streamingIdx],\n                isStreaming: false,\n              };\n            }\n            // Add tool call message\n            updated.push({\n              role: 'tool',\n              content: toolContentSnapshot,\n            });\n            return updated;\n          });\n        } else if (chunk.type === 'ToolResult' && chunk.toolResult) {\n          // Show tool result in CLI format, then start new streaming message\n          const result = chunk.toolResult;\n\n          // Add tool_result to pending list (will be stored as user message)\n          pendingToolResults.push({\n            type: 'tool_result',\n            tool_use_id: result.toolCallId,\n            content: result.content,\n            is_error: result.isError,\n          });\n\n          // Sanitize content: replace tabs with spaces (Ink can't render tabs)\n          const sanitizedContent = result.content.replace(/\\t/g, '  ');\n          const preview = sanitizedContent.slice(0, 500);\n          const truncated = sanitizedContent.length > 500;\n          // Format like CLI: indented with separators\n          const indentedPreview = preview\n            .split('\\n')\n            .map(line => `  ${line}`)\n            .join('\\n');\n          const toolResultContent = `[Tool result preview]\\n-------\\n${indentedPreview}${truncated ? '...' : ''}\\n-------`;\n          currentSegment = ''; // Reset for next text segment\n          setConversation(prev => [\n            ...prev,\n            { role: 'tool' as const, content: toolResultContent },\n            // Add new streaming placeholder for AI continuation\n            { role: 'assistant' as const, content: '', isStreaming: true },\n          ]);\n        } else if (chunk.type === 'Done') {\n          // Mark streaming complete and remove empty trailing assistant messages\n          setConversation(prev => {\n            const updated = [...prev];\n            // Remove empty streaming assistant messages at the end\n            while (\n              updated.length > 0 &&\n              updated[updated.length - 1].role === 'assistant' &&\n              updated[updated.length - 1].isStreaming &&\n              !updated[updated.length - 1].content\n            ) {\n              updated.pop();\n            }\n            // Mark any remaining streaming message as complete\n            const lastAssistantIdx = updated.findLastIndex(\n              m => m.role === 'assistant' && m.isStreaming\n            );\n            if (lastAssistantIdx >= 0) {\n              updated[lastAssistantIdx] = {\n                ...updated[lastAssistantIdx],\n                isStreaming: false,\n              };\n            }\n            return updated;\n          });\n        } else if (chunk.type === 'Status' && chunk.status) {\n          const statusMessage = chunk.status;\n          // Status messages (e.g., compaction notifications)\n          setConversation(prev => [\n            ...prev,\n            {\n              role: 'tool',\n              content: statusMessage,\n            },\n          ]);\n        } else if (chunk.type === 'Interrupted') {\n          // Agent was interrupted by user\n          // Use ⚠ (U+26A0) without emoji selector - width 1 in both string-width and terminal\n          setConversation(prev => {\n            const updated = [\n              ...prev,\n              { role: 'tool' as const, content: '⚠ Agent interrupted' },\n            ];\n            // Mark any streaming message as complete\n            const lastAssistantIdx = updated.findLastIndex(\n              m => m.role === 'assistant' && m.isStreaming\n            );\n            if (lastAssistantIdx >= 0) {\n              updated[lastAssistantIdx] = {\n                ...updated[lastAssistantIdx],\n                isStreaming: false,\n              };\n            }\n            return updated;\n          });\n        } else if (chunk.type === 'TokenUpdate' && chunk.tokens) {\n          // Update token usage display (tok/s is now calculated from Text chunks)\n          setTokenUsage(chunk.tokens);\n        } else if (chunk.type === 'Error' && chunk.error) {\n          setError(chunk.error);\n        }\n      }"
    },
    {
      "type": "arrow_function",
      "line": 875,
      "column": 60,
      "text": "(a, b) => a + b"
    },
    {
      "type": "arrow_function",
      "line": 893,
      "column": 26,
      "text": "prev => {\n            const updated = [...prev];\n            const streamingIdx = updated.findLastIndex(m => m.isStreaming);\n            if (streamingIdx >= 0) {\n              updated[streamingIdx] = {\n                ...updated[streamingIdx],\n                content: segmentSnapshot,\n              };\n            }\n            return updated;\n          }"
    },
    {
      "type": "arrow_function",
      "line": 895,
      "column": 55,
      "text": "m => m.isStreaming"
    },
    {
      "type": "arrow_function",
      "line": 934,
      "column": 26,
      "text": "prev => {\n            const updated = [...prev];\n            const streamingIdx = updated.findLastIndex(m => m.isStreaming);\n            if (streamingIdx >= 0) {\n              // Mark current segment as complete\n              updated[streamingIdx] = {\n                ...updated[streamingIdx],\n                isStreaming: false,\n              };\n            }\n            // Add tool call message\n            updated.push({\n              role: 'tool',\n              content: toolContentSnapshot,\n            });\n            return updated;\n          }"
    },
    {
      "type": "arrow_function",
      "line": 936,
      "column": 55,
      "text": "m => m.isStreaming"
    },
    {
      "type": "arrow_function",
      "line": 970,
      "column": 17,
      "text": "line => `  ${line}`"
    },
    {
      "type": "arrow_function",
      "line": 974,
      "column": 26,
      "text": "prev => [\n            ...prev,\n            { role: 'tool' as const, content: toolResultContent },\n            // Add new streaming placeholder for AI continuation\n            { role: 'assistant' as const, content: '', isStreaming: true },\n          ]"
    },
    {
      "type": "arrow_function",
      "line": 982,
      "column": 26,
      "text": "prev => {\n            const updated = [...prev];\n            // Remove empty streaming assistant messages at the end\n            while (\n              updated.length > 0 &&\n              updated[updated.length - 1].role === 'assistant' &&\n              updated[updated.length - 1].isStreaming &&\n              !updated[updated.length - 1].content\n            ) {\n              updated.pop();\n            }\n            // Mark any remaining streaming message as complete\n            const lastAssistantIdx = updated.findLastIndex(\n              m => m.role === 'assistant' && m.isStreaming\n            );\n            if (lastAssistantIdx >= 0) {\n              updated[lastAssistantIdx] = {\n                ...updated[lastAssistantIdx],\n                isStreaming: false,\n              };\n            }\n            return updated;\n          }"
    },
    {
      "type": "arrow_function",
      "line": 995,
      "column": 14,
      "text": "m => m.role === 'assistant' && m.isStreaming"
    },
    {
      "type": "arrow_function",
      "line": 1008,
      "column": 26,
      "text": "prev => [\n            ...prev,\n            {\n              role: 'tool',\n              content: statusMessage,\n            },\n          ]"
    },
    {
      "type": "arrow_function",
      "line": 1018,
      "column": 26,
      "text": "prev => {\n            const updated = [\n              ...prev,\n              { role: 'tool' as const, content: '⚠ Agent interrupted' },\n            ];\n            // Mark any streaming message as complete\n            const lastAssistantIdx = updated.findLastIndex(\n              m => m.role === 'assistant' && m.isStreaming\n            );\n            if (lastAssistantIdx >= 0) {\n              updated[lastAssistantIdx] = {\n                ...updated[lastAssistantIdx],\n                isStreaming: false,\n              };\n            }\n            return updated;\n          }"
    },
    {
      "type": "arrow_function",
      "line": 1025,
      "column": 14,
      "text": "m => m.role === 'assistant' && m.isStreaming"
    },
    {
      "type": "arrow_function",
      "line": 1116,
      "column": 43,
      "text": "async (providerName: string) => {\n    if (!sessionRef.current) return;\n\n    try {\n      setIsLoading(true);\n      await sessionRef.current.switchProvider(providerName);\n      setCurrentProvider(providerName);\n      setShowProviderSelector(false);\n    } catch (err) {\n      const errorMessage =\n        err instanceof Error ? err.message : 'Failed to switch provider';\n      setError(errorMessage);\n    } finally {\n      setIsLoading(false);\n    }\n  }"
    },
    {
      "type": "arrow_function",
      "line": 1134,
      "column": 40,
      "text": "() => {\n    if (historyEntries.length === 0) {\n      return;\n    }\n\n    // Save current input if we're starting navigation\n    if (historyIndex === -1) {\n      setSavedInput(inputValue);\n    }\n\n    const newIndex = historyIndex === -1 ? 0 : Math.min(historyIndex + 1, historyEntries.length - 1);\n    setHistoryIndex(newIndex);\n    setInputValue(historyEntries[newIndex].display);\n  }"
    },
    {
      "type": "arrow_function",
      "line": 1150,
      "column": 40,
      "text": "() => {\n    if (historyIndex === -1) return;\n\n    if (historyIndex === 0) {\n      // Return to saved input\n      setHistoryIndex(-1);\n      setInputValue(savedInput);\n    } else {\n      const newIndex = historyIndex - 1;\n      setHistoryIndex(newIndex);\n      setInputValue(historyEntries[newIndex].display);\n    }\n  }"
    },
    {
      "type": "arrow_function",
      "line": 1165,
      "column": 39,
      "text": "() => {\n    setIsSearchMode(true);\n    setSearchQuery('');\n    setSearchResults([]);\n    setSearchResultIndex(0);\n  }"
    },
    {
      "type": "arrow_function",
      "line": 1173,
      "column": 40,
      "text": "async (query: string) => {\n    setSearchQuery(query);\n    if (!query.trim()) {\n      setSearchResults([]);\n      return;\n    }\n\n    try {\n      const { persistenceSearchHistory } = await import('@sengac/codelet-napi');\n      const results = persistenceSearchHistory(query, currentProjectRef.current);\n      const entries: HistoryEntry[] = results.map((h: { display: string; timestamp: string; project: string; sessionId: string; hasPastedContent?: boolean }) => ({\n        display: h.display,\n        timestamp: h.timestamp,\n        project: h.project,\n        sessionId: h.sessionId,\n        hasPastedContent: h.hasPastedContent ?? false,\n      }));\n      setSearchResults(entries);\n      setSearchResultIndex(0);\n    } catch {\n      // Search is optional - continue without it\n    }\n  }"
    },
    {
      "type": "arrow_function",
      "line": 1183,
      "column": 50,
      "text": "(h: { display: string; timestamp: string; project: string; sessionId: string; hasPastedContent?: boolean }) => ({\n        display: h.display,\n        timestamp: h.timestamp,\n        project: h.project,\n        sessionId: h.sessionId,\n        hasPastedContent: h.hasPastedContent ?? false,\n      })"
    },
    {
      "type": "arrow_function",
      "line": 1198,
      "column": 41,
      "text": "() => {\n    if (searchResults.length > 0 && searchResultIndex < searchResults.length) {\n      setInputValue(searchResults[searchResultIndex].display);\n    }\n    setIsSearchMode(false);\n    setSearchQuery('');\n    setSearchResults([]);\n  }"
    },
    {
      "type": "arrow_function",
      "line": 1208,
      "column": 41,
      "text": "() => {\n    setIsSearchMode(false);\n    setSearchQuery('');\n    setSearchResults([]);\n  }"
    },
    {
      "type": "arrow_function",
      "line": 1215,
      "column": 36,
      "text": "(date: Date): string => {\n    const now = new Date();\n    const diffMs = now.getTime() - date.getTime();\n    const diffMins = Math.floor(diffMs / 60000);\n    const diffHours = Math.floor(diffMs / 3600000);\n    const diffDays = Math.floor(diffMs / 86400000);\n\n    // Format time as HH:MM\n    const timeStr = date.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: false });\n\n    if (diffMins < 1) return 'just now';\n    if (diffMins < 60) return `${diffMins}m ago`;\n    if (diffHours < 24) return `${diffHours}h ago`;\n    if (diffDays === 1) return `yesterday ${timeStr}`;\n    if (diffDays < 7) {\n      const dayName = date.toLocaleDateString('en-US', { weekday: 'short' });\n      return `${dayName} ${timeStr}`;\n    }\n    // For older sessions, show date and time\n    const monthDay = date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });\n    return `${monthDay} ${timeStr}`;\n  }"
    },
    {
      "type": "arrow_function",
      "line": 1239,
      "column": 39,
      "text": "async () => {\n    try {\n      const { persistenceListSessions } = await import('@sengac/codelet-napi');\n      const sessions = persistenceListSessions(currentProjectRef.current);\n\n      // Sort by updatedAt descending (most recent first)\n      const sorted = [...sessions].sort((a: SessionManifest, b: SessionManifest) =>\n        new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()\n      );\n\n      setAvailableSessions(sorted);\n      setResumeSessionIndex(0);\n      setIsResumeMode(true);\n    } catch (err) {\n      const errorMessage = err instanceof Error ? err.message : 'Failed to list sessions';\n      setConversation(prev => [\n        ...prev,\n        { role: 'tool', content: `Resume failed: ${errorMessage}` },\n      ]);\n    }\n  }"
    },
    {
      "type": "arrow_function",
      "line": 1245,
      "column": 40,
      "text": "(a: SessionManifest, b: SessionManifest) =>\n        new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()"
    },
    {
      "type": "arrow_function",
      "line": 1254,
      "column": 22,
      "text": "prev => [\n        ...prev,\n        { role: 'tool', content: `Resume failed: ${errorMessage}` },\n      ]"
    },
    {
      "type": "arrow_function",
      "line": 1262,
      "column": 41,
      "text": "async () => {\n    if (availableSessions.length === 0 || resumeSessionIndex >= availableSessions.length) {\n      return;\n    }\n\n    const selectedSession = availableSessions[resumeSessionIndex];\n\n    try {\n      const { persistenceGetSessionMessages, persistenceGetSessionMessageEnvelopes } = await import('@sengac/codelet-napi');\n      const messages = persistenceGetSessionMessages(selectedSession.id);\n\n      // CRITICAL: Restore messages to CodeletSession for LLM context\n      // Without this, the AI would have no context of the restored conversation\n      // even though the UI shows historical messages\n      if (sessionRef.current) {\n        sessionRef.current.restoreMessages(messages);\n      }\n\n      // Get FULL envelopes with all content blocks (ToolUse, ToolResult, Text, etc.)\n      const envelopes: string[] = persistenceGetSessionMessageEnvelopes(selectedSession.id);\n\n      // Convert full envelopes to conversation format for UI display\n      // This properly restores tool calls, tool results, thinking, etc.\n      //\n      // CRITICAL: Tool results are stored in separate user envelopes after assistant\n      // messages, but we need to interleave them correctly by matching tool_use_id.\n      const restored: ConversationMessage[] = [];\n\n      // First pass: collect all tool results by their tool_use_id\n      const toolResultsByUseId = new Map<string, { content: string; isError: boolean }>();\n      for (const envelopeJson of envelopes) {\n        try {\n          const envelope = JSON.parse(envelopeJson);\n          const messageType = envelope.type || envelope.message_type || envelope.messageType;\n          const message = envelope.message;\n          if (!message) continue;\n\n          if (messageType === 'user') {\n            const contents = message.content || [];\n            for (const content of contents) {\n              if (content.type === 'tool_result' && content.tool_use_id) {\n                toolResultsByUseId.set(content.tool_use_id, {\n                  content: content.content || '',\n                  isError: content.is_error || false,\n                });\n              }\n            }\n          }\n        } catch {\n          // Skip malformed envelopes in first pass\n        }\n      }\n\n      // Second pass: process envelopes and interleave tool results\n      for (const envelopeJson of envelopes) {\n        try {\n          const envelope = JSON.parse(envelopeJson);\n          const messageType = envelope.type || envelope.message_type || envelope.messageType;\n          const message = envelope.message;\n\n          if (!message) continue;\n\n          if (messageType === 'user') {\n            // User messages - extract text only (tool results handled via interleaving)\n            const contents = message.content || [];\n            for (const content of contents) {\n              if (content.type === 'text' && content.text) {\n                restored.push({ role: 'user', content: `${content.text}`, isStreaming: false });\n              }\n              // Skip tool_result here - they're interleaved with tool_use below\n            }\n          } else if (messageType === 'assistant') {\n            // Assistant messages - extract text, tool use, and thinking\n            // Interleave tool results immediately after their corresponding tool_use\n            const contents = message.content || [];\n            let textContent = '';\n\n            for (const content of contents) {\n              if (content.type === 'text' && content.text) {\n                textContent += content.text;\n              } else if (content.type === 'tool_use') {\n                // Flush accumulated text first\n                if (textContent) {\n                  restored.push({ role: 'assistant', content: textContent, isStreaming: false });\n                  textContent = '';\n                }\n                // Tool call\n                let toolContent = `[Planning to use tool: ${content.name}]`;\n                const input = content.input;\n                if (typeof input === 'object' && input !== null) {\n                  for (const [key, value] of Object.entries(input)) {\n                    const displayValue = typeof value === 'string' ? value : JSON.stringify(value);\n                    toolContent += `\\n  ${key}: ${displayValue}`;\n                  }\n                }\n                restored.push({ role: 'tool', content: toolContent, isStreaming: false });\n\n                // Immediately show the tool result (interleaved)\n                const toolResult = toolResultsByUseId.get(content.id);\n                if (toolResult) {\n                  const preview = toolResult.content.slice(0, 500);\n                  const truncated = toolResult.content.length > 500;\n                  const indentedPreview = preview.split('\\n').map((line: string) => `  ${line}`).join('\\n');\n                  restored.push({\n                    role: 'tool',\n                    content: `[Tool result preview]\\n-------\\n${indentedPreview}${truncated ? '...' : ''}\\n-------`,\n                    isStreaming: false,\n                  });\n                }\n              } else if (content.type === 'thinking' && content.thinking) {\n                // Thinking block (could show or hide based on preference)\n                // For now, skip thinking blocks in restore (like Claude Code does)\n              }\n            }\n\n            // Flush remaining text\n            if (textContent) {\n              restored.push({ role: 'assistant', content: textContent, isStreaming: false });\n            }\n          }\n        } catch {\n          // If envelope parsing fails, fall back to simple format\n          logger.warn('Failed to parse envelope, falling back to simple format');\n        }\n      }\n\n      // If envelope parsing yielded nothing, fall back to simple messages\n      if (restored.length === 0) {\n        for (const m of messages) {\n          restored.push({\n            role: m.role === 'user' ? 'user' : 'assistant',\n            content: m.content,\n            isStreaming: false,\n          });\n        }\n      }\n\n      // Update state - replace current conversation entirely\n      setCurrentSessionId(selectedSession.id);\n      setConversation(restored);\n      setIsResumeMode(false);\n      setAvailableSessions([]);\n      setResumeSessionIndex(0);\n      // Don't rename resumed sessions with their first new message\n      isFirstMessageRef.current = false;\n\n      // Restore token usage from session manifest\n      if (selectedSession.tokenUsage) {\n        setTokenUsage({\n          inputTokens: selectedSession.tokenUsage.totalInputTokens,\n          outputTokens: selectedSession.tokenUsage.totalOutputTokens,\n        });\n      }\n\n      // Build confirmation message with compaction info\n      let confirmationMsg = `Session resumed: \"${selectedSession.name}\" (${selectedSession.messageCount} messages)`;\n      if (selectedSession.compaction) {\n        confirmationMsg += `\\n[Compaction: ${selectedSession.compaction.summary}]`;\n      }\n\n      // Add confirmation message\n      setConversation(prev => [\n        ...prev,\n        { role: 'tool', content: confirmationMsg },\n      ]);\n    } catch (err) {\n      const errorMessage = err instanceof Error ? err.message : 'Failed to restore session';\n      setConversation(prev => [\n        ...prev,\n        { role: 'tool', content: `Resume failed: ${errorMessage}` },\n      ]);\n      setIsResumeMode(false);\n      setAvailableSessions([]);\n      setResumeSessionIndex(0);\n    }\n  }"
    },
    {
      "type": "arrow_function",
      "line": 1364,
      "column": 66,
      "text": "(line: string) => `  ${line}`"
    },
    {
      "type": "arrow_function",
      "line": 1423,
      "column": 22,
      "text": "prev => [\n        ...prev,\n        { role: 'tool', content: confirmationMsg },\n      ]"
    },
    {
      "type": "arrow_function",
      "line": 1429,
      "column": 22,
      "text": "prev => [\n        ...prev,\n        { role: 'tool', content: `Resume failed: ${errorMessage}` },\n      ]"
    },
    {
      "type": "arrow_function",
      "line": 1440,
      "column": 41,
      "text": "() => {\n    setIsResumeMode(false);\n    setAvailableSessions([]);\n    setResumeSessionIndex(0);\n  }"
    },
    {
      "type": "arrow_function",
      "line": 1448,
      "column": 4,
      "text": "(input, key) => {\n      // Skip mouse events (handled by VirtualList)\n      if (input.startsWith('[M') || key.mouse) {\n        return;\n      }\n\n      // NAPI-006: Search mode keyboard handling\n      if (isSearchMode) {\n        if (key.escape) {\n          handleSearchCancel();\n          return;\n        }\n        if (key.return) {\n          handleSearchSelect();\n          return;\n        }\n        if (key.upArrow) {\n          setSearchResultIndex(prev => Math.max(0, prev - 1));\n          return;\n        }\n        if (key.downArrow) {\n          setSearchResultIndex(prev => Math.min(searchResults.length - 1, prev + 1));\n          return;\n        }\n        if (key.backspace || key.delete) {\n          void handleSearchInput(searchQuery.slice(0, -1));\n          return;\n        }\n        // Accept printable characters for search query\n        const clean = input\n          .split('')\n          .filter((ch) => {\n            const code = ch.charCodeAt(0);\n            return code >= 32 && code <= 126;\n          })\n          .join('');\n        if (clean) {\n          void handleSearchInput(searchQuery + clean);\n        }\n        return;\n      }\n\n      // NAPI-003: Resume mode keyboard handling\n      if (isResumeMode) {\n        if (key.escape) {\n          handleResumeCancel();\n          return;\n        }\n        if (key.return) {\n          void handleResumeSelect();\n          return;\n        }\n        if (key.upArrow) {\n          setResumeSessionIndex(prev => Math.max(0, prev - 1));\n          return;\n        }\n        if (key.downArrow) {\n          setResumeSessionIndex(prev => Math.min(availableSessions.length - 1, prev + 1));\n          return;\n        }\n        // No text input in resume mode - just navigation\n        return;\n      }\n\n      if (showProviderSelector) {\n        if (key.escape) {\n          setShowProviderSelector(false);\n          return;\n        }\n        if (key.upArrow) {\n          setSelectedProviderIndex(prev =>\n            prev > 0 ? prev - 1 : availableProviders.length - 1\n          );\n          return;\n        }\n        if (key.downArrow) {\n          setSelectedProviderIndex(prev =>\n            prev < availableProviders.length - 1 ? prev + 1 : 0\n          );\n          return;\n        }\n        if (key.return) {\n          void handleSwitchProvider(availableProviders[selectedProviderIndex]);\n          return;\n        }\n        return;\n      }\n\n      // Esc key handling - interrupt if loading, close if not\n      if (key.escape) {\n        if (isLoading && sessionRef.current) {\n          // Interrupt the agent execution\n          sessionRef.current.interrupt();\n        } else {\n          // Close the modal\n          onClose();\n        }\n        return;\n      }\n\n      // Tab to toggle provider selector\n      if (key.tab && availableProviders.length > 1) {\n        setShowProviderSelector(true);\n        const idx = availableProviders.indexOf(currentProvider);\n        setSelectedProviderIndex(idx >= 0 ? idx : 0);\n        return;\n      }\n    }"
    },
    {
      "type": "arrow_function",
      "line": 1465,
      "column": 31,
      "text": "prev => Math.max(0, prev - 1)"
    },
    {
      "type": "arrow_function",
      "line": 1469,
      "column": 31,
      "text": "prev => Math.min(searchResults.length - 1, prev + 1)"
    },
    {
      "type": "arrow_function",
      "line": 1479,
      "column": 18,
      "text": "(ch) => {\n            const code = ch.charCodeAt(0);\n            return code >= 32 && code <= 126;\n          }"
    },
    {
      "type": "arrow_function",
      "line": 1501,
      "column": 32,
      "text": "prev => Math.max(0, prev - 1)"
    },
    {
      "type": "arrow_function",
      "line": 1505,
      "column": 32,
      "text": "prev => Math.min(availableSessions.length - 1, prev + 1)"
    },
    {
      "type": "arrow_function",
      "line": 1518,
      "column": 35,
      "text": "prev =>\n            prev > 0 ? prev - 1 : availableProviders.length - 1"
    },
    {
      "type": "arrow_function",
      "line": 1524,
      "column": 35,
      "text": "prev =>\n            prev < availableProviders.length - 1 ? prev + 1 : 0"
    },
    {
      "type": "arrow_function",
      "line": 1563,
      "column": 36,
      "text": "(): ConversationLine[] => {\n    const maxWidth = terminalWidth - 6; // Account for borders and padding\n    const lines: ConversationLine[] = [];\n\n    conversation.forEach((msg, msgIndex) => {\n      // Add role prefix to first line\n      const prefix =\n        msg.role === 'user' ? 'You: ' : msg.role === 'assistant' ? 'AI: ' : '';\n      // Normalize emoji variation selectors for consistent width calculation\n      const normalizedContent = normalizeEmojiWidth(msg.content);\n      const contentLines = normalizedContent.split('\\n');\n\n      contentLines.forEach((lineContent, lineIndex) => {\n        let displayContent =\n          lineIndex === 0 ? `${prefix}${lineContent}` : lineContent;\n        // Add streaming indicator to last line of streaming message\n        const isLastLine = lineIndex === contentLines.length - 1;\n        if (msg.isStreaming && isLastLine) {\n          displayContent += '...';\n        }\n\n        // Wrap long lines manually to fit terminal width (using visual width for Unicode)\n        if (getVisualWidth(displayContent) === 0) {\n          lines.push({ role: msg.role, content: ' ', messageIndex: msgIndex });\n        } else {\n          // Split into words, keeping whitespace\n          const words = displayContent.split(/(\\s+)/);\n          let currentLine = '';\n          let currentWidth = 0;\n\n          for (const word of words) {\n            const wordWidth = getVisualWidth(word);\n\n            if (wordWidth === 0) continue;\n\n            // If word alone exceeds max width, force break it character by character\n            if (wordWidth > maxWidth) {\n              // Flush current line first\n              if (currentLine) {\n                lines.push({ role: msg.role, content: currentLine, messageIndex: msgIndex });\n                currentLine = '';\n                currentWidth = 0;\n              }\n              // Break long word by visual width\n              let chunk = '';\n              let chunkWidth = 0;\n              for (const char of word) {\n                const charWidth = getVisualWidth(char);\n                if (chunkWidth + charWidth > maxWidth && chunk) {\n                  lines.push({ role: msg.role, content: chunk, messageIndex: msgIndex });\n                  chunk = char;\n                  chunkWidth = charWidth;\n                } else {\n                  chunk += char;\n                  chunkWidth += charWidth;\n                }\n              }\n              if (chunk) {\n                currentLine = chunk;\n                currentWidth = chunkWidth;\n              }\n              continue;\n            }\n\n            // Check if word fits on current line\n            if (currentWidth + wordWidth > maxWidth) {\n              // Flush current line and start new one\n              if (currentLine.trim()) {\n                lines.push({ role: msg.role, content: currentLine.trimEnd(), messageIndex: msgIndex });\n              }\n              // Don't start line with whitespace\n              currentLine = word.trim() ? word : '';\n              currentWidth = word.trim() ? wordWidth : 0;\n            } else {\n              currentLine += word;\n              currentWidth += wordWidth;\n            }\n          }\n\n          // Flush remaining content\n          if (currentLine.trim()) {\n            lines.push({ role: msg.role, content: currentLine.trimEnd(), messageIndex: msgIndex });\n          } else if (lines.length === 0 || lines[lines.length - 1]?.messageIndex !== msgIndex) {\n            // Ensure at least one line per content section\n            lines.push({ role: msg.role, content: ' ', messageIndex: msgIndex });\n          }\n        }\n      });\n\n      // Add empty line after each message for spacing (use space to ensure line renders)\n      lines.push({ role: msg.role, content: ' ', messageIndex: msgIndex });\n    });\n\n    return lines;\n  }"
    },
    {
      "type": "arrow_function",
      "line": 1567,
      "column": 25,
      "text": "(msg, msgIndex) => {\n      // Add role prefix to first line\n      const prefix =\n        msg.role === 'user' ? 'You: ' : msg.role === 'assistant' ? 'AI: ' : '';\n      // Normalize emoji variation selectors for consistent width calculation\n      const normalizedContent = normalizeEmojiWidth(msg.content);\n      const contentLines = normalizedContent.split('\\n');\n\n      contentLines.forEach((lineContent, lineIndex) => {\n        let displayContent =\n          lineIndex === 0 ? `${prefix}${lineContent}` : lineContent;\n        // Add streaming indicator to last line of streaming message\n        const isLastLine = lineIndex === contentLines.length - 1;\n        if (msg.isStreaming && isLastLine) {\n          displayContent += '...';\n        }\n\n        // Wrap long lines manually to fit terminal width (using visual width for Unicode)\n        if (getVisualWidth(displayContent) === 0) {\n          lines.push({ role: msg.role, content: ' ', messageIndex: msgIndex });\n        } else {\n          // Split into words, keeping whitespace\n          const words = displayContent.split(/(\\s+)/);\n          let currentLine = '';\n          let currentWidth = 0;\n\n          for (const word of words) {\n            const wordWidth = getVisualWidth(word);\n\n            if (wordWidth === 0) continue;\n\n            // If word alone exceeds max width, force break it character by character\n            if (wordWidth > maxWidth) {\n              // Flush current line first\n              if (currentLine) {\n                lines.push({ role: msg.role, content: currentLine, messageIndex: msgIndex });\n                currentLine = '';\n                currentWidth = 0;\n              }\n              // Break long word by visual width\n              let chunk = '';\n              let chunkWidth = 0;\n              for (const char of word) {\n                const charWidth = getVisualWidth(char);\n                if (chunkWidth + charWidth > maxWidth && chunk) {\n                  lines.push({ role: msg.role, content: chunk, messageIndex: msgIndex });\n                  chunk = char;\n                  chunkWidth = charWidth;\n                } else {\n                  chunk += char;\n                  chunkWidth += charWidth;\n                }\n              }\n              if (chunk) {\n                currentLine = chunk;\n                currentWidth = chunkWidth;\n              }\n              continue;\n            }\n\n            // Check if word fits on current line\n            if (currentWidth + wordWidth > maxWidth) {\n              // Flush current line and start new one\n              if (currentLine.trim()) {\n                lines.push({ role: msg.role, content: currentLine.trimEnd(), messageIndex: msgIndex });\n              }\n              // Don't start line with whitespace\n              currentLine = word.trim() ? word : '';\n              currentWidth = word.trim() ? wordWidth : 0;\n            } else {\n              currentLine += word;\n              currentWidth += wordWidth;\n            }\n          }\n\n          // Flush remaining content\n          if (currentLine.trim()) {\n            lines.push({ role: msg.role, content: currentLine.trimEnd(), messageIndex: msgIndex });\n          } else if (lines.length === 0 || lines[lines.length - 1]?.messageIndex !== msgIndex) {\n            // Ensure at least one line per content section\n            lines.push({ role: msg.role, content: ' ', messageIndex: msgIndex });\n          }\n        }\n      });\n\n      // Add empty line after each message for spacing (use space to ensure line renders)\n      lines.push({ role: msg.role, content: ' ', messageIndex: msgIndex });\n    }"
    },
    {
      "type": "arrow_function",
      "line": 1575,
      "column": 27,
      "text": "(lineContent, lineIndex) => {\n        let displayContent =\n          lineIndex === 0 ? `${prefix}${lineContent}` : lineContent;\n        // Add streaming indicator to last line of streaming message\n        const isLastLine = lineIndex === contentLines.length - 1;\n        if (msg.isStreaming && isLastLine) {\n          displayContent += '...';\n        }\n\n        // Wrap long lines manually to fit terminal width (using visual width for Unicode)\n        if (getVisualWidth(displayContent) === 0) {\n          lines.push({ role: msg.role, content: ' ', messageIndex: msgIndex });\n        } else {\n          // Split into words, keeping whitespace\n          const words = displayContent.split(/(\\s+)/);\n          let currentLine = '';\n          let currentWidth = 0;\n\n          for (const word of words) {\n            const wordWidth = getVisualWidth(word);\n\n            if (wordWidth === 0) continue;\n\n            // If word alone exceeds max width, force break it character by character\n            if (wordWidth > maxWidth) {\n              // Flush current line first\n              if (currentLine) {\n                lines.push({ role: msg.role, content: currentLine, messageIndex: msgIndex });\n                currentLine = '';\n                currentWidth = 0;\n              }\n              // Break long word by visual width\n              let chunk = '';\n              let chunkWidth = 0;\n              for (const char of word) {\n                const charWidth = getVisualWidth(char);\n                if (chunkWidth + charWidth > maxWidth && chunk) {\n                  lines.push({ role: msg.role, content: chunk, messageIndex: msgIndex });\n                  chunk = char;\n                  chunkWidth = charWidth;\n                } else {\n                  chunk += char;\n                  chunkWidth += charWidth;\n                }\n              }\n              if (chunk) {\n                currentLine = chunk;\n                currentWidth = chunkWidth;\n              }\n              continue;\n            }\n\n            // Check if word fits on current line\n            if (currentWidth + wordWidth > maxWidth) {\n              // Flush current line and start new one\n              if (currentLine.trim()) {\n                lines.push({ role: msg.role, content: currentLine.trimEnd(), messageIndex: msgIndex });\n              }\n              // Don't start line with whitespace\n              currentLine = word.trim() ? word : '';\n              currentWidth = word.trim() ? wordWidth : 0;\n            } else {\n              currentLine += word;\n              currentWidth += wordWidth;\n            }\n          }\n\n          // Flush remaining content\n          if (currentLine.trim()) {\n            lines.push({ role: msg.role, content: currentLine.trimEnd(), messageIndex: msgIndex });\n          } else if (lines.length === 0 || lines[lines.length - 1]?.messageIndex !== msgIndex) {\n            // Ensure at least one line per content section\n            lines.push({ role: msg.role, content: ' ', messageIndex: msgIndex });\n          }\n        }\n      }"
    },
    {
      "type": "arrow_function",
      "line": 1734,
      "column": 36,
      "text": "(provider, idx) => (\n              <Box key={provider}>\n                <Text\n                  backgroundColor={\n                    idx === selectedProviderIndex ? 'cyan' : undefined\n                  }\n                  color={idx === selectedProviderIndex ? 'black' : 'white'}\n                >\n                  {idx === selectedProviderIndex ? '> ' : '  '}\n                  {provider}\n                  {provider === currentProvider ? ' (current)' : ''}\n                </Text>\n              </Box>\n            )"
    },
    {
      "type": "arrow_function",
      "line": 1789,
      "column": 44,
      "text": "(entry, idx) => (\n              <Box key={`${entry.sessionId}-${entry.timestamp}`}>\n                <Text\n                  backgroundColor={idx === searchResultIndex ? 'magenta' : undefined}\n                  color={idx === searchResultIndex ? 'black' : 'white'}\n                >\n                  {idx === searchResultIndex ? '> ' : '  '}\n                  {entry.display.slice(0, terminalWidth - 10)}\n                </Text>\n              </Box>\n            )"
    },
    {
      "type": "arrow_function",
      "line": 1840,
      "column": 48,
      "text": "(session, idx) => {\n              const isSelected = idx === resumeSessionIndex;\n              const updatedAt = new Date(session.updatedAt);\n              const timeAgo = formatTimeAgo(updatedAt);\n              const provider = session.provider || 'unknown';\n              return (\n                <Box key={session.id} flexDirection=\"column\">\n                  <Text\n                    backgroundColor={isSelected ? 'blue' : undefined}\n                    color={isSelected ? 'black' : 'white'}\n                  >\n                    {isSelected ? '> ' : '  '}\n                    {session.name}\n                  </Text>\n                  <Text\n                    backgroundColor={isSelected ? 'blue' : undefined}\n                    color={isSelected ? 'black' : 'gray'}\n                    dimColor={!isSelected}\n                  >\n                    {'    '}\n                    {session.messageCount} messages | {provider} | {timeAgo}\n                  </Text>\n                </Box>\n              );\n            })}"
    },
    {
      "type": "arrow_function",
      "line": 1930,
      "column": 24,
      "text": "(line) => {\n              const color =\n                line.role === 'user'\n                  ? 'green'\n                  : line.role === 'tool'\n                    ? 'yellow'\n                    : 'white';\n              return (\n                <Box flexGrow={1}>\n                  <Text color={color}>{line.content}</Text>\n                </Box>\n              );\n            }"
    },
    {
      "type": "arrow_function",
      "line": 1943,
      "column": 26,
      "text": "(_line, index) => `line-${index}`}\n            emptyMessage=\"Type a message to start...\""
    }
  ]
}
{
  "matches": [
    {
      "type": "function_item",
      "name": "default",
      "line": 18,
      "column": 4,
      "text": "fn default() -> Self {\n        Self {\n            input_tokens: 0,\n            output_tokens: 0,\n            cache_read_input_tokens: Some(0),\n            cache_creation_input_tokens: Some(0),\n        }\n    }"
    },
    {
      "type": "function_item",
      "name": "text",
      "line": 89,
      "column": 4,
      "text": "pub fn text(text: String) -> Self {\n        Self {\n            chunk_type: \"Text\".to_string(),\n            text: Some(text),\n            tool_call: None,\n            tool_result: None,\n            status: None,\n            queued_inputs: None,\n            tokens: None,\n            error: None,\n        }\n    }"
    },
    {
      "type": "function_item",
      "name": "tool_call",
      "line": 102,
      "column": 4,
      "text": "pub fn tool_call(info: ToolCallInfo) -> Self {\n        Self {\n            chunk_type: \"ToolCall\".to_string(),\n            text: None,\n            tool_call: Some(info),\n            tool_result: None,\n            status: None,\n            queued_inputs: None,\n            tokens: None,\n            error: None,\n        }\n    }"
    },
    {
      "type": "function_item",
      "name": "tool_result",
      "line": 115,
      "column": 4,
      "text": "pub fn tool_result(info: ToolResultInfo) -> Self {\n        Self {\n            chunk_type: \"ToolResult\".to_string(),\n            text: None,\n            tool_call: None,\n            tool_result: Some(info),\n            status: None,\n            queued_inputs: None,\n            tokens: None,\n            error: None,\n        }\n    }"
    },
    {
      "type": "function_item",
      "name": "status",
      "line": 128,
      "column": 4,
      "text": "pub fn status(message: String) -> Self {\n        Self {\n            chunk_type: \"Status\".to_string(),\n            text: None,\n            tool_call: None,\n            tool_result: None,\n            status: Some(message),\n            queued_inputs: None,\n            tokens: None,\n            error: None,\n        }\n    }"
    },
    {
      "type": "function_item",
      "name": "interrupted",
      "line": 141,
      "column": 4,
      "text": "pub fn interrupted(queued_inputs: Vec<String>) -> Self {\n        Self {\n            chunk_type: \"Interrupted\".to_string(),\n            text: None,\n            tool_call: None,\n            tool_result: None,\n            status: None,\n            queued_inputs: Some(queued_inputs),\n            tokens: None,\n            error: None,\n        }\n    }"
    },
    {
      "type": "function_item",
      "name": "token_update",
      "line": 154,
      "column": 4,
      "text": "pub fn token_update(tokens: TokenTracker) -> Self {\n        Self {\n            chunk_type: \"TokenUpdate\".to_string(),\n            text: None,\n            tool_call: None,\n            tool_result: None,\n            status: None,\n            queued_inputs: None,\n            tokens: Some(tokens),\n            error: None,\n        }\n    }"
    },
    {
      "type": "function_item",
      "name": "done",
      "line": 167,
      "column": 4,
      "text": "pub fn done() -> Self {\n        Self {\n            chunk_type: \"Done\".to_string(),\n            text: None,\n            tool_call: None,\n            tool_result: None,\n            status: None,\n            queued_inputs: None,\n            tokens: None,\n            error: None,\n        }\n    }"
    },
    {
      "type": "function_item",
      "name": "error",
      "line": 180,
      "column": 4,
      "text": "pub fn error(message: String) -> Self {\n        Self {\n            chunk_type: \"Error\".to_string(),\n            text: None,\n            tool_call: None,\n            tool_result: None,\n            status: None,\n            queued_inputs: None,\n            tokens: None,\n            error: Some(message),\n        }\n    }"
    }
  ]
}
{
  "matches": [
    {
      "type": "function_item",
      "name": "new",
      "line": 49,
      "column": 4,
      "text": "pub fn new(provider_name: Option<String>) -> Result<Self> {\n        // Load environment variables from .env file (if present)\n        // This is required for API keys to be available when running from Node.js\n        let _ = dotenvy::dotenv();\n\n        let session = codelet_cli::session::Session::new(provider_name.as_deref())\n            .map_err(|e| Error::from_reason(format!(\"Failed to create session: {e}\")))?;\n\n        // Inject context reminders (CLAUDE.md discovery, environment info)\n        let mut session = session;\n        session.inject_context_reminders();\n\n        Ok(Self {\n            inner: Arc::new(Mutex::new(session)),\n            is_interrupted: Arc::new(AtomicBool::new(false)),\n            interrupt_notify: Arc::new(Notify::new()),\n        })\n    }"
    },
    {
      "type": "function_item",
      "name": "interrupt",
      "line": 80,
      "column": 4,
      "text": "pub fn interrupt(&self) {\n        self.is_interrupted.store(true, Release);\n        // Wake any waiting stream loop immediately (NAPI-004)\n        // Uses notify_one() to store permit if not currently waiting in select\n        self.interrupt_notify.notify_one();\n    }"
    },
    {
      "type": "function_item",
      "name": "reset_interrupt",
      "line": 92,
      "column": 4,
      "text": "pub fn reset_interrupt(&self) {\n        self.is_interrupted.store(false, Release);\n    }"
    },
    {
      "type": "function_item",
      "name": "toggle_debug",
      "line": 106,
      "column": 4,
      "text": "pub fn toggle_debug(&self, debug_dir: Option<String>) -> Result<DebugCommandResult> {\n        let result = handle_debug_command_with_dir(debug_dir.as_deref());\n\n        // If debug was just enabled, set session metadata\n        if result.enabled {\n            let session = self.inner.blocking_lock();\n            if let Ok(manager_arc) = get_debug_capture_manager() {\n                if let Ok(mut manager) = manager_arc.lock() {\n                    manager.set_session_metadata(SessionMetadata {\n                        provider: Some(session.current_provider_name().to_string()),\n                        model: Some(session.current_provider_name().to_string()),\n                        context_window: Some(session.provider_manager().context_window()),\n                        max_output_tokens: None,\n                    });\n                }\n            }\n        }\n\n        Ok(DebugCommandResult {\n            enabled: result.enabled,\n            session_file: result.session_file,\n            message: result.message,\n        })\n    }"
    },
    {
      "type": "function_item",
      "name": "compact",
      "line": 139,
      "column": 4,
      "text": "pub async fn compact(&self) -> Result<CompactionResult> {\n        let mut session = self.inner.lock().await;\n\n        // Check if there's anything to compact\n        if session.messages.is_empty() {\n            return Err(Error::from_reason(\"Nothing to compact - no messages yet\"));\n        }\n\n        // Get current token count for reporting\n        let original_tokens = session.token_tracker.input_tokens;\n\n        // Capture compaction.manual.start event\n        if let Ok(manager_arc) = get_debug_capture_manager() {\n            if let Ok(mut manager) = manager_arc.lock() {\n                if manager.is_enabled() {\n                    manager.capture(\n                        \"compaction.manual.start\",\n                        serde_json::json!({\n                            \"command\": \"/compact\",\n                            \"originalTokens\": original_tokens,\n                            \"messageCount\": session.messages.len(),\n                        }),\n                        None,\n                    );\n                }\n            }\n        }\n\n        // Execute compaction\n        let metrics = execute_compaction(&mut session).await.map_err(|e| {\n            // Capture compaction.manual.failed event\n            if let Ok(manager_arc) = get_debug_capture_manager() {\n                if let Ok(mut manager) = manager_arc.lock() {\n                    if manager.is_enabled() {\n                        manager.capture(\n                            \"compaction.manual.failed\",\n                            serde_json::json!({\n                                \"command\": \"/compact\",\n                                \"error\": e.to_string(),\n                            }),\n                            None,\n                        );\n                    }\n                }\n            }\n            Error::from_reason(format!(\"Compaction failed: {e}\"))\n        })?;\n\n        // Capture compaction.manual.complete event\n        if let Ok(manager_arc) = get_debug_capture_manager() {\n            if let Ok(mut manager) = manager_arc.lock() {\n                if manager.is_enabled() {\n                    manager.capture(\n                        \"compaction.manual.complete\",\n                        serde_json::json!({\n                            \"command\": \"/compact\",\n                            \"originalTokens\": metrics.original_tokens,\n                            \"compactedTokens\": metrics.compacted_tokens,\n                            \"compressionRatio\": metrics.compression_ratio,\n                            \"turnsSummarized\": metrics.turns_summarized,\n                            \"turnsKept\": metrics.turns_kept,\n                        }),\n                        None,\n                    );\n                }\n            }\n        }\n\n        Ok(CompactionResult {\n            original_tokens: metrics.original_tokens as u32,\n            compacted_tokens: metrics.compacted_tokens as u32,\n            compression_ratio: metrics.compression_ratio * 100.0, // Convert to percentage\n            turns_summarized: metrics.turns_summarized as u32,\n            turns_kept: metrics.turns_kept as u32,\n        })\n    }"
    },
    {
      "type": "function_item",
      "name": "current_provider_name",
      "line": 218,
      "column": 4,
      "text": "pub fn current_provider_name(&self) -> Result<String> {\n        // Use blocking lock for sync getter\n        let session = self.inner.blocking_lock();\n        Ok(session.current_provider_name().to_string())\n    }"
    },
    {
      "type": "function_item",
      "name": "available_providers",
      "line": 226,
      "column": 4,
      "text": "pub fn available_providers(&self) -> Result<Vec<String>> {\n        let session = self.inner.blocking_lock();\n\n        // Get raw provider names without formatting\n        let providers = session.provider_manager().list_available_providers();\n\n        // Strip formatting like \"Claude (/claude)\" -> \"claude\"\n        let clean_providers: Vec<String> = providers\n            .into_iter()\n            .map(|p| {\n                // Extract provider name from format like \"Claude (/claude)\"\n                if let Some(start) = p.find(\"(/\") {\n                    if let Some(end) = p.find(')') {\n                        return p[start + 2..end].to_string();\n                    }\n                }\n                p.to_lowercase()\n            })\n            .collect();\n\n        Ok(clean_providers)\n    }"
    },
    {
      "type": "function_item",
      "name": "token_tracker",
      "line": 251,
      "column": 4,
      "text": "pub fn token_tracker(&self) -> Result<TokenTracker> {\n        let session = self.inner.blocking_lock();\n\n        Ok(TokenTracker {\n            input_tokens: session.token_tracker.input_tokens as u32,\n            output_tokens: session.token_tracker.output_tokens as u32,\n            cache_read_input_tokens: session\n                .token_tracker\n                .cache_read_input_tokens\n                .map(|t| t as u32),\n            cache_creation_input_tokens: session\n                .token_tracker\n                .cache_creation_input_tokens\n                .map(|t| t as u32),\n        })\n    }"
    },
    {
      "type": "function_item",
      "name": "messages",
      "line": 270,
      "column": 4,
      "text": "pub fn messages(&self) -> Result<Vec<Message>> {\n        let session = self.inner.blocking_lock();\n\n        let messages: Vec<Message> = session\n            .messages\n            .iter()\n            .map(|msg| {\n                let (role, content) = match msg {\n                    rig::message::Message::User { content, .. } => {\n                        let text = content\n                            .iter()\n                            .filter_map(|c| match c {\n                                rig::message::UserContent::Text(t) => Some(t.text.clone()),\n                                _ => None,\n                            })\n                            .collect::<Vec<_>>()\n                            .join(\"\\n\");\n                        let text = if text.is_empty() {\n                            \"[non-text content]\".to_string()\n                        } else {\n                            text\n                        };\n                        (\"user\".to_string(), text)\n                    }\n                    rig::message::Message::Assistant { content, .. } => {\n                        let text = content\n                            .iter()\n                            .filter_map(|c| match c {\n                                rig::message::AssistantContent::Text(t) => Some(t.text.clone()),\n                                _ => None,\n                            })\n                            .collect::<Vec<_>>()\n                            .join(\"\\n\");\n                        let text = if text.is_empty() {\n                            \"[non-text content]\".to_string()\n                        } else {\n                            text\n                        };\n                        (\"assistant\".to_string(), text)\n                    }\n                };\n                Message { role, content }\n            })\n            .collect();\n\n        Ok(messages)\n    }"
    },
    {
      "type": "function_item",
      "name": "switch_provider",
      "line": 320,
      "column": 4,
      "text": "pub async fn switch_provider(&self, provider_name: String) -> Result<()> {\n        let mut session = self.inner.lock().await;\n\n        session\n            .switch_provider(&provider_name)\n            .map_err(|e| Error::from_reason(format!(\"Failed to switch provider: {e}\")))?;\n\n        Ok(())\n    }"
    },
    {
      "type": "function_item",
      "name": "clear_history",
      "line": 339,
      "column": 4,
      "text": "pub fn clear_history(&self) -> Result<()> {\n        let mut session = self.inner.blocking_lock();\n\n        session.messages.clear();\n        session.turns.clear();\n        session.token_tracker = codelet_core::compaction::TokenTracker {\n            input_tokens: 0,\n            output_tokens: 0,\n            cache_read_input_tokens: Some(0),\n            cache_creation_input_tokens: Some(0),\n        };\n\n        // Reinject context reminders to restore CLAUDE.md and environment info\n        // This ensures the AI retains project context after clearing history\n        session.inject_context_reminders();\n\n        Ok(())\n    }"
    },
    {
      "type": "function_item",
      "name": "restore_messages",
      "line": 377,
      "column": 4,
      "text": "pub fn restore_messages(&self, messages: Vec<Message>) -> Result<()> {\n        use rig::message::{AssistantContent, Message as RigMessage, UserContent};\n        use rig::OneOrMany;\n\n        let mut session = self.inner.blocking_lock();\n\n        // Clear existing state before restoring\n        session.messages.clear();\n        session.turns.clear();\n        session.token_tracker = codelet_core::compaction::TokenTracker {\n            input_tokens: 0,\n            output_tokens: 0,\n            cache_read_input_tokens: Some(0),\n            cache_creation_input_tokens: Some(0),\n        };\n\n        // Convert persistence messages to rig messages\n        for msg in messages {\n            let rig_msg = match msg.role.as_str() {\n                \"user\" => RigMessage::User {\n                    content: OneOrMany::one(UserContent::text(&msg.content)),\n                },\n                \"assistant\" => RigMessage::Assistant {\n                    id: None,\n                    content: OneOrMany::one(AssistantContent::text(&msg.content)),\n                },\n                // Skip unknown roles (e.g., \"tool\" messages from persistence)\n                _ => continue,\n            };\n            session.messages.push(rig_msg);\n        }\n\n        // Re-inject context reminders to ensure CLAUDE.md and environment info\n        // are present after restoration\n        session.inject_context_reminders();\n\n        Ok(())\n    }"
    },
    {
      "type": "function_item",
      "name": "prompt",
      "line": 425,
      "column": 4,
      "text": "pub async fn prompt(\n        &self,\n        input: String,\n        #[napi(ts_arg_type = \"(chunk: StreamChunk) => void\")] callback: StreamCallback,\n    ) -> Result<()> {\n        // Reset interrupt flag at start of each prompt\n        self.is_interrupted.store(false, Release);\n\n        // Clone Arcs for use in async block\n        let session_arc = Arc::clone(&self.inner);\n        let is_interrupted = Arc::clone(&self.is_interrupted);\n        let interrupt_notify = Arc::clone(&self.interrupt_notify);\n\n        // Create NAPI output handler\n        let output = NapiOutput::new(&callback);\n\n        // Lock session and run the stream\n        let mut session = session_arc.lock().await;\n\n        // Get provider and create agent\n        let current_provider = session.current_provider_name().to_string();\n\n        let result = match current_provider.as_str() {\n            \"claude\" => {\n                let provider = session\n                    .provider_manager_mut()\n                    .get_claude()\n                    .map_err(|e| Error::from_reason(format!(\"Failed to get provider: {e}\")))?;\n                let rig_agent = provider.create_rig_agent(None);\n                let agent = RigAgent::with_default_depth(rig_agent);\n                run_agent_stream(\n                    agent,\n                    &input,\n                    &mut session,\n                    is_interrupted,\n                    Arc::clone(&interrupt_notify),\n                    &output,\n                )\n                .await\n            }\n            \"openai\" => {\n                let provider = session\n                    .provider_manager_mut()\n                    .get_openai()\n                    .map_err(|e| Error::from_reason(format!(\"Failed to get provider: {e}\")))?;\n                let rig_agent = provider.create_rig_agent(None);\n                let agent = RigAgent::with_default_depth(rig_agent);\n                run_agent_stream(\n                    agent,\n                    &input,\n                    &mut session,\n                    is_interrupted,\n                    Arc::clone(&interrupt_notify),\n                    &output,\n                )\n                .await\n            }\n            \"gemini\" => {\n                let provider = session\n                    .provider_manager_mut()\n                    .get_gemini()\n                    .map_err(|e| Error::from_reason(format!(\"Failed to get provider: {e}\")))?;\n                let rig_agent = provider.create_rig_agent(None);\n                let agent = RigAgent::with_default_depth(rig_agent);\n                run_agent_stream(\n                    agent,\n                    &input,\n                    &mut session,\n                    is_interrupted,\n                    Arc::clone(&interrupt_notify),\n                    &output,\n                )\n                .await\n            }\n            \"codex\" => {\n                let provider = session\n                    .provider_manager_mut()\n                    .get_codex()\n                    .map_err(|e| Error::from_reason(format!(\"Failed to get provider: {e}\")))?;\n                let rig_agent = provider.create_rig_agent(None);\n                let agent = RigAgent::with_default_depth(rig_agent);\n                run_agent_stream(\n                    agent,\n                    &input,\n                    &mut session,\n                    is_interrupted,\n                    interrupt_notify,\n                    &output,\n                )\n                .await\n            }\n            _ => {\n                return Err(Error::from_reason(format!(\n                    \"Unsupported provider: {current_provider}\"\n                )));\n            }\n        };\n\n        result.map_err(|e| Error::from_reason(format!(\"Stream error: {e}\")))\n    }"
    }
  ]
}
{
  "matches": [
    {
      "type": "function_item",
      "name": "run_agent_stream_with_interruption",
      "line": 44,
      "column": 0,
      "text": "pub(super) async fn run_agent_stream_with_interruption<M, O>(\n    agent: RigAgent<M>,\n    prompt: &str,\n    session: &mut Session,\n    event_stream: &mut (dyn futures::Stream<Item = TuiEvent> + Unpin + Send),\n    input_queue: &mut InputQueue,\n    is_interrupted: Arc<AtomicBool>,\n    output: &O,\n) -> Result<()>\nwhere\n    M: CompletionModel,\n    M::StreamingResponse: WasmCompatSend + GetTokenUsage,\n    O: StreamOutput,\n{\n    run_agent_stream_internal(\n        agent,\n        prompt,\n        session,\n        Some(event_stream),\n        Some(input_queue),\n        is_interrupted,\n        None, // CLI mode doesn't use Notify - uses keyboard event stream\n        output,\n    )\n    .await\n}"
    },
    {
      "type": "function_item",
      "name": "run_agent_stream",
      "line": 78,
      "column": 0,
      "text": "pub async fn run_agent_stream<M, O>(\n    agent: RigAgent<M>,\n    prompt: &str,\n    session: &mut Session,\n    is_interrupted: Arc<AtomicBool>,\n    interrupt_notify: Arc<Notify>,\n    output: &O,\n) -> Result<()>\nwhere\n    M: CompletionModel,\n    M::StreamingResponse: WasmCompatSend + GetTokenUsage,\n    O: StreamOutput,\n{\n    run_agent_stream_internal::<M, O, dyn futures::Stream<Item = TuiEvent> + Unpin + Send>(\n        agent,\n        prompt,\n        session,\n        None,\n        None,\n        is_interrupted,\n        Some(interrupt_notify),\n        output,\n    )\n    .await\n}"
    },
    {
      "type": "function_item",
      "name": "run_agent_stream_internal",
      "line": 111,
      "column": 0,
      "text": "async fn run_agent_stream_internal<M, O, E>(\n    agent: RigAgent<M>,\n    prompt: &str,\n    session: &mut Session,\n    mut event_stream: Option<&mut E>,\n    mut input_queue: Option<&mut InputQueue>,\n    is_interrupted: Arc<AtomicBool>,\n    interrupt_notify: Option<Arc<Notify>>,\n    output: &O,\n) -> Result<()>\nwhere\n    M: CompletionModel,\n    M::StreamingResponse: WasmCompatSend + GetTokenUsage,\n    O: StreamOutput,\n    E: futures::Stream<Item = TuiEvent> + Unpin + Send + ?Sized,\n{\n    use rig::message::{Message, UserContent};\n    use rig::OneOrMany;\n    use std::time::Instant;\n    use uuid::Uuid;\n\n    // CLI-022: Generate request ID for correlation\n    let request_id = Uuid::new_v4().to_string();\n    let api_start_time = Instant::now();\n\n    // CRITICAL: Add user prompt to message history for persistence (CLI-008)\n    session.messages.push(Message::User {\n        content: OneOrMany::one(UserContent::text(prompt)),\n    });\n\n    // HOOK-BASED COMPACTION\n    let context_window = session.provider_manager().context_window() as u64;\n    let threshold = calculate_compaction_threshold(context_window);\n\n    let token_state = Arc::new(Mutex::new(TokenState {\n        input_tokens: session.token_tracker.input_tokens,\n        cache_read_input_tokens: session.token_tracker.cache_read_input_tokens.unwrap_or(0),\n        cache_creation_input_tokens: session\n            .token_tracker\n            .cache_creation_input_tokens\n            .unwrap_or(0),\n        compaction_needed: false,\n    }));\n\n    let hook = CompactionHook::new(Arc::clone(&token_state), threshold);\n\n    // DEBUG: Log compaction check setup\n    if let Ok(manager_arc) = get_debug_capture_manager() {\n        if let Ok(mut manager) = manager_arc.lock() {\n            if manager.is_enabled() {\n                if let Ok(state) = token_state.lock() {\n                    manager.capture(\n                        \"compaction.check\",\n                        serde_json::json!({\n                            \"timing\": \"hook-setup\",\n                            \"inputTokens\": state.input_tokens,\n                            \"cacheReadInputTokens\": state.cache_read_input_tokens,\n                            \"threshold\": threshold,\n                            \"contextWindow\": context_window,\n                            \"messageCount\": session.messages.len(),\n                        }),\n                        None,\n                    );\n                }\n            }\n        }\n    }\n\n    // CLI-022: Capture api.request event\n    if let Ok(manager_arc) = get_debug_capture_manager() {\n        if let Ok(mut manager) = manager_arc.lock() {\n            if manager.is_enabled() {\n                manager.capture(\n                    \"api.request\",\n                    serde_json::json!({\n                        \"provider\": session.current_provider_name(),\n                        \"model\": session.current_provider_name(),\n                        \"prompt\": prompt,\n                        \"promptLength\": prompt.len(),\n                        \"messageCount\": session.messages.len(),\n                    }),\n                    Some(codelet_common::debug_capture::CaptureOptions {\n                        request_id: Some(request_id.clone()),\n                    }),\n                );\n            }\n        }\n    }\n\n    // Start streaming with history and hook\n    let mut stream = agent\n        .prompt_streaming_with_history_and_hook(prompt, &mut session.messages, hook)\n        .await;\n\n    // CLI-022: Capture api.response.start event\n    if let Ok(manager_arc) = get_debug_capture_manager() {\n        if let Ok(mut manager) = manager_arc.lock() {\n            if manager.is_enabled() {\n                manager.capture(\n                    \"api.response.start\",\n                    serde_json::json!({\n                        \"provider\": session.current_provider_name(),\n                    }),\n                    Some(codelet_common::debug_capture::CaptureOptions {\n                        request_id: Some(request_id.clone()),\n                    }),\n                );\n            }\n        }\n    }\n\n    // Only create status display and interval for CLI mode (unused in NAPI)\n    let status = if event_stream.is_some() {\n        Some(StatusDisplay::new())\n    } else {\n        None\n    };\n    let mut status_interval = if event_stream.is_some() {\n        Some(interval(Duration::from_secs(1)))\n    } else {\n        None\n    };\n\n    // Track assistant response content for adding to messages (CLI-008)\n    let mut assistant_text = String::new();\n    let mut tool_calls_buffer: Vec<rig::message::AssistantContent> = Vec::new();\n    let mut last_tool_name: Option<String> = None;\n\n    // Store previous cumulative totals for emitting running totals\n    let prev_input_tokens = session.token_tracker.input_tokens;\n    let prev_output_tokens = session.token_tracker.output_tokens;\n\n    // Track accumulated tokens within THIS run_agent_stream call\n    // (which may have multiple API calls due to tool use)\n    let mut turn_accumulated_input: u64 = 0;\n    let mut turn_accumulated_output: u64 = 0;\n    let mut turn_cache_read: u64 = 0;\n    let mut turn_cache_creation: u64 = 0;\n    // Track current API call's tokens (reset on FinalResponse for next tool call)\n    let mut current_api_input: u64 = 0;\n    let mut current_api_output: u64 = 0;\n\n    loop {\n        // Check interruption at start of each iteration (works for both modes)\n        // Use Acquire ordering to synchronize with Release store from interrupt setter\n        if is_interrupted.load(Acquire) {\n            // Emit interrupted notification\n            let queued = if let Some(ref mut iq) = input_queue {\n                iq.dequeue_all()\n            } else {\n                vec![]\n            };\n            output.emit_interrupted(&queued);\n\n            // Still add partial response to message history\n            if !assistant_text.is_empty() {\n                handle_final_response(&assistant_text, &mut session.messages)?;\n            }\n\n            output.emit_done();\n            break;\n        }\n\n        // Process next chunk - different based on mode\n        let chunk = match (&mut event_stream, &mut status_interval, &status) {\n            (Some(es), Some(si), Some(st)) => {\n                // CLI mode: Use tokio::select! with event stream and status interval\n                tokio::select! {\n                    c = stream.next() => Some(c),\n                    event = es.next() => {\n                        if let Some(TuiEvent::Key(key)) = event {\n                            if key.code == KeyCode::Esc {\n                                is_interrupted.store(true, Release);\n                            }\n                        }\n                        None // No chunk, loop will check interrupted flag\n                    }\n                    _ = si.tick() => {\n                        let _ = st.format_status();\n                        None // No chunk, continue loop\n                    }\n                }\n            }\n            _ => {\n                // NAPI mode: Use tokio::select! with interrupt notification (NAPI-004)\n                // This allows immediate ESC response even during blocking operations\n                match &interrupt_notify {\n                    Some(notify) => {\n                        let interrupt_fut = notify.notified();\n                        tokio::select! {\n                            c = stream.next() => Some(c),\n                            _ = interrupt_fut => None, // Wakes immediately when interrupt() called\n                        }\n                    }\n                    None => {\n                        // Fallback for any mode without notify (shouldn't happen in practice)\n                        Some(stream.next().await)\n                    }\n                }\n            }\n        };\n\n        // Process chunk if we got one\n        if let Some(chunk) = chunk {\n            match chunk {\n                Some(Ok(MultiTurnStreamItem::StreamAssistantItem(\n                    StreamedAssistantContent::Text(text),\n                ))) => {\n                    handle_text_chunk(&text.text, &mut assistant_text, Some(&request_id), output)?;\n                }\n                Some(Ok(MultiTurnStreamItem::StreamAssistantItem(\n                    StreamedAssistantContent::ToolCall(tool_call),\n                ))) => {\n                    handle_tool_call(\n                        &tool_call,\n                        &mut session.messages,\n                        &mut assistant_text,\n                        &mut tool_calls_buffer,\n                        &mut last_tool_name,\n                        output,\n                    )?;\n                }\n                Some(Ok(MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult(\n                    tool_result,\n                )))) => {\n                    handle_tool_result(\n                        &tool_result,\n                        &mut session.messages,\n                        &mut tool_calls_buffer,\n                        &last_tool_name,\n                        output,\n                    )?;\n\n                    // Emit intermediate CUMULATIVE token update after tool result\n                    // Use accumulated values (already updated by Usage and FinalResponse handlers)\n                    output.emit_tokens(&TokenInfo {\n                        input_tokens: prev_input_tokens + turn_accumulated_input,\n                        output_tokens: prev_output_tokens + turn_accumulated_output,\n                        cache_read_input_tokens: Some(turn_cache_read),\n                        cache_creation_input_tokens: Some(turn_cache_creation),\n                    });\n                }\n                Some(Ok(MultiTurnStreamItem::Usage(usage))) => {\n                    // Usage events come from:\n                    // 1. MessageStart (input tokens, output=0) - marks start of new API call\n                    // 2. MessageDelta (input + output tokens) - streaming updates\n\n                    if usage.output_tokens == 0 {\n                        // MessageStart - new API call starting\n                        // First, commit previous API call's tokens (if any) to accumulated totals\n                        // This handles multi-API-call turns (tool use) where FinalResponse only comes at end\n                        turn_accumulated_input += current_api_input;\n                        turn_accumulated_output += current_api_output;\n                        // Now track the new API call's input tokens\n                        current_api_input = usage.input_tokens;\n                        current_api_output = 0;\n                    } else {\n                        // MessageDelta - update current API call's output tokens\n                        current_api_output = usage.output_tokens;\n                    }\n\n                    turn_cache_read = usage.cache_read_input_tokens.unwrap_or(turn_cache_read);\n                    turn_cache_creation = usage\n                        .cache_creation_input_tokens\n                        .unwrap_or(turn_cache_creation);\n\n                    // Emit CUMULATIVE totals (previous session + accumulated + current API call)\n                    output.emit_tokens(&TokenInfo {\n                        input_tokens: prev_input_tokens\n                            + turn_accumulated_input\n                            + current_api_input,\n                        output_tokens: prev_output_tokens\n                            + turn_accumulated_output\n                            + current_api_output,\n                        cache_read_input_tokens: Some(turn_cache_read),\n                        cache_creation_input_tokens: Some(turn_cache_creation),\n                    });\n                }\n                Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp))) => {\n                    // Get final usage directly from FinalResponse (most accurate source)\n                    let usage = final_resp.usage();\n\n                    // For non-Anthropic providers that don't emit Usage, use FinalResponse values\n                    if current_api_input == 0 {\n                        current_api_input = usage.input_tokens;\n                    }\n                    current_api_output = usage.output_tokens;\n\n                    // Commit final API call to accumulated totals\n                    turn_accumulated_input += current_api_input;\n                    turn_accumulated_output += current_api_output;\n\n                    // Update cache tokens from FinalResponse\n                    turn_cache_read = usage.cache_read_input_tokens.unwrap_or(turn_cache_read);\n                    turn_cache_creation = usage\n                        .cache_creation_input_tokens\n                        .unwrap_or(turn_cache_creation);\n\n                    // Emit CUMULATIVE token update\n                    output.emit_tokens(&TokenInfo {\n                        input_tokens: prev_input_tokens + turn_accumulated_input,\n                        output_tokens: prev_output_tokens + turn_accumulated_output,\n                        cache_read_input_tokens: Some(turn_cache_read),\n                        cache_creation_input_tokens: Some(turn_cache_creation),\n                    });\n\n                    // CLI-022: Capture api.response.end event\n                    if let Ok(manager_arc) = get_debug_capture_manager() {\n                        if let Ok(mut manager) = manager_arc.lock() {\n                            if manager.is_enabled() {\n                                let duration_ms = api_start_time.elapsed().as_millis() as u64;\n                                manager.capture(\n                                    \"api.response.end\",\n                                    serde_json::json!({\n                                        \"duration\": duration_ms,\n                                        \"usage\": {\n                                            \"inputTokens\": usage.input_tokens,\n                                            \"outputTokens\": usage.output_tokens,\n                                            \"cacheReadInputTokens\": usage.cache_read_input_tokens,\n                                            \"cacheCreationInputTokens\": usage.cache_creation_input_tokens,\n                                        },\n                                        \"responseLength\": assistant_text.len(),\n                                    }),\n                                    Some(codelet_common::debug_capture::CaptureOptions {\n                                        request_id: Some(request_id.clone()),\n                                    }),\n                                );\n\n                                manager.capture(\n                                    \"token.update\",\n                                    serde_json::json!({\n                                        \"inputTokens\": usage.input_tokens,\n                                        \"outputTokens\": usage.output_tokens,\n                                        \"cacheReadInputTokens\": usage.cache_read_input_tokens,\n                                        \"cacheCreationInputTokens\": usage.cache_creation_input_tokens,\n                                        \"totalInputTokens\": usage.input_tokens,\n                                        \"totalOutputTokens\": usage.output_tokens,\n                                    }),\n                                    None,\n                                );\n                            }\n                        }\n                    }\n\n                    handle_final_response(&assistant_text, &mut session.messages)?;\n                    output.emit_done();\n                    break;\n                }\n                Some(Err(e)) => {\n                    // Check if this error is due to compaction hook cancellation\n                    // PromptCancelled means the hook cancelled the request because tokens > threshold\n                    let error_str = e.to_string();\n                    let is_compaction_cancel = error_str.contains(\"PromptCancelled\");\n\n                    // Check if compaction was actually triggered by the hook\n                    let compaction_triggered = token_state\n                        .lock()\n                        .map(|state| state.compaction_needed)\n                        .unwrap_or(false);\n\n                    if is_compaction_cancel && compaction_triggered {\n                        // This is a compaction cancellation - break to run compaction logic\n                        // Don't log as error, this is expected behavior\n                        break;\n                    }\n\n                    // CLI-022: Capture api.error event (for real errors, not compaction)\n                    if let Ok(manager_arc) = get_debug_capture_manager() {\n                        if let Ok(mut manager) = manager_arc.lock() {\n                            if manager.is_enabled() {\n                                manager.capture(\n                                    \"api.error\",\n                                    serde_json::json!({\n                                        \"error\": error_str,\n                                        \"duration\": api_start_time.elapsed().as_millis() as u64,\n                                    }),\n                                    Some(codelet_common::debug_capture::CaptureOptions {\n                                        request_id: Some(request_id.clone()),\n                                    }),\n                                );\n                            }\n                        }\n                    }\n                    output.emit_error(&error_str);\n                    return Err(anyhow::anyhow!(\"Agent error: {e}\"));\n                }\n                None => {\n                    // Stream ended unexpectedly\n                    if !assistant_text.is_empty() {\n                        handle_final_response(&assistant_text, &mut session.messages)?;\n                    }\n                    output.emit_done();\n                    break;\n                }\n                _ => {\n                    // Other stream items (ignored)\n                }\n            }\n\n            // Flush buffered output after processing each chunk\n            // This is a no-op for CLI (unbuffered) but triggers batched text emission for NAPI\n            // Provides ~10-50ms latency for text streaming while dramatically reducing callback count\n            output.flush();\n        }\n    }\n\n    // Check if hook triggered compaction\n    let compaction_needed = token_state\n        .lock()\n        .map(|state| state.compaction_needed)\n        .unwrap_or(false);\n\n    if compaction_needed && !is_interrupted.load(Acquire) {\n        // Capture compaction.triggered event\n        if let Ok(manager_arc) = get_debug_capture_manager() {\n            if let Ok(mut manager) = manager_arc.lock() {\n                if manager.is_enabled() {\n                    if let Ok(state) = token_state.lock() {\n                        manager.capture(\n                            \"compaction.triggered\",\n                            serde_json::json!({\n                                \"timing\": \"hook-triggered\",\n                                \"inputTokens\": state.input_tokens,\n                                \"cacheReadInputTokens\": state.cache_read_input_tokens,\n                                \"threshold\": threshold,\n                                \"contextWindow\": context_window,\n                            }),\n                            None,\n                        );\n                    }\n                }\n            }\n        }\n\n        output.emit_status(\"\\n[Generating summary...]\");\n\n        // Execute compaction\n        match execute_compaction(session).await {\n            Ok(metrics) => {\n                // Capture context.update event after compaction\n                if let Ok(manager_arc) = get_debug_capture_manager() {\n                    if let Ok(mut manager) = manager_arc.lock() {\n                        if manager.is_enabled() {\n                            manager.capture(\n                                \"context.update\",\n                                serde_json::json!({\n                                    \"type\": \"compaction\",\n                                    \"originalTokens\": metrics.original_tokens,\n                                    \"compactedTokens\": metrics.compacted_tokens,\n                                    \"compressionRatio\": metrics.compression_ratio,\n                                }),\n                                None,\n                            );\n                        }\n                    }\n                }\n\n                output.emit_status(&format!(\n                    \"[Context compacted: {}→{} tokens, {:.0}% compression]\",\n                    metrics.original_tokens,\n                    metrics.compacted_tokens,\n                    metrics.compression_ratio * 100.0\n                ));\n                output.emit_status(\"[Continuing with compacted context...]\\n\");\n\n                // NOTE: execute_compaction already sets session.token_tracker.input_tokens\n                // to the correct new_total_tokens calculated from compacted messages.\n                // We only reset output_tokens and cache metrics here.\n                session.token_tracker.output_tokens = 0;\n                session.token_tracker.cache_read_input_tokens = None;\n                session.token_tracker.cache_creation_input_tokens = None;\n\n                // Re-add the user's original prompt to session.messages so the agent\n                // can continue processing it with the compacted context\n                session.messages.push(Message::User {\n                    content: OneOrMany::one(UserContent::text(prompt)),\n                });\n\n                // Create fresh hook and token state for the retry\n                let retry_token_state = Arc::new(Mutex::new(TokenState {\n                    input_tokens: session.token_tracker.input_tokens,\n                    cache_read_input_tokens: session\n                        .token_tracker\n                        .cache_read_input_tokens\n                        .unwrap_or(0),\n                    cache_creation_input_tokens: session\n                        .token_tracker\n                        .cache_creation_input_tokens\n                        .unwrap_or(0),\n                    compaction_needed: false,\n                }));\n                let retry_hook = CompactionHook::new(Arc::clone(&retry_token_state), threshold);\n\n                // Start new stream with compacted context\n                let mut retry_stream = agent\n                    .prompt_streaming_with_history_and_hook(\n                        prompt,\n                        &mut session.messages,\n                        retry_hook,\n                    )\n                    .await;\n\n                // Reset tracking for this retry\n                let mut retry_assistant_text = String::new();\n                let mut retry_tool_calls_buffer: Vec<rig::message::AssistantContent> = Vec::new();\n                let mut retry_last_tool_name: Option<String> = None;\n\n                // Process retry stream\n                loop {\n                    if is_interrupted.load(Acquire) {\n                        let queued = if let Some(ref mut iq) = input_queue {\n                            iq.dequeue_all()\n                        } else {\n                            vec![]\n                        };\n                        output.emit_interrupted(&queued);\n                        if !retry_assistant_text.is_empty() {\n                            handle_final_response(&retry_assistant_text, &mut session.messages)?;\n                        }\n                        output.emit_done();\n                        break;\n                    }\n\n                    match retry_stream.next().await {\n                        Some(Ok(MultiTurnStreamItem::StreamAssistantItem(\n                            StreamedAssistantContent::Text(text),\n                        ))) => {\n                            handle_text_chunk(&text.text, &mut retry_assistant_text, None, output)?;\n                        }\n                        Some(Ok(MultiTurnStreamItem::StreamAssistantItem(\n                            StreamedAssistantContent::ToolCall(tool_call),\n                        ))) => {\n                            handle_tool_call(\n                                &tool_call,\n                                &mut session.messages,\n                                &mut retry_assistant_text,\n                                &mut retry_tool_calls_buffer,\n                                &mut retry_last_tool_name,\n                                output,\n                            )?;\n                        }\n                        Some(Ok(MultiTurnStreamItem::StreamUserItem(\n                            StreamedUserContent::ToolResult(tool_result),\n                        ))) => {\n                            handle_tool_result(\n                                &tool_result,\n                                &mut session.messages,\n                                &mut retry_tool_calls_buffer,\n                                &retry_last_tool_name,\n                                output,\n                            )?;\n                        }\n                        Some(Ok(MultiTurnStreamItem::Usage(usage))) => {\n                            output.emit_tokens(&TokenInfo {\n                                input_tokens: usage.input_tokens,\n                                output_tokens: usage.output_tokens,\n                                cache_read_input_tokens: usage.cache_read_input_tokens,\n                                cache_creation_input_tokens: usage.cache_creation_input_tokens,\n                            });\n                        }\n                        Some(Ok(MultiTurnStreamItem::FinalResponse(_))) => {\n                            handle_final_response(&retry_assistant_text, &mut session.messages)?;\n                            output.emit_done();\n                            break;\n                        }\n                        Some(Err(e)) => {\n                            output.emit_error(&e.to_string());\n                            return Err(anyhow::anyhow!(\"Agent error after compaction: {e}\"));\n                        }\n                        None => {\n                            if !retry_assistant_text.is_empty() {\n                                handle_final_response(\n                                    &retry_assistant_text,\n                                    &mut session.messages,\n                                )?;\n                            }\n                            output.emit_done();\n                            break;\n                        }\n                        _ => {}\n                    }\n                    output.flush();\n                }\n\n                return Ok(());\n            }\n            Err(e) => {\n                // Compaction failed - DO NOT reset token tracker!\n                // Keep the high token values so next turn will retry compaction.\n                output.emit_status(&format!(\"Warning: Compaction failed: {e}\"));\n                output.emit_status(\"[Context still large - will retry compaction on next turn]\\n\");\n\n                // Capture compaction failure for debugging\n                if let Ok(manager_arc) = get_debug_capture_manager() {\n                    if let Ok(mut manager) = manager_arc.lock() {\n                        if manager.is_enabled() {\n                            manager.capture(\n                                \"compaction.failed\",\n                                serde_json::json!({\n                                    \"error\": e.to_string(),\n                                    \"inputTokens\": session.token_tracker.input_tokens,\n                                }),\n                                None,\n                            );\n                        }\n                    }\n                }\n\n                // Return error so caller knows compaction failed\n                return Err(anyhow::anyhow!(\"Compaction failed: {e}\"));\n            }\n        }\n    }\n\n    // Update session token tracker with accumulated values from this turn\n    // Use turn_accumulated_* which correctly sums all API calls in this turn\n    if !is_interrupted.load(Acquire) {\n        session.token_tracker.input_tokens += turn_accumulated_input;\n        session.token_tracker.output_tokens += turn_accumulated_output;\n        // Cache tokens are per-request, not cumulative (use latest values)\n        session.token_tracker.cache_read_input_tokens = Some(turn_cache_read);\n        session.token_tracker.cache_creation_input_tokens = Some(turn_cache_creation);\n    }\n\n    Ok(())\n}"
    }
  ]
}
