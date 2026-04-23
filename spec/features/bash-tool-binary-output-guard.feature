@done
@tool-execution
@bug-fix
@tools
@BUG-142
Feature: Bash tool should guard against reading binary files (cat of PNG/PDF/etc) and return a structured error
  """
  Introduce a new module codelet/tools/src/bash_binary_guard.rs with a pure function `detect_bash_binary_output(stdout_bytes: &[u8]) -> Option<BinaryKind>` where BinaryKind is a small enum {Image(ImageMediaType), Pdf, Other}. Detection logic: (1) if stdout contains a NUL byte anywhere in the first N bytes (N=8192) → Some(Other) unless we also match a magic signature below; (2) if the first 16 bytes match PNG/JPEG/GIF/WebP → Some(Image(...)); (3) if first 5 bytes are '%PDF-' → Some(Pdf); (4) if first 2 bytes are 0x1F 0x8B (gzip), or first 4 bytes are 'PK\x03\x04' (zip), or first 4 bytes are 0x7F 'ELF' → Some(Other). Priority: magic signature wins over generic NUL-byte detection so we can name the type.
  Problem: codelet/tools/src/bash_output.rs uses String for stdout (already lossy to UTF-8) — we cannot reliably detect NUL bytes after stdout has been converted to a Rust String. Investigation needed: either (a) change StreamBuffers to hold Vec<u8> instead of String (intrusive), OR (b) accept that the upstream read/decode in bash_streams.rs already decodes to UTF-8 and use String-level detection (check for '\0' char and magic byte strings). Prefer (b) as less intrusive — Rust's String::contains('\x00') works on any UTF-8 string, and magic bytes are all ASCII-safe byte patterns that survive UTF-8 decoding when checked via .as_bytes().
  Integration point: BashOutput::into_result() in bash_output.rs becomes the single choke-point — both BashTool::call() and BashTool::call_with_streaming() route through into_result(). Modify into_result() to call detect_bash_binary_output(self.stdout.as_bytes()) FIRST; if Some(kind), short-circuit to Err(ToolError::Execution { tool: "bash", message: format_binary_guard_message(kind) }), regardless of self.success. Otherwise fall through to existing success/error formatting.
  Message format (deterministic, testable): for Image(mt) → 'Bash output suppressed: detected {PNG|JPEG|GIF|WebP} image. Use the Read tool on the file instead; the Bash tool does not return binary bytes to the model.'; for Pdf → 'Bash output suppressed: detected PDF document. Use the Read tool on the file instead...'; for Other → 'Bash output suppressed: detected binary content. Use the Read tool on the file instead...'. No file path in the message (we don't reliably know which file the user was catting — the caller has that context).
  Tests: unit tests for detect_bash_binary_output live inside bash_binary_guard.rs (#[cfg(test)] mod tests) — fast, deterministic, no I/O. Integration tests for end-to-end Bash execution live in a new codelet/tools/src/bash_binary_guard_integration_tests.rs that actually spawns `cat` / `printf` / `gzip -c` under BashTool::call and asserts the returned Result. Fixtures: create tiny PNG/PDF/ELF/ZIP bytes inline (no external files) using tempfile::NamedTempFile so tests are hermetic.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Binary output is detected when captured stdout contains NUL bytes (0x00) OR when its leading bytes match a known binary magic signature (PNG/JPEG/GIF/WebP/PDF/ELF/ZIP/GZIP), evaluated on the raw byte buffer before UTF-8 truncation
  #   2. When binary output is detected, the Bash tool MUST return a ToolError::Execution whose message is a structured, actionable directive: (a) names the offending command, (b) identifies the detected binary type when known (e.g. 'PNG image'), (c) instructs the agent to use the Read tool on the same file instead, and (d) does not forward any of the binary bytes to the model
  #   3. Detection applies to BOTH the non-streaming call() path and the call_with_streaming() path — in both cases the final buffered stdout is checked before being returned to the caller (streaming callbacks to the UI are not affected; only the returned buffered output is replaced)
  #   4. Detection applies ONLY to stdout — stderr content is left unchanged (stderr is expected to be text diagnostics, and some programs emit mixed stderr legitimately)
  #   5. Text output containing occasional high-bit UTF-8 sequences (emoji, accented chars, CJK) MUST NOT be classified as binary — detection uses NUL-byte presence AND magic-byte matching, not simple non-ASCII heuristics
  #   6. Binary detection runs regardless of exit status — a command that prints binary and exits 0 is still intercepted; a command that prints binary and fails (exit!=0) returns the binary-guard error instead of the usual format_error() (the guard takes precedence because forwarding binary garbage helps no one)
  #   7. If the detected binary magic signature maps to a known ImageMediaType or PDF, the error message explicitly names the type (e.g. 'detected PNG image' / 'detected PDF document') so the agent knows which Read mode to use; for unknown binaries (ELF/ZIP/generic) the message says 'detected binary content' without fabricating a type
  #   8. The guard is triggered regardless of which command produced the bytes — `cat /tmp/icon.png` → triggers; `base64 -d <<< 'iVBORw0...'` → triggers; `printf '\x00\x01\x02'` → triggers; `curl -o - binary.bin` → triggers. The mechanism is content-based, not command-name-based
  #
  # EXAMPLES:
  #   1. Given cat /tmp/icon.png (a valid PNG) succeeds with exit 0, when Bash tool returns, then the caller receives ToolError::Execution whose message contains 'detected PNG image' and 'use the Read tool instead' — the PNG bytes themselves are never in the returned output
  #   2. Given cat /tmp/doc.pdf (a valid PDF) succeeds with exit 0, when Bash tool returns, then the caller receives ToolError::Execution whose message contains 'detected PDF document' and 'use the Read tool instead'
  #   3. Given cat /bin/ls (an ELF binary) succeeds with exit 0, when Bash tool returns, then the caller receives ToolError::Execution whose message contains 'detected binary content' (no specific type since ELF is not a known image/pdf) and 'use the Read tool instead'
  #   4. Given echo 'hello 👋 world — accents: café' succeeds, when Bash tool returns, then output is returned normally (no false-positive binary detection from emoji/UTF-8 multi-byte chars)
  #   5. Given printf '\x00\x01\x02\x03hello' succeeds with exit 0 (raw bytes including a NUL), when Bash tool returns, then the caller receives ToolError::Execution with 'detected binary content' and 'use the Read tool instead' — the raw bytes are not forwarded
  #   6. Given cat /tmp/missing.png fails with exit 1 and stderr 'cat: /tmp/missing.png: No such file or directory' (no stdout), when Bash tool returns, then the caller receives the usual exit-code-1 ToolError::Execution with the stderr diagnostic preserved (no binary-guard triggered because stdout is empty)
  #   7. Given `hexdump -C /tmp/icon.png | head -5` succeeds and produces TEXT output (hex dump is text, not binary), when Bash tool returns, then the output is returned normally (the hexdump lines are text — no false positive even though the source file is binary)
  #   8. Given `gzip -c /tmp/big.log` succeeds with exit 0 and prints gzip-compressed bytes to stdout (magic: 0x1F 0x8B), when Bash tool returns, then the caller receives ToolError::Execution with 'detected binary content' and 'use the Read tool instead'
  #   9. Given `{ printf 'header\n'; cat /tmp/icon.png; }` succeeds with exit 0 (mixed text prefix then binary payload — stdout contains NUL bytes), when Bash tool returns, then the caller receives ToolError::Execution with 'detected binary content' and 'use the Read tool instead' (the text prefix is discarded because the payload is binary)
  #   10. Given call_with_streaming() executes `cat /tmp/icon.png` with a stream_callback, when the command completes with exit 0, then: (a) the stream_callback MAY have received intermediate chunks during execution (not guarded — UI streaming is out of scope), BUT (b) the FINAL returned Result is ToolError::Execution with the binary-guard message — the caller never sees the binary bytes in the buffered return value
  #
  # ========================================
  Background: User Story
    As a AI agent operating under a Rhai-scripted custom provider
    I want to have the Bash tool detect and refuse binary output instead of emitting garbled bytes to the model
    So that I get a clear, structured error telling me to use Read instead, preventing context-pollution and model confusion

  @unit
  @tools
  @bug-fix
  Scenario: PNG bytes on stdout trigger the image-aware binary guard
    Given a bash command prints PNG magic bytes (0x89 0x50 0x4E 0x47) followed by a PNG payload to stdout
    And the command exits with status 0
    When the Bash tool returns
    Then the caller receives a ToolError::Execution
    And the error message contains "detected PNG image"
    And the error message contains "Use the Read tool"
    And the error message does NOT contain any of the raw PNG bytes

  @unit
  @tools
  @bug-fix
  Scenario: JPEG bytes on stdout trigger the image-aware binary guard
    Given a bash command prints JPEG magic bytes (0xFF 0xD8 0xFF) followed by a JPEG payload to stdout
    And the command exits with status 0
    When the Bash tool returns
    Then the caller receives a ToolError::Execution
    And the error message contains "detected JPEG image"
    And the error message contains "Use the Read tool"

  @unit
  @tools
  @bug-fix
  Scenario: PDF bytes on stdout trigger the document-aware binary guard
    Given a bash command prints PDF magic bytes ("%PDF-1.4") to stdout
    And the command exits with status 0
    When the Bash tool returns
    Then the caller receives a ToolError::Execution
    And the error message contains "detected PDF document"
    And the error message contains "Use the Read tool"

  @unit
  @tools
  @bug-fix
  Scenario: ELF binary on stdout triggers the generic binary guard
    Given a bash command prints ELF magic bytes (0x7F 0x45 0x4C 0x46) to stdout
    And the command exits with status 0
    When the Bash tool returns
    Then the caller receives a ToolError::Execution
    And the error message contains "detected binary content"
    And the error message contains "Use the Read tool"
    And the error message does NOT contain the word "PNG"
    And the error message does NOT contain the word "PDF"

  @unit
  @tools
  @bug-fix
  Scenario: Gzip-compressed stdout triggers the generic binary guard
    Given a bash command prints gzip magic bytes (0x1F 0x8B) followed by compressed payload to stdout
    And the command exits with status 0
    When the Bash tool returns
    Then the caller receives a ToolError::Execution
    And the error message contains "detected binary content"
    And the error message contains "Use the Read tool"

  @unit
  @tools
  @bug-fix
  Scenario: Raw NUL bytes in stdout trigger the generic binary guard
    Given a bash command prints "\x00\x01\x02\x03hello" to stdout (bytes with an embedded NUL)
    And the command exits with status 0
    When the Bash tool returns
    Then the caller receives a ToolError::Execution
    And the error message contains "detected binary content"
    And the error message does NOT contain the raw bytes

  @unit
  @tools
  @bug-fix
  Scenario: Plain text with emoji and high-bit UTF-8 is NOT flagged as binary
    Given a bash command prints "hello 👋 world — café résumé" to stdout
    And the command exits with status 0
    When the Bash tool returns
    Then the caller receives Ok containing the original text
    And the returned string equals the command's stdout unchanged

  @unit
  @tools
  @bug-fix
  Scenario: hexdump output of a binary file is text and passes through unchanged
    Given a bash command pipeline produces canonical hexdump text (e.g. "00000000  89 50 4e 47 0d 0a 1a 0a  |.PNG....|")
    And the command exits with status 0
    When the Bash tool returns
    Then the caller receives Ok containing the hexdump lines unchanged
    And no binary guard is triggered

  @unit
  @tools
  @bug-fix
  Scenario: Missing-file failure preserves stderr diagnostic and does NOT trigger the guard
    Given a bash command fails with exit code 1
    And stdout is empty
    And stderr contains "cat: /tmp/missing.png: No such file or directory"
    When the Bash tool returns
    Then the caller receives a ToolError::Execution
    And the error message contains "exit code 1"
    And the error message contains the stderr diagnostic
    And the error message does NOT contain "detected binary content"
    And the error message does NOT contain "Use the Read tool"

  @unit
  @tools
  @bug-fix
  Scenario: Text prefix followed by PNG payload is intercepted by the guard
    Given a bash command prints "header\n" then a PNG payload to stdout (mixed text-then-binary)
    And the command exits with status 0
    When the Bash tool returns
    Then the caller receives a ToolError::Execution
    And the error message contains "detected PNG image"
    And the error message does NOT contain the text prefix "header"

  @integration
  @tools
  @bug-fix
  Scenario: call_with_streaming replaces the final buffered return value with the guard error
    Given a stream_callback is provided to call_with_streaming
    And a bash command prints PNG bytes to stdout
    And the command exits with status 0
    When call_with_streaming returns
    Then the buffered Result returned to the caller is a ToolError::Execution
    And the error message contains "detected PNG image"
    And the error message contains "Use the Read tool"

  @unit
  @tools
  @bug-fix
  Scenario: Binary payload combined with a non-zero exit status still returns the guard error
    Given a bash command prints PNG bytes to stdout
    And the command exits with status 2
    When the Bash tool returns
    Then the caller receives a ToolError::Execution
    And the error message contains "detected PNG image"
    And the error message does NOT contain "exit code 2"
