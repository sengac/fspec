@done
@attachment-viewer
@viewer
@RPC-372
Feature: Axum attachment viewer HTTP server with markdown rendering

  """
  New workspace crate codelet/attachment-viewer follows the fspec.pro axum architecture. lib.rs exposes build_router(state)/build_router_with_config(cfg), ViewerHandle, start_viewer(cwd)->Result<ViewerHandle> and ViewerHandle::stop(). state.rs defines ViewerState as a Clone newtype over Arc<Inner{cwd}>, injected via the axum State extractor. Routes: GET /view/{*path} and GET /health, wrapped with CorsLayer::permissive() and TraceLayer::new_for_http(). The view handler percent-decodes the path, lexically normalizes it relative to cwd, rejects traversal outside cwd with 403, renders .md/.markdown via render_markdown+viewer_template (text/html), and serves other extensions raw with a content-type map (png/jpeg/gif/svg/pdf/txt; default application/octet-stream). Missing file -> 404, other errors -> 500, never panics. render_markdown uses pulldown-cmark (GFM) wrapping mermaid blocks as <pre class="mermaid"> and other code blocks as <pre class="code-block" data-language=...> with HTML-escaped content. viewer_template returns a full HTML doc including the mermaid CDN script. start_viewer binds 127.0.0.1:0 and serves with axum::serve(...).with_graceful_shutdown(rx). All files <300 lines. Browser launching and key wiring are out of scope (RPC-373/RPC-374).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Markdown files (.md/.markdown) are rendered to an HTML viewer page; all other files are served raw with a content-type based on extension
  #   2. Mermaid fenced code blocks render to <pre class="mermaid"> for client-side rendering; other code blocks render to <pre class="code-block" data-language=...> with escaped content
  #   3. Requested paths are resolved relative to cwd and must stay within cwd; traversal outside cwd returns 403
  #   4. A missing file returns 404; any other read/processing error returns 500; the request path never panics
  #   5. start_viewer binds 127.0.0.1 on a random port and returns a handle exposing that port; stop() shuts the server down cleanly
  #   6. The HTTP server follows the fspec.pro axum architecture: build_router/build_router_with_config factory, Clone state newtype over Arc injected via State extractor, CorsLayer::permissive + TraceLayer layers
  #
  # EXAMPLES:
  #   1. GET /view/spec/attachments/RPC-001/design.md returns 200 text/html with the rendered heading and the basename in the title
  #   2. A markdown file containing a ```mermaid fenced block renders <pre class="mermaid"> in the HTML output
  #   3. GET /view/logo.png returns 200 image/png with the raw image bytes
  #   4. GET /view/../../etc/passwd returns 403 Forbidden
  #   5. GET /view/missing.md returns 404 File not found
  #   6. GET /health returns 200 ok
  #   7. start_viewer(tempdir) returns a handle whose port is non-zero and reachable on /health; after stop() the server task ends
  #
  # ========================================

  Background: User Story
    As a fspec TUI developer
    I want to serve card attachments and FOUNDATION.md through a local HTTP viewer that renders markdown (incl. mermaid) and serves images/pdf raw
    So that the board's A and D keys can open rich, browser-rendered documents safely scoped to the project directory

  Scenario: Render a markdown attachment as an HTML page
    Given a viewer server bound to a project directory containing a markdown file with a heading
    When I request that markdown file under the /view path
    Then the response status is 200
    And the Content-Type is text/html
    And the body contains the rendered heading
    And the document title is the file basename

  Scenario: Render mermaid code blocks for client-side rendering
    Given a viewer server bound to a project directory containing a markdown file with a mermaid fenced code block
    When I request that markdown file under the /view path
    Then the response status is 200
    And the body contains a pre element with class mermaid

  Scenario: Serve a binary image attachment raw with the correct content-type
    Given a viewer server bound to a project directory containing a PNG image
    When I request that image under the /view path
    Then the response status is 200
    And the Content-Type is image/png
    And the body is the raw image bytes

  Scenario: Block directory traversal outside the project directory
    Given a viewer server bound to a project directory
    When I request a path that traverses above the project directory under /view
    Then the response status is 403

  Scenario: Return not found for a missing file
    Given a viewer server bound to a project directory
    When I request a markdown file that does not exist under /view
    Then the response status is 404

  Scenario: Report health
    Given a viewer server bound to a project directory
    When I request the /health endpoint
    Then the response status is 200
    And the body indicates the server is ok

  Scenario: Start on a random local port and stop cleanly
    Given a project directory
    When I start the viewer server for that directory
    Then the returned handle exposes a non-zero port
    And a request to /health on that port succeeds
    And after I stop the handle the server task ends
