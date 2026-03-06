/**
 * fspec WebMCP Extension - Main-World Discovery Script
 *
 * This function runs in the page's MAIN world JavaScript context
 * (injected via chrome.scripting.executeScript with world: 'MAIN').
 *
 * It intercepts navigator.modelContext.registerTool() and
 * unregisterTool() calls to detect WebMCP tool registrations,
 * and handles tool invocation requests from the extension.
 *
 * Communication is exclusively via window.postMessage with
 * FSPEC_ prefixed message types.
 *
 * Implemented by: EXT-006
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
  const registeredTools = new Map<string, (args: Record<string, unknown>) => unknown>();

  /**
   * Intercept navigator.modelContext to detect tool registrations.
   *
   * The WebMCP API (Chrome 146+) provides:
   *   navigator.modelContext.registerTool(toolDef)
   *   navigator.modelContext.unregisterTool(toolName)
   *
   * We monkey-patch these methods to capture tool metadata.
   */
  function setupModelContextInterceptor(): void {
    const nav = navigator as Record<string, unknown>;

    // If navigator.modelContext already exists, intercept it
    if (nav.modelContext) {
      interceptExistingModelContext(nav.modelContext as Record<string, unknown>);
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
    const originalRegister = mc.registerTool as
      | ((toolDef: Record<string, unknown>) => unknown)
      | undefined;
    const originalUnregister = mc.unregisterTool as
      | ((name: string) => unknown)
      | undefined;

    mc.registerTool = function (toolDef: Record<string, unknown>): unknown {
      const name = toolDef.name as string;
      const description = (toolDef.description as string) || '';
      const inputSchema = toolDef.inputSchema as Record<string, unknown> | undefined;
      const executeFn = toolDef.execute as
        | ((args: Record<string, unknown>) => unknown)
        | undefined;

      // Store execute function for later invocation
      if (executeFn) {
        registeredTools.set(name, executeFn);
      }

      // Notify extension via postMessage
      window.postMessage(
        {
          type: 'FSPEC_WEBMCP_TOOL_REGISTERED',
          tool: { name, description, inputSchema },
          origin: pageOrigin,
        },
        '*'
      );

      // Call original if it exists
      if (originalRegister) {
        return originalRegister.call(mc, toolDef);
      }
      return undefined;
    };

    mc.unregisterTool = function (name: string): unknown {
      registeredTools.delete(name);

      // Notify extension via postMessage
      window.postMessage(
        {
          type: 'FSPEC_WEBMCP_TOOL_UNREGISTERED',
          toolName: name,
          origin: pageOrigin,
        },
        '*'
      );

      // Call original if it exists
      if (originalUnregister) {
        return originalUnregister.call(mc, name);
      }
      return undefined;
    };
  }

  /**
   * Listen for tool invocation requests from the content script.
   * The content script forwards FSPEC_INVOKE_TOOL messages via postMessage.
   */
  function setupInvocationListener(): void {
    window.addEventListener('message', (event: MessageEvent) => {
      if (event.source !== window) {
        return;
      }

      const data = event.data as { type?: string; correlationId?: string; toolName?: string; args?: Record<string, unknown> } | undefined;
      if (!data || data.type !== 'FSPEC_INVOKE_TOOL') {
        return;
      }

      const { correlationId, toolName, args } = data;
      if (!correlationId || !toolName) {
        return;
      }

      const executeFn = registeredTools.get(toolName);
      if (!executeFn) {
        window.postMessage(
          {
            type: 'FSPEC_INVOKE_RESULT',
            correlationId,
            error: `Tool "${toolName}" not found in page context`,
          },
          '*'
        );
        return;
      }

      // Execute the tool and handle both sync and async results
      try {
        const result = executeFn(args ?? {});

        // Handle Promise results
        if (result && typeof (result as Promise<unknown>).then === 'function') {
          (result as Promise<unknown>)
            .then((resolved) => {
              window.postMessage(
                {
                  type: 'FSPEC_INVOKE_RESULT',
                  correlationId,
                  result: resolved,
                },
                '*'
              );
            })
            .catch((err: Error) => {
              window.postMessage(
                {
                  type: 'FSPEC_INVOKE_RESULT',
                  correlationId,
                  error: err.message || String(err),
                },
                '*'
              );
            });
        } else {
          window.postMessage(
            {
              type: 'FSPEC_INVOKE_RESULT',
              correlationId,
              result,
            },
            '*'
          );
        }
      } catch (err: unknown) {
        const message = err instanceof Error ? err.message : String(err);
        window.postMessage(
          {
            type: 'FSPEC_INVOKE_RESULT',
            correlationId,
            error: message,
          },
          '*'
        );
      }
    });
  }

  // Initialize
  setupModelContextInterceptor();
  setupInvocationListener();
}
