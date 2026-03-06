/**
 * fspec WebMCP Extension - Main-World Discovery Script
 *
 * This function runs in the page's MAIN world JavaScript context
 * (injected via chrome.scripting.executeScript with world: 'MAIN').
 *
 * It uses a layered discovery strategy to detect WebMCP tool registrations
 * from multiple sources:
 *
 * Layer 1: navigator.modelContext interception (native Chrome API + W3C polyfills)
 * Layer 2: WebMCP class prototype interception (webmcp.dev polyfill library)
 * Layer 3: Post-load snapshot of well-known globals (window.webMCP, window.mcp)
 * Layer 4: ModelContextTesting API (opportunistic, when WebMCPTesting flag enabled)
 *
 * Communication is exclusively via window.postMessage with
 * FSPEC_ prefixed message types.
 *
 * Originally implemented by: EXT-006
 * Layered discovery added by: EXT-009
 */

/**
 * The self-contained discovery function injected into the main world.
 * Must NOT reference any outer-scope variables (it's serialized and injected).
 */
export function webmcpDiscoveryFunction(): void {
  // Guard against double-injection
  if ((window as Record<string, unknown>).__fspec_webmcp_discovery_active) {
    return;
  }
  (window as Record<string, unknown>).__fspec_webmcp_discovery_active = true;

  const pageOrigin = window.location.hostname;

  /** Map of registered tool names → tool execute functions */
  const registeredTools = new Map<
    string,
    (args: Record<string, unknown>) => unknown
  >();

  /** Set of tool names already discovered (prevents duplicate notifications) */
  const discoveredToolNames = new Set<string>();

  /**
   * Reference to ModelContextTesting API if available.
   * Used by the invocation listener to call executeTool() for
   * tools discovered via Layer 4 that have no stored callback.
   */
  let modelContextTesting: {
    executeTool: (
      name: string,
      args: string,
      ...rest: unknown[]
    ) => Promise<string | null>;
  } | null = null;

  /**
   * Map of well-known WebMCP instances found during post-load snapshot.
   * Used as fallback for invocation when no execute callback was captured
   * (tools registered before injection). Instances may expose executeTool()
   * or a tools map with individual execute functions.
   */
  const snapshotInstances: Array<Record<string, unknown>> = [];

  /**
   * Notify the extension that a tool was registered.
   * Deduplicates by tool name to prevent multiple notifications for the same tool.
   */
  function notifyToolRegistered(
    name: string,
    description: string,
    inputSchema?: Record<string, unknown>,
    executeFn?: (args: Record<string, unknown>) => unknown
  ): void {
    if (executeFn) {
      registeredTools.set(name, executeFn);
    }

    if (discoveredToolNames.has(name)) {
      return;
    }
    discoveredToolNames.add(name);

    window.postMessage(
      {
        type: 'FSPEC_WEBMCP_TOOL_REGISTERED',
        tool: { name, description, inputSchema },
        origin: pageOrigin,
      },
      '*'
    );
  }

  /**
   * Notify the extension that a tool was unregistered.
   */
  function notifyToolUnregistered(name: string): void {
    registeredTools.delete(name);
    discoveredToolNames.delete(name);

    window.postMessage(
      {
        type: 'FSPEC_WEBMCP_TOOL_UNREGISTERED',
        toolName: name,
        origin: pageOrigin,
      },
      '*'
    );
  }

  // ========================================================================
  // Layer 1: navigator.modelContext interception
  // ========================================================================

  /**
   * Intercept navigator.modelContext to detect tool registrations.
   * Covers: native Chrome WebMCP API, @mcp-b/global polyfill, W3C-compliant polyfills.
   */
  function setupModelContextInterceptor(): void {
    const nav = navigator as Record<string, unknown>;

    // If navigator.modelContext already exists, intercept it
    if (nav.modelContext) {
      interceptExistingModelContext(
        nav.modelContext as Record<string, unknown>
      );
      return;
    }

    // Otherwise, watch for it to be defined (some sites set it up lazily)
    let modelContext: Record<string, unknown> | null = null;
    Object.defineProperty(nav, 'modelContext', {
      configurable: true,
      get() {
        return modelContext;
      },
      set(value: Record<string, unknown>) {
        modelContext = value;
        if (value) {
          interceptExistingModelContext(value);
        }
      },
    });
  }

  function interceptExistingModelContext(mc: Record<string, unknown>): void {
    // Layer 4: Check for ModelContextTesting API first
    if (setupModelContextTesting(mc)) {
      return; // Testing API available — no need for monkey-patching
    }

    const originalRegister = mc.registerTool as
      | ((toolDef: Record<string, unknown>) => unknown)
      | undefined;
    const originalUnregister = mc.unregisterTool as
      | ((name: string) => unknown)
      | undefined;

    mc.registerTool = function (toolDef: Record<string, unknown>): unknown {
      const name = toolDef.name as string;
      const description = (toolDef.description as string) || '';
      const inputSchema = toolDef.inputSchema as
        | Record<string, unknown>
        | undefined;
      const executeFn = toolDef.execute as
        | ((args: Record<string, unknown>) => unknown)
        | undefined;

      notifyToolRegistered(name, description, inputSchema, executeFn);

      // Call original if it exists
      if (originalRegister) {
        return originalRegister.call(mc, toolDef);
      }
      return undefined;
    };

    mc.unregisterTool = function (name: string): unknown {
      notifyToolUnregistered(name);

      // Call original if it exists
      if (originalUnregister) {
        return originalUnregister.call(mc, name);
      }
      return undefined;
    };
  }

  // ========================================================================
  // Layer 2: WebMCP class prototype interception
  // ========================================================================

  /**
   * Wrap a WebMCP class's prototype.registerTool to intercept tool registrations.
   * Covers: webmcp.dev and similar sites using the WebMCP library.
   */
  function wrapWebMCPClass(WebMCPClass: Record<string, unknown>): void {
    const proto = WebMCPClass.prototype as Record<string, unknown>;
    if (!proto || typeof proto.registerTool !== 'function') {
      return;
    }

    // Guard against double-wrapping
    if ((proto as Record<string, unknown>).__fspec_wrapped) {
      return;
    }
    (proto as Record<string, unknown>).__fspec_wrapped = true;

    const origRegister = proto.registerTool as (
      name: string,
      description: string,
      schema: Record<string, unknown>,
      fn: (args: Record<string, unknown>) => unknown
    ) => unknown;

    proto.registerTool = function (
      name: string,
      description: string,
      schema: Record<string, unknown>,
      fn: (args: Record<string, unknown>) => unknown
    ): unknown {
      notifyToolRegistered(name, description || '', schema, fn);

      // Call original
      return origRegister.call(this, name, description, schema, fn);
    };
  }

  /**
   * Set up interception of the WebMCP class on window.
   * Handles both existing and late-assigned WebMCP classes.
   */
  function setupWebMCPClassInterceptor(): void {
    const win = window as Record<string, unknown>;

    // If WebMCP class already exists, wrap it
    if (win.WebMCP && typeof win.WebMCP === 'function') {
      wrapWebMCPClass(win.WebMCP as Record<string, unknown>);
    }

    // Trap future assignment of WebMCP on window
    let currentWebMCP = win.WebMCP;
    try {
      Object.defineProperty(win, 'WebMCP', {
        configurable: true,
        get() {
          return currentWebMCP;
        },
        set(NewClass: unknown) {
          currentWebMCP = NewClass;
          if (NewClass && typeof NewClass === 'function') {
            wrapWebMCPClass(NewClass as Record<string, unknown>);
          }
        },
      });
    } catch {
      // defineProperty can fail if the property is non-configurable
    }

    // Trap late assignment of WebMCP instances on well-known globals.
    // Catches patterns like: window.webMCP = new WebMCP() after page load.
    setupInstanceTraps(win);
  }

  /**
   * Install Object.defineProperty traps on well-known instance globals
   * (window.webMCP, window.mcp) to detect late assignment of WebMCP
   * instances and wrap their registerTool methods.
   */
  function setupInstanceTraps(win: Record<string, unknown>): void {
    const instanceGlobals = ['webMCP', 'mcp', 'webmcp'];

    for (const key of instanceGlobals) {
      // If already an instance, wrap it now
      if (win[key] && typeof win[key] === 'object') {
        wrapInstanceIfNeeded(win[key] as Record<string, unknown>);
      }

      // Trap future assignment
      let currentValue = win[key];
      try {
        Object.defineProperty(win, key, {
          configurable: true,
          get() {
            return currentValue;
          },
          set(newValue: unknown) {
            currentValue = newValue;
            if (newValue && typeof newValue === 'object') {
              wrapInstanceIfNeeded(newValue as Record<string, unknown>);
            }
          },
        });
      } catch {
        // defineProperty can fail if the property is non-configurable
      }
    }
  }

  /**
   * Wrap an existing WebMCP instance's registerTool if it has one.
   * This catches tools registered on instances assigned to well-known globals.
   */
  function wrapInstanceIfNeeded(instance: Record<string, unknown>): void {
    if (
      typeof instance.registerTool !== 'function' ||
      instance.__fspec_instance_wrapped
    ) {
      return;
    }
    instance.__fspec_instance_wrapped = true;

    const origRegister = instance.registerTool as (
      name: string,
      description: string,
      schema: Record<string, unknown>,
      fn: (args: Record<string, unknown>) => unknown
    ) => unknown;

    instance.registerTool = function (
      name: string,
      description: string,
      schema: Record<string, unknown>,
      fn: (args: Record<string, unknown>) => unknown
    ): unknown {
      notifyToolRegistered(name, description || '', schema, fn);
      return origRegister.call(instance, name, description, schema, fn);
    };
  }

  // ========================================================================
  // Layer 3: Post-load snapshot
  // ========================================================================

  /**
   * Scan well-known globals for WebMCP instances that already have tools registered.
   * This catches tools registered before our script was injected.
   */
  function performPostLoadSnapshot(): void {
    const win = window as Record<string, unknown>;
    const wellKnownGlobals = ['webMCP', 'mcp', 'webmcp'];

    for (const key of wellKnownGlobals) {
      const instance = win[key] as Record<string, unknown> | undefined;
      if (!instance || typeof instance !== 'object') {
        continue;
      }

      // Check if instance has a getTools method
      if (typeof instance.getTools === 'function') {
        snapshotInstances.push(instance);
        try {
          const tools = (
            instance.getTools as () => Array<{
              name: string;
              description?: string;
              inputSchema?: Record<string, unknown>;
            }>
          )();
          if (Array.isArray(tools)) {
            for (const tool of tools) {
              if (tool.name && !discoveredToolNames.has(tool.name)) {
                notifyToolRegistered(
                  tool.name,
                  tool.description || '',
                  tool.inputSchema
                );
              }
            }
          }
        } catch {
          // getTools() may throw — skip this instance
        }
      }
    }
  }

  // ========================================================================
  // Layer 4: ModelContextTesting API (opportunistic)
  // ========================================================================

  /**
   * If Chrome's ModelContextTesting API is available (WebMCPTesting flag),
   * use its proper event/query APIs instead of monkey-patching.
   * Returns true if the testing API was set up successfully.
   */
  function setupModelContextTesting(mc: Record<string, unknown>): boolean {
    const testing = mc.testing as
      | {
          ontoolchange: ((event: unknown) => void) | null;
          listTools: () => Array<{
            name: string;
            description?: string;
            inputSchema?: Record<string, unknown>;
          }>;
          executeTool?: (
            name: string,
            args: string,
            ...rest: unknown[]
          ) => Promise<string | null>;
        }
      | undefined;

    if (!testing || typeof testing.listTools !== 'function') {
      return false;
    }

    // Store reference for invocation if executeTool is available
    if (typeof testing.executeTool === 'function') {
      const execFn = testing.executeTool;
      modelContextTesting = {
        executeTool: (
          name: string,
          args: string,
          ...rest: unknown[]
        ): Promise<string | null> => execFn.call(testing, name, args, ...rest),
      };
    }

    // Register ontoolchange for real-time notifications
    testing.ontoolchange = () => {
      try {
        const tools = testing.listTools();
        if (Array.isArray(tools)) {
          for (const tool of tools) {
            if (tool.name && !discoveredToolNames.has(tool.name)) {
              notifyToolRegistered(
                tool.name,
                tool.description || '',
                tool.inputSchema
              );
            }
          }
        }
      } catch {
        // listTools() may throw
      }
    };

    // Do an initial scan for already-registered tools
    try {
      const tools = testing.listTools();
      if (Array.isArray(tools)) {
        for (const tool of tools) {
          if (tool.name) {
            notifyToolRegistered(
              tool.name,
              tool.description || '',
              tool.inputSchema
            );
          }
        }
      }
    } catch {
      // listTools() may throw
    }

    return true;
  }

  // ========================================================================
  // Tool invocation listener
  // ========================================================================

  /**
   * Post an invocation result (success or error) back to the content script.
   */
  function postInvokeResult(
    correlationId: string,
    result: unknown,
    error?: string
  ): void {
    if (error !== undefined) {
      window.postMessage(
        { type: 'FSPEC_INVOKE_RESULT', correlationId, error },
        '*'
      );
    } else {
      window.postMessage(
        { type: 'FSPEC_INVOKE_RESULT', correlationId, result },
        '*'
      );
    }
  }

  /**
   * Resolve a possibly-async tool result and post it back.
   */
  function resolveAndPost(correlationId: string, result: unknown): void {
    if (result && typeof (result as Promise<unknown>).then === 'function') {
      (result as Promise<unknown>)
        .then(resolved => {
          postInvokeResult(correlationId, resolved);
        })
        .catch((err: Error) => {
          postInvokeResult(
            correlationId,
            undefined,
            err.message || String(err)
          );
        });
    } else {
      postInvokeResult(correlationId, result);
    }
  }

  /**
   * Listen for tool invocation requests from the content script.
   * The content script forwards FSPEC_INVOKE_TOOL messages via postMessage.
   *
   * Invocation priority:
   * 1. registeredTools map (callbacks captured from Layers 1, 2, and instance traps)
   * 2. ModelContextTesting.executeTool() (Layer 4)
   * 3. Snapshot instance executeTool() (Layer 3)
   * 4. Error: tool not found
   */
  function setupInvocationListener(): void {
    window.addEventListener('message', (event: MessageEvent) => {
      if (event.source !== window) {
        return;
      }

      const data = event.data as
        | {
            type?: string;
            correlationId?: string;
            toolName?: string;
            args?: Record<string, unknown>;
          }
        | undefined;
      if (!data || data.type !== 'FSPEC_INVOKE_TOOL') {
        return;
      }

      const { correlationId, toolName, args } = data;
      if (!correlationId || !toolName) {
        return;
      }

      // Priority 1: Direct callback from Layers 1, 2, or instance traps
      const executeFn = registeredTools.get(toolName);
      if (executeFn) {
        try {
          resolveAndPost(correlationId, executeFn(args ?? {}));
        } catch (err: unknown) {
          const message = err instanceof Error ? err.message : String(err);
          postInvokeResult(correlationId, undefined, message);
        }
        return;
      }

      // Priority 2: ModelContextTesting API (Layer 4)
      if (modelContextTesting) {
        modelContextTesting
          .executeTool(toolName, JSON.stringify(args ?? {}))
          .then(result => {
            postInvokeResult(correlationId, result);
          })
          .catch((err: Error) => {
            postInvokeResult(
              correlationId,
              undefined,
              err.message || String(err)
            );
          });
        return;
      }

      // Priority 3: Snapshot instances (Layer 3)
      for (const instance of snapshotInstances) {
        if (typeof instance.executeTool === 'function') {
          try {
            const result = (
              instance.executeTool as (
                name: string,
                args: Record<string, unknown>
              ) => unknown
            )(toolName, args ?? {});
            resolveAndPost(correlationId, result);
            return;
          } catch {
            // This instance couldn't handle it — try next
          }
        }
      }

      // No handler found
      postInvokeResult(
        correlationId,
        undefined,
        `Tool "${toolName}" not found in page context`
      );
    });
  }

  // ========================================================================
  // Initialize all layers
  // ========================================================================

  // Layer 1: navigator.modelContext (+ Layer 4 if testing API available)
  setupModelContextInterceptor();

  // Layer 2: WebMCP class prototype interception
  setupWebMCPClassInterceptor();

  // Tool invocation listener
  setupInvocationListener();

  // Layer 3: Post-load snapshot (delayed to let page scripts finish)
  setTimeout(() => {
    performPostLoadSnapshot();
  }, 500);
}
