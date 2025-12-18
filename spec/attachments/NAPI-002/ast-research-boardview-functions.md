{
  "matches": [
    {
      "type": "arrow_function",
      "line": 43,
      "column": 34,
      "text": "state => state.workUnits"
    },
    {
      "type": "arrow_function",
      "line": 44,
      "column": 33,
      "text": "state => state.cwd"
    },
    {
      "type": "arrow_function",
      "line": 45,
      "column": 31,
      "text": "state => state.setCwd"
    },
    {
      "type": "arrow_function",
      "line": 46,
      "column": 33,
      "text": "state => state.loadData"
    },
    {
      "type": "arrow_function",
      "line": 47,
      "column": 45,
      "text": "state => state.loadCheckpointCounts"
    },
    {
      "type": "arrow_function",
      "line": 48,
      "column": 39,
      "text": "state => state.moveWorkUnitUp"
    },
    {
      "type": "arrow_function",
      "line": 49,
      "column": 41,
      "text": "state => state.moveWorkUnitDown"
    },
    {
      "type": "arrow_function",
      "line": 50,
      "column": 30,
      "text": "state => state.error"
    },
    {
      "type": "arrow_function",
      "line": 51,
      "column": 33,
      "text": "state => state.isLoaded"
    },
    {
      "type": "arrow_function",
      "line": 76,
      "column": 12,
      "text": "() => {\n    if (cwd) {\n      setCwd(cwd);\n    }\n  }"
    },
    {
      "type": "arrow_function",
      "line": 83,
      "column": 12,
      "text": "() => {\n    process.stdout.write('\\x1b[?1000h'); // Enable button event tracking\n    return () => {\n      process.stdout.write('\\x1b[?1000l'); // Disable on unmount\n    };\n  }"
    },
    {
      "type": "arrow_function",
      "line": 85,
      "column": 11,
      "text": "() => {\n      process.stdout.write('\\x1b[?1000l'); // Disable on unmount\n    }"
    },
    {
      "type": "arrow_function",
      "line": 91,
      "column": 12,
      "text": "() => {\n    void loadData();\n    void loadCheckpointCounts();\n    // eslint-disable-next-line react-hooks/exhaustive-deps\n  }"
    },
    {
      "type": "arrow_function",
      "line": 100,
      "column": 12,
      "text": "() => {\n    const workUnitsPath = path.join(storeCwd, 'spec', 'work-units.json');\n\n    // Check if file exists before watching\n    if (!fs.existsSync(workUnitsPath)) return;\n\n    // Chokidar watches specific file, handles atomic operations automatically\n    const watcher = chokidar.watch(workUnitsPath, {\n      ignoreInitial: true,  // Don't trigger on initial scan\n      persistent: false,\n    });\n\n    // Listen for all change events (chokidar normalizes across platforms)\n    watcher.on('change', () => {\n      void loadData();\n    });\n\n    // Add error handler to prevent silent failures\n    watcher.on('error', (error) => {\n      console.warn('Work units watcher error:', error.message);\n    });\n\n    // Cleanup watcher on unmount\n    return () => {\n      void watcher.close();\n    };\n  }"
    },
    {
      "type": "arrow_function",
      "line": 113,
      "column": 25,
      "text": "() => {\n      void loadData();\n    }"
    },
    {
      "type": "arrow_function",
      "line": 118,
      "column": 24,
      "text": "(error) => {\n      console.warn('Work units watcher error:', error.message);\n    }"
    },
    {
      "type": "arrow_function",
      "line": 123,
      "column": 11,
      "text": "() => {\n      void watcher.close();\n    }"
    },
    {
      "type": "arrow_function",
      "line": 130,
      "column": 12,
      "text": "() => {\n    if (!showStashPanel) return;\n\n    const stashPath = path.join(storeCwd, '.git', 'refs', 'stash');\n\n    // Check if file exists before watching\n    if (!fs.existsSync(stashPath)) return;\n\n    // Chokidar watches specific file, handles atomic operations automatically\n    const watcher = chokidar.watch(stashPath, {\n      ignoreInitial: true,  // Don't trigger on initial scan\n      persistent: false,\n    });\n\n    // Listen for all change events (chokidar normalizes across platforms)\n    watcher.on('change', () => {\n      void loadCheckpointCounts();\n    });\n\n    // Add error handler to prevent silent failures (BOARD-018)\n    watcher.on('error', (error) => {\n      console.warn('Git refs watcher error:', error.message);\n    });\n\n    return () => {\n      void watcher.close();\n    };\n  }"
    },
    {
      "type": "arrow_function",
      "line": 145,
      "column": 25,
      "text": "() => {\n      void loadCheckpointCounts();\n    }"
    },
    {
      "type": "arrow_function",
      "line": 150,
      "column": 24,
      "text": "(error) => {\n      console.warn('Git refs watcher error:', error.message);\n    }"
    },
    {
      "type": "arrow_function",
      "line": 154,
      "column": 11,
      "text": "() => {\n      void watcher.close();\n    }"
    },
    {
      "type": "arrow_function",
      "line": 162,
      "column": 39,
      "text": "status => {\n    const units = workUnits.filter(wu => wu.status === status);\n    const totalPoints = units.reduce((sum, wu) => {\n      const estimate = typeof wu.estimate === 'number' ? wu.estimate : 0;\n      return sum + estimate;\n    }, 0);\n    return { status, units, count: units.length, totalPoints };\n  }"
    },
    {
      "type": "arrow_function",
      "line": 163,
      "column": 35,
      "text": "wu => wu.status === status"
    },
    {
      "type": "arrow_function",
      "line": 164,
      "column": 37,
      "text": "(sum, wu) => {\n      const estimate = typeof wu.estimate === 'number' ? wu.estimate : 0;\n      return sum + estimate;\n    }"
    },
    {
      "type": "arrow_function",
      "line": 172,
      "column": 12,
      "text": "() => {\n    if (!initialFocusSet && workUnits.length > 0) {\n      const firstNonEmptyIndex = groupedWorkUnits.findIndex(col => col.units.length > 0);\n      if (firstNonEmptyIndex >= 0) {\n        setFocusedColumnIndex(firstNonEmptyIndex);\n        setInitialFocusSet(true);\n      }\n    }\n  }"
    },
    {
      "type": "arrow_function",
      "line": 174,
      "column": 60,
      "text": "col => col.units.length > 0"
    },
    {
      "type": "arrow_function",
      "line": 183,
      "column": 12,
      "text": "() => {\n    let server: Server | null = null;\n\n    try {\n      logger.info('[BOARDVIEW] Setting up IPC server for checkpoint updates');\n      server = createIPCServer((message) => {\n        logger.info(`[BOARDVIEW] IPC message received: ${JSON.stringify(message)}`);\n        if (message.type === 'checkpoint-changed') {\n          logger.info('[BOARDVIEW] Triggering loadCheckpointCounts() from IPC event');\n          void useFspecStore.getState().loadCheckpointCounts();\n          logger.info('[BOARDVIEW] loadCheckpointCounts() call completed (async)');\n        }\n      });\n\n      const ipcPath = getIPCPath();\n      server.listen(ipcPath);\n      logger.info(`[BOARDVIEW] IPC server listening on ${ipcPath}`);\n    } catch (error) {\n      // IPC server failed to start (non-fatal - TUI still works)\n      logger.error(`[BOARDVIEW] Failed to start IPC server: ${error}`);\n    }\n\n    return () => {\n      if (server) {\n        logger.info('[BOARDVIEW] Cleaning up IPC server');\n        cleanupIPCServer(server);\n      }\n    };\n  }"
    },
    {
      "type": "arrow_function",
      "line": 188,
      "column": 31,
      "text": "(message) => {\n        logger.info(`[BOARDVIEW] IPC message received: ${JSON.stringify(message)}`);\n        if (message.type === 'checkpoint-changed') {\n          logger.info('[BOARDVIEW] Triggering loadCheckpointCounts() from IPC event');\n          void useFspecStore.getState().loadCheckpointCounts();\n          logger.info('[BOARDVIEW] loadCheckpointCounts() call completed (async)');\n        }\n      }"
    },
    {
      "type": "arrow_function",
      "line": 205,
      "column": 11,
      "text": "() => {\n      if (server) {\n        logger.info('[BOARDVIEW] Cleaning up IPC server');\n        cleanupIPCServer(server);\n      }\n    }"
    },
    {
      "type": "arrow_function",
      "line": 214,
      "column": 12,
      "text": "() => {\n    let httpServer: HttpServer | null = null;\n\n    const initServer = async () => {\n      try {\n        logger.info('[BoardView] Starting attachment server');\n        httpServer = await startAttachmentServer({\n          cwd: storeCwd,\n          port: 0, // Random available port\n        });\n\n        const port = getServerPort(httpServer);\n        if (port) {\n          setAttachmentServerPort(port);\n          logger.info(`[BoardView] Attachment server started on port ${port}`);\n        }\n      } catch (error) {\n        // Server startup failure is non-fatal - TUI continues working (REFAC-004 business rule 6)\n        logger.warn(`[BoardView] Failed to start attachment server: ${error}`);\n      }\n    };\n\n    void initServer();\n\n    return () => {\n      if (httpServer) {\n        logger.info('[BoardView] Stopping attachment server');\n        void stopAttachmentServer(httpServer).catch((error) => {\n          logger.error(`[BoardView] Error stopping attachment server: ${error}`);\n        });\n      }\n    };\n  }"
    },
    {
      "type": "arrow_function",
      "line": 217,
      "column": 23,
      "text": "async () => {\n      try {\n        logger.info('[BoardView] Starting attachment server');\n        httpServer = await startAttachmentServer({\n          cwd: storeCwd,\n          port: 0, // Random available port\n        });\n\n        const port = getServerPort(httpServer);\n        if (port) {\n          setAttachmentServerPort(port);\n          logger.info(`[BoardView] Attachment server started on port ${port}`);\n        }\n      } catch (error) {\n        // Server startup failure is non-fatal - TUI continues working (REFAC-004 business rule 6)\n        logger.warn(`[BoardView] Failed to start attachment server: ${error}`);\n      }\n    }"
    },
    {
      "type": "arrow_function",
      "line": 238,
      "column": 11,
      "text": "() => {\n      if (httpServer) {\n        logger.info('[BoardView] Stopping attachment server');\n        void stopAttachmentServer(httpServer).catch((error) => {\n          logger.error(`[BoardView] Error stopping attachment server: ${error}`);\n        });\n      }\n    }"
    },
    {
      "type": "arrow_function",
      "line": 241,
      "column": 52,
      "text": "(error) => {\n          logger.error(`[BoardView] Error stopping attachment server: ${error}`);\n        }"
    },
    {
      "type": "arrow_function",
      "line": 249,
      "column": 37,
      "text": "() => {\n    const currentColumn = groupedWorkUnits[focusedColumnIndex];\n    if (currentColumn && currentColumn.units.length > 0) {\n      return currentColumn.units[selectedWorkUnitIndex] || null;\n    }\n    return null;\n  }"
    },
    {
      "type": "arrow_function",
      "line": 258,
      "column": 25,
      "text": "() => {\n    return !!(currentlySelectedWorkUnit &&\n              currentlySelectedWorkUnit.attachments &&\n              currentlySelectedWorkUnit.attachments.length > 0);\n  }"
    },
    {
      "type": "arrow_function",
      "line": 265,
      "column": 11,
      "text": "(input, key) => {\n    if (key.escape) {\n      if (viewMode === 'detail' || viewMode === 'checkpoint-viewer' || viewMode === 'changed-files-viewer') {\n        setViewMode('board');\n        setSelectedWorkUnit(null);\n        return;\n      }\n      onExit?.();\n      return;\n    }\n\n    // C key to open checkpoint viewer (GIT-004)\n    if (input === 'c' || input === 'C') {\n      setViewMode('checkpoint-viewer');\n      return;\n    }\n\n    // F key to open changed files viewer (GIT-004)\n    if (input === 'f' || input === 'F') {\n      setViewMode('changed-files-viewer');\n      return;\n    }\n\n    // D key to open FOUNDATION.md in browser (TUI-029)\n    if (input === 'd' || input === 'D') {\n      const foundationPath = 'spec/FOUNDATION.md';\n\n      if (attachmentServerPort) {\n        // Open via HTTP server (same as attachments)\n        const url = `http://localhost:${attachmentServerPort}/view/${foundationPath}`;\n        logger.info(`[BoardView] Opening FOUNDATION.md URL: ${url}`);\n\n        openInBrowser({ url, wait: false }).catch((error: Error) => {\n          logger.error(`[BoardView] Failed to open FOUNDATION.md: ${error.message}`);\n        });\n      } else {\n        logger.warn(`[BoardView] Attachment server not available, cannot open FOUNDATION.md`);\n      }\n      return;\n    }\n\n    // A key to open attachment dialog (TUI-019)\n    if (input === 'a' || input === 'A') {\n      if (hasAttachments()) {\n        setShowAttachmentDialog(true);\n      }\n      return;\n    }\n\n    // Tab key to switch panels (BOARD-003)\n    if (key.tab) {\n      if (focusedPanel === 'board') {\n        setFocusedPanel('stash');\n      } else if (focusedPanel === 'stash') {\n        setFocusedPanel('files');\n      } else {\n        setFocusedPanel('board');\n      }\n      return;\n    }\n  }"
    },
    {
      "type": "arrow_function",
      "line": 297,
      "column": 50,
      "text": "(error: Error) => {\n          logger.error(`[BoardView] Failed to open FOUNDATION.md: ${error.message}`);\n        }"
    },
    {
      "type": "method_signature",
      "name": "setViewMode",
      "line": 332,
      "column": 24,
      "text": "setViewMode('board')"
    },
    {
      "type": "method_signature",
      "name": "setViewMode",
      "line": 343,
      "column": 24,
      "text": "setViewMode('board')"
    },
    {
      "type": "arrow_function",
      "line": 355,
      "column": 15,
      "text": "(input, key) => {\n        if (key.escape) {\n          onExit?.();\n        }\n      }"
    },
    {
      "type": "arrow_function",
      "line": 380,
      "column": 15,
      "text": "(input, key) => {\n        if (key.escape) {\n          onExit?.();\n        }\n      }"
    },
    {
      "type": "arrow_function",
      "line": 411,
      "column": 23,
      "text": "(line: string, _index: number, isSelected: boolean): React.ReactNode => {\n      const indicator = isSelected ? '>' : ' ';\n      return (\n        <Box flexGrow={1}>\n          <Text color={isSelected ? 'cyan' : 'white'}>\n            {indicator} {line || ' '}\n          </Text>\n        </Box>\n      );\n    }"
    },
    {
      "type": "arrow_function",
      "line": 425,
      "column": 15,
      "text": "(input, key) => {\n        if (key.escape) {\n          setViewMode('board');\n          setSelectedWorkUnit(null);\n        }\n      }"
    },
    {
      "type": "arrow_function",
      "line": 471,
      "column": 24,
      "text": "(delta) => {\n          setFocusedColumnIndex(prev => {\n            const newIndex = prev + delta;\n            if (newIndex < 0) return columns.length - 1;\n            if (newIndex >= columns.length) return 0;\n            return newIndex;\n          });\n          setSelectedWorkUnitIndex(0);\n        }"
    },
    {
      "type": "arrow_function",
      "line": 472,
      "column": 32,
      "text": "prev => {\n            const newIndex = prev + delta;\n            if (newIndex < 0) return columns.length - 1;\n            if (newIndex >= columns.length) return 0;\n            return newIndex;\n          }"
    },
    {
      "type": "arrow_function",
      "line": 483,
      "column": 37,
      "text": "prev => {\n              const newIndex = prev + delta;\n              if (newIndex < 0) return currentColumn.units.length - 1;\n              if (newIndex >= currentColumn.units.length) return 0;\n              return newIndex;\n            }"
    },
    {
      "type": "method_definition",
      "name": "if",
      "line": 493,
      "column": 10,
      "text": "if (focusedPanel === 'board') {\n            const currentColumn = groupedWorkUnits[focusedColumnIndex];\n            if (currentColumn.units.length > 0) {\n              const workUnit = currentColumn.units[selectedWorkUnitIndex];\n              setSelectedWorkUnit(workUnit);\n              setViewMode('detail');\n            }\n          }"
    },
    {
      "type": "method_definition",
      "name": "async",
      "line": 513,
      "column": 20,
      "text": "async () => {\n          // BOARD-010: Move work unit down with ] key\n          const currentColumn = groupedWorkUnits[focusedColumnIndex];\n          if (currentColumn.units.length > 0 && selectedWorkUnitIndex < currentColumn.units.length - 1) {\n            const workUnit = currentColumn.units[selectedWorkUnitIndex];\n            await moveWorkUnitDown(workUnit.id);\n            await loadData();\n            // BOARD-010: Move selection cursor down with the work unit\n            setSelectedWorkUnitIndex(selectedWorkUnitIndex + 1);\n          }\n        }"
    },
    {
      "type": "arrow_function",
      "line": 530,
      "column": 20,
      "text": "(attachment) => {\n            // REFAC-004: Use HTTP URL if attachment server is running, otherwise fall back to file://\n            let url: string;\n            if (attachmentServerPort) {\n              // Construct HTTP URL: http://localhost:PORT/view/{relativePath}\n              url = `http://localhost:${attachmentServerPort}/view/${attachment}`;\n              logger.info(`[BoardView] Opening attachment URL: ${url}`);\n            } else {\n              // Fallback to file:// URL if server not available\n              const absolutePath = path.isAbsolute(attachment)\n                ? attachment\n                : path.resolve(cwd || process.cwd(), attachment);\n              url = `file://${absolutePath}`;\n              logger.warn(`[BoardView] Attachment server not available, using file:// URL: ${url}`);\n            }\n\n            openInBrowser({ url, wait: false }).catch((error: Error) => {\n              logger.error(`[BoardView] Failed to open attachment: ${error.message}`);\n            });\n            setShowAttachmentDialog(false);\n          }"
    },
    {
      "type": "arrow_function",
      "line": 546,
      "column": 54,
      "text": "(error: Error) => {\n              logger.error(`[BoardView] Failed to open attachment: ${error.message}`);\n            }"
    }
  ]
}
