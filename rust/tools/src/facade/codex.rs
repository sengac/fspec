//! Codex-specific tool facades.
//!
//! These facades adapt the internal tool interfaces for Codex (GPT-5.1-codex) models,
//! using the exact tool names and parameter schemas from the Codex CLI
//! (codex-rs/core/src/tools/spec.rs).
//!
//! Codex models are trained on:
//! - `shell_command` (not `Bash` or `run_command`)
//! - `read_file` with `file_path` parameter
//! - `list_dir` with `dir_path` parameter (not `path`)
//! - `grep_files` with `pattern`, `include`, `path`, `limit` parameters
//!
//! Feature: spec/features/codex-native-tool-facades.feature

use super::param_extract::{
    extract_optional_bool, extract_optional_string, extract_optional_uint, extract_required_string,
};
use super::traits::{
    BashToolFacade, ExecToolFacade, FileToolFacade, HitlToolFacade, InternalBashParams,
    InternalExecParams, InternalFileParams, InternalHitlParams, InternalIndentationParams,
    InternalLsParams, InternalSearchParams, LsToolFacade, SearchToolFacade, ToolDefinition,
};
use crate::ToolError;
use serde_json::{json, Value};

// ============================================================================
// Shell Command Facade
// ============================================================================

/// Codex-specific facade for shell command execution.
///
/// Maps Codex's `shell_command` tool to the internal BashTool parameters.
/// The Codex CLI defines `shell_command` with:
/// - `command` (required): The shell script to execute
/// - `workdir` (optional): Working directory for execution
/// - `timeout_ms` (optional): Timeout in milliseconds
/// - `login` (optional): Whether to run with login shell semantics
/// - `sandbox_permissions` (optional): Sandbox escalation control
/// - `justification` (optional): Approval justification
/// - `prefix_rule` (optional): Permission prefix pattern
///
/// The `login`, `sandbox_permissions`, `justification`, and `prefix_rule` params
/// are accepted in the schema for model compatibility but silently ignored.
pub struct CodexShellCommandFacade;

impl BashToolFacade for CodexShellCommandFacade {
    fn provider(&self) -> &'static str {
        "codex"
    }

    fn tool_name(&self) -> &'static str {
        "shell_command"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "shell_command".to_string(),
            description: "Execute a shell command in the user's default shell. Returns stdout on success, stderr on failure.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell script to execute in the user's default shell"
                    },
                    "workdir": {
                        "type": "string",
                        "description": "The working directory to execute the command in"
                    },
                    "timeout_ms": {
                        "type": "integer",
                        "description": "The timeout for the command in milliseconds"
                    },
                    "login": {
                        "type": "boolean",
                        "description": "Whether to run the shell with login shell semantics. Defaults to true."
                    },
                    "sandbox_permissions": {
                        "type": "string",
                        "description": "Sandbox permissions for the command. Defaults to \"use_default\"."
                    },
                    "justification": {
                        "type": "string",
                        "description": "Approval justification when sandbox_permissions is \"require_escalated\"."
                    },
                    "prefix_rule": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Permission prefix pattern for escalated sandbox permissions."
                    }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalBashParams, ToolError> {
        let command = extract_required_string(&input, "command", "shell_command")?;
        let cwd = extract_optional_string(&input, "workdir");
        let timeout_ms = input
            .get("timeout_ms")
            .and_then(crate::facade::param_extract::value_as_u64_lenient);
        Ok(InternalBashParams::Execute {
            command,
            cwd,
            timeout_ms,
        })
    }
}

// ============================================================================
// Read File Facade
// ============================================================================

/// Codex-specific facade for file reading.
///
/// Maps Codex's `read_file` tool to the internal ReadTool parameters.
/// The Codex CLI defines `read_file` with:
/// - `file_path` (required): Absolute path to the file
/// - `offset` (optional): Line number to start reading from (1-based)
/// - `limit` (optional): Maximum number of lines to return
/// - `mode` (optional): Mode selector — "slice" (default) or "indentation"
/// - `indentation` (optional): Nested object for indentation-aware block reading
pub struct CodexReadFileFacade;

impl FileToolFacade for CodexReadFileFacade {
    fn provider(&self) -> &'static str {
        "codex"
    }

    fn tool_name(&self) -> &'static str {
        "read_file"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "read_file".to_string(),
            description: "Reads a local file with 1-indexed line numbers, supporting slice and indentation-aware block modes.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to the file to read"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "The line number to start reading from. Must be 1 or greater."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "The maximum number of lines to return."
                    },
                    "mode": {
                        "type": "string",
                        "description": "Optional mode selector: \"slice\" for simple ranges (default) or \"indentation\" to expand around an anchor line."
                    },
                    "indentation": {
                        "type": "object",
                        "properties": {
                            "anchor_line": {
                                "type": "integer",
                                "description": "Anchor line to center the indentation lookup on (defaults to offset)."
                            },
                            "max_levels": {
                                "type": "integer",
                                "description": "How many parent indentation levels (smaller indents) to include."
                            },
                            "include_siblings": {
                                "type": "boolean",
                                "description": "When true, include additional blocks that share the anchor indentation."
                            },
                            "include_header": {
                                "type": "boolean",
                                "description": "Include doc comments or attributes directly above the selected block."
                            },
                            "max_lines": {
                                "type": "integer",
                                "description": "Hard cap on the number of lines returned when using indentation mode."
                            }
                        },
                        "additionalProperties": false
                    }
                },
                "required": ["file_path"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalFileParams, ToolError> {
        let file_path = extract_required_string(&input, "file_path", "read_file")?;
        let offset = extract_optional_uint(&input, "offset");
        let limit = extract_optional_uint(&input, "limit");
        let mode = extract_optional_string(&input, "mode");

        // Extract indentation sub-object if present
        let indentation = input.get("indentation").and_then(|v| {
            if v.is_object() {
                Some(InternalIndentationParams {
                    anchor_line: extract_optional_uint(v, "anchor_line"),
                    max_levels: extract_optional_uint(v, "max_levels"),
                    include_siblings: extract_optional_bool(v, "include_siblings"),
                    include_header: extract_optional_bool(v, "include_header"),
                    max_lines: extract_optional_uint(v, "max_lines"),
                })
            } else {
                None
            }
        });

        Ok(InternalFileParams::Read {
            file_path,
            offset,
            limit,
            mode,
            indentation,
        })
    }
}

// ============================================================================
// List Directory Facade
// ============================================================================

/// Codex-specific facade for directory listing.
///
/// Maps Codex's `list_dir` tool to the internal LsTool parameters.
/// The Codex CLI defines `list_dir` with:
/// - `dir_path` (required): Absolute path to the directory to list
/// - `offset` (optional): Entry number to start listing from
/// - `limit` (optional): Maximum number of entries to return
/// - `depth` (optional): Maximum directory depth to traverse
///
/// Note: Unlike Z.AI's `list_dir` which uses `path`, Codex uses `dir_path`.
pub struct CodexListDirFacade;

impl LsToolFacade for CodexListDirFacade {
    fn provider(&self) -> &'static str {
        "codex"
    }

    fn tool_name(&self) -> &'static str {
        "list_dir"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "list_dir".to_string(),
            description: "Lists entries in a local directory with 1-indexed entry numbers and simple type labels.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "dir_path": {
                        "type": "string",
                        "description": "Absolute path to the directory to list."
                    },
                    "offset": {
                        "type": "integer",
                        "description": "The entry number to start listing from. Must be 1 or greater."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "The maximum number of entries to return."
                    },
                    "depth": {
                        "type": "integer",
                        "description": "The maximum directory depth to traverse. Must be 1 or greater."
                    }
                },
                "required": ["dir_path"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalLsParams, ToolError> {
        // Codex uses `dir_path`, not `path`
        let dir_path = extract_optional_string(&input, "dir_path");
        let offset = extract_optional_uint(&input, "offset");
        let limit = extract_optional_uint(&input, "limit");
        let depth = extract_optional_uint(&input, "depth");
        Ok(InternalLsParams::List {
            path: dir_path,
            offset,
            limit,
            depth,
        })
    }
}

// ============================================================================
// View Image Facade
// ============================================================================

/// Codex-specific facade for viewing image files.
///
/// Maps Codex's `view_image` tool to the internal ReadTool via `InternalFileParams::Read`.
/// The Codex CLI defines `view_image` with:
/// - `path` (required): Local filesystem path to an image file
///
/// This facade maps `view_image { path }` → `InternalFileParams::Read { file_path: path }`
/// so it delegates to ReadTool, which already handles image files (PNG, JPEG, GIF, WEBP)
/// by detecting file type and returning base64-encoded image data.
///
/// The `detail` parameter from the original Codex CLI is accepted in the schema
/// for model compatibility but not used (our ReadTool always returns the full image).
pub struct CodexViewImageFacade;

impl FileToolFacade for CodexViewImageFacade {
    fn provider(&self) -> &'static str {
        "codex"
    }

    fn tool_name(&self) -> &'static str {
        "view_image"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "view_image".to_string(),
            description:
                "View a local image from the filesystem (only use if given a full filepath \
                by the user, and the image isn't already attached to the thread context \
                within <image ...> tags)."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Local filesystem path to an image file"
                    },
                    "detail": {
                        "type": "string",
                        "description": "Optional detail override. The only supported value is `original`; omit this field for default resized behavior."
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalFileParams, ToolError> {
        let path = extract_required_string(&input, "path", "view_image")?;
        // detail is accepted for model compatibility but not used —
        // ReadTool always returns the full image as base64
        Ok(InternalFileParams::Read {
            file_path: path,
            offset: None,
            limit: None,
            mode: None,
            indentation: None,
        })
    }
}

// ============================================================================
// Grep Files Facade
// ============================================================================

/// Codex-specific facade for file content searching.
///
/// Maps Codex's `grep_files` tool to the internal GrepTool parameters.
/// The Codex CLI defines `grep_files` with:
/// - `pattern` (required): Regular expression pattern to search for
/// - `include` (optional): Glob that limits which files are searched (e.g., '*.rs')
/// - `path` (optional): Directory or file path to search
/// - `limit` (optional): Maximum number of file paths to return
pub struct CodexGrepFilesFacade;

impl SearchToolFacade for CodexGrepFilesFacade {
    fn provider(&self) -> &'static str {
        "codex"
    }

    fn tool_name(&self) -> &'static str {
        "grep_files"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "grep_files".to_string(),
            description: "Search file contents using regex pattern. Returns matching lines with file paths and line numbers. Supports glob filters to limit which files are searched.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regular expression pattern to search for."
                    },
                    "include": {
                        "type": "string",
                        "description": "Optional glob that limits which files are searched (e.g. '*.rs')"
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory or file path to search. Defaults to session working directory."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of file paths to return (defaults to 100)."
                    }
                },
                "required": ["pattern"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalSearchParams, ToolError> {
        let pattern = extract_required_string(&input, "pattern", "grep_files")?;
        let path = extract_optional_string(&input, "path");
        let include = extract_optional_string(&input, "include");
        let limit = extract_optional_uint(&input, "limit");

        Ok(InternalSearchParams::Grep {
            pattern,
            path,
            include,
            limit,
        })
    }
}

// ============================================================================
// Shell Facade (BUG-114: execvp-style, no shell interpretation)
// ============================================================================

/// Codex-specific facade for raw shell execution (execvp-style).
///
/// Maps Codex's `shell` tool to the unified exec tool via `InternalExecParams::Run`.
/// The Codex CLI defines `shell` with:
/// - `command` (required): Array of strings passed to execvp()
/// - `workdir` (optional): Working directory for execution
/// - `timeout_ms` (optional): Timeout in milliseconds (converted to seconds for unified exec)
///
/// Unlike `shell_command` (which uses BashToolFacade for one-shot commands),
/// `shell` always passes command as an argv array with no shell interpretation.
///
/// Feature: spec/features/codex-shell-exec-facades.feature
pub struct CodexShellFacade;

impl ExecToolFacade for CodexShellFacade {
    fn provider(&self) -> &'static str {
        "codex"
    }

    fn tool_name(&self) -> &'static str {
        "shell"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "shell".to_string(),
            description: "Execute a command via execvp (no shell interpretation). The command is an array of strings passed directly as argv.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "argv passed to execvp()"
                    },
                    "workdir": {
                        "type": "string",
                        "description": "The working directory to execute the command in"
                    },
                    "timeout_ms": {
                        "type": "number",
                        "description": "The timeout for the command in milliseconds"
                    }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalExecParams, ToolError> {
        // command is required and must be an array of strings
        let command_val = input.get("command").ok_or_else(|| ToolError::Validation {
            tool: "shell",
            message: "Missing required parameter: command".to_string(),
        })?;
        if !command_val.is_array() {
            return Err(ToolError::Validation {
                tool: "shell",
                message: "command must be an array of strings".to_string(),
            });
        }
        // Validate the array is non-empty
        if let Some(arr) = command_val.as_array() {
            if arr.is_empty() {
                return Err(ToolError::Validation {
                    tool: "shell",
                    message: "command array must not be empty".to_string(),
                });
            }
        }

        let workdir = extract_optional_string(&input, "workdir");
        let timeout_ms = input
            .get("timeout_ms")
            .and_then(crate::facade::param_extract::value_as_u64_lenient);
        let timeout_secs = timeout_ms.map(|ms| ms / 1000);

        Ok(InternalExecParams::Run {
            command: command_val.clone(),
            workdir,
            tty: false,
            yield_time_ms: None,
            max_output_tokens: None,
            timeout_secs,
        })
    }
}

// ============================================================================
// Exec Command Facade (BUG-114: PTY-capable unified exec)
// ============================================================================

/// Codex-specific facade for PTY-capable command execution.
///
/// Maps Codex's `exec_command` tool to the unified exec tool via `InternalExecParams::Run`.
/// The Codex CLI defines `exec_command` with:
/// - `cmd` (required): Shell command to execute (string)
/// - `workdir` (optional): Working directory for execution
/// - `shell` (optional): Shell binary to use (accepted but ignored)
/// - `tty` (optional): Allocate PTY (default false)
/// - `yield_time_ms` (optional): Wait time before yielding
/// - `max_output_tokens` (optional): Max output tokens
/// - `login` (optional): Login shell semantics (accepted but ignored)
///
/// The `shell` and `login` params are accepted in the schema for model compatibility
/// but silently ignored.
///
/// Feature: spec/features/codex-shell-exec-facades.feature
pub struct CodexExecCommandFacade;

impl ExecToolFacade for CodexExecCommandFacade {
    fn provider(&self) -> &'static str {
        "codex"
    }

    fn tool_name(&self) -> &'static str {
        "exec_command"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "exec_command".to_string(),
            description: "Execute a shell command with optional PTY allocation. Returns session_id when the process is still running for follow-up via write_stdin.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "cmd": {
                        "type": "string",
                        "description": "Shell command to execute"
                    },
                    "workdir": {
                        "type": "string",
                        "description": "The working directory to execute the command in"
                    },
                    "shell": {
                        "type": "string",
                        "description": "Shell binary to use"
                    },
                    "tty": {
                        "type": "boolean",
                        "description": "Allocate PTY. Defaults to false."
                    },
                    "yield_time_ms": {
                        "type": "number",
                        "description": "Wait time in milliseconds before yielding control back"
                    },
                    "max_output_tokens": {
                        "type": "number",
                        "description": "Maximum number of output tokens"
                    },
                    "login": {
                        "type": "boolean",
                        "description": "Whether to run with login shell semantics"
                    }
                },
                "required": ["cmd"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalExecParams, ToolError> {
        let cmd = extract_required_string(&input, "cmd", "exec_command")?;
        let workdir = extract_optional_string(&input, "workdir");
        let tty = extract_optional_bool(&input, "tty").unwrap_or(false);
        let yield_time_ms = input
            .get("yield_time_ms")
            .and_then(crate::facade::param_extract::value_as_u64_lenient);
        let max_output_tokens = input
            .get("max_output_tokens")
            .and_then(crate::facade::param_extract::value_as_u64_lenient);
        // shell and login params are silently ignored

        Ok(InternalExecParams::Run {
            command: Value::String(cmd),
            workdir,
            tty,
            yield_time_ms,
            max_output_tokens,
            timeout_secs: None,
        })
    }
}

// ============================================================================
// Write Stdin Facade (BUG-115: write to running PTY session)
// ============================================================================

/// Codex-specific facade for writing to running PTY sessions.
///
/// Maps Codex's `write_stdin` tool to the unified exec tool via
/// `InternalExecParams::Write` (non-empty chars) or `InternalExecParams::Poll` (empty/absent chars).
/// The Codex CLI defines `write_stdin` with:
/// - `session_id` (required): Hex string ID of running session from exec_command
/// - `chars` (optional): Characters to write — empty or absent means poll
/// - `yield_time_ms` (optional): Wait time for output
/// - `max_output_tokens` (optional): Max output tokens
///
/// The session_id is a hex UUID segment string (e.g. "05e8aa9c") returned by exec_command.
/// Empty `chars` or missing `chars` field triggers poll action instead of write.
///
/// Feature: spec/features/codex-write-stdin-facade.feature
pub struct CodexWriteStdinFacade;

impl ExecToolFacade for CodexWriteStdinFacade {
    fn provider(&self) -> &'static str {
        "codex"
    }

    fn tool_name(&self) -> &'static str {
        "write_stdin"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "write_stdin".to_string(),
            description: "Send input to a running session's stdin and poll for output. Empty or absent chars polls without sending input.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Session ID returned by exec_command"
                    },
                    "chars": {
                        "type": "string",
                        "description": "Characters to write to stdin. Empty or absent = poll for output only."
                    },
                    "yield_time_ms": {
                        "type": "number",
                        "description": "Wait time in milliseconds for output"
                    },
                    "max_output_tokens": {
                        "type": "number",
                        "description": "Maximum number of output tokens"
                    }
                },
                "required": ["session_id"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalExecParams, ToolError> {
        // session_id is required and must be a string (hex UUID segment from exec_command)
        let session_id_val = input
            .get("session_id")
            .ok_or_else(|| ToolError::Validation {
                tool: "write_stdin",
                message: "Missing required parameter: session_id".to_string(),
            })?;
        let session_id = match session_id_val {
            Value::String(s) => {
                if s.is_empty() {
                    return Err(ToolError::Validation {
                        tool: "write_stdin",
                        message: "session_id must not be empty".to_string(),
                    });
                }
                s.clone()
            }
            Value::Null => {
                return Err(ToolError::Validation {
                    tool: "write_stdin",
                    message: "Missing required parameter: session_id".to_string(),
                });
            }
            _ => {
                return Err(ToolError::Validation {
                    tool: "write_stdin",
                    message: "session_id must be a string".to_string(),
                });
            }
        };

        let chars = extract_optional_string(&input, "chars").unwrap_or_default();
        let yield_time_ms = input
            .get("yield_time_ms")
            .and_then(crate::facade::param_extract::value_as_u64_lenient);
        let max_output_tokens = input
            .get("max_output_tokens")
            .and_then(crate::facade::param_extract::value_as_u64_lenient);

        // Empty chars = poll, non-empty chars = write
        if chars.is_empty() {
            Ok(InternalExecParams::Poll {
                session_id,
                yield_time_ms,
                max_output_tokens,
            })
        } else {
            Ok(InternalExecParams::Write {
                session_id,
                input: chars,
                yield_time_ms,
                max_output_tokens,
            })
        }
    }
}

// ============================================================================
// Request User Input Facade (BUG-116: maps to HITL tool)
// ============================================================================

/// Codex-specific facade for requesting structured user input.
///
/// Maps Codex's `request_user_input` tool to the provider-agnostic HITL tool (TOOL-017).
/// The Codex CLI defines `request_user_input` with:
/// - `questions` (required): Array of 1-3 questions with id, header, question, options
///
/// The Codex schema is structurally identical to the HITL tool schema, so the
/// questions array passes through unchanged. The facade's role is:
/// 1. Present the tool definition with `additionalProperties: false` (Codex convention)
/// 2. Convert cancellation from the HITL handler to a Codex-specific error message
///
/// Feature: spec/features/codex-request-user-input-facade.feature
pub struct CodexRequestUserInputFacade;

impl HitlToolFacade for CodexRequestUserInputFacade {
    fn provider(&self) -> &'static str {
        "codex"
    }

    fn tool_name(&self) -> &'static str {
        "request_user_input"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "request_user_input".to_string(),
            description: "Request structured input from the user. Presents a modal with \
                1-3 questions, each with optional multiple-choice options and freeform text \
                input. The agent loop pauses until the user responds. Use when you need user \
                preferences, decisions, or clarifications that cannot be inferred from context."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "questions": {
                        "type": "array",
                        "description": "Array of 1-3 questions to present to the user.",
                        "minItems": 1,
                        "maxItems": 3,
                        "items": {
                            "type": "object",
                            "required": ["id", "header", "question"],
                            "properties": {
                                "id": {
                                    "type": "string",
                                    "description": "Stable snake_case identifier for mapping answers."
                                },
                                "header": {
                                    "type": "string",
                                    "description": "Short label shown in UI (max 12 chars)."
                                },
                                "question": {
                                    "type": "string",
                                    "description": "Single-sentence prompt shown to user."
                                },
                                "options": {
                                    "type": "array",
                                    "description": "Optional mutually exclusive choices (2-3 items).",
                                    "minItems": 2,
                                    "maxItems": 3,
                                    "items": {
                                        "type": "object",
                                        "required": ["label", "description"],
                                        "properties": {
                                            "label": {
                                                "type": "string",
                                                "description": "1-5 word label."
                                            },
                                            "description": {
                                                "type": "string",
                                                "description": "One sentence explaining impact."
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                "required": ["questions"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalHitlParams, ToolError> {
        use crate::request_user_input::{HitlOption, HitlQuestion};

        let questions_val = input
            .get("questions")
            .ok_or_else(|| ToolError::Validation {
                tool: "request_user_input",
                message: "Missing required parameter: questions".to_string(),
            })?;

        let questions_arr = questions_val
            .as_array()
            .ok_or_else(|| ToolError::Validation {
                tool: "request_user_input",
                message: "questions must be an array".to_string(),
            })?;

        let mut questions = Vec::new();
        for q in questions_arr {
            let id = q
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let header = q
                .get("header")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let question = q
                .get("question")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();

            let options = q.get("options").and_then(|opts_val| {
                opts_val.as_array().map(|opts_arr| {
                    opts_arr
                        .iter()
                        .map(|o| HitlOption {
                            label: o
                                .get("label")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string(),
                            description: o
                                .get("description")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string(),
                        })
                        .collect::<Vec<_>>()
                })
            });

            questions.push(HitlQuestion {
                id,
                header,
                question,
                options,
            });
        }

        Ok(InternalHitlParams::Request { questions })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    // =========================================================================
    // BUG-116: request_user_input facade tests (HitlToolFacade)
    // Feature: spec/features/codex-request-user-input-facade.feature
    // =========================================================================

    /// Scenario: CodexRequestUserInputFacade maps questions to InternalHitlParams and returns answers
    #[test]
    fn test_codex_request_user_input_maps_questions() {
        // @step Given a CodexRequestUserInputFacade instance
        let facade = CodexRequestUserInputFacade;

        // @step When the Codex model calls request_user_input with 2 questions each having 2 options
        let input = json!({
            "questions": [
                {
                    "id": "approach",
                    "header": "Approach",
                    "question": "Which approach do you prefer?",
                    "options": [
                        { "label": "Option A", "description": "First choice" },
                        { "label": "Option B", "description": "Second choice" }
                    ]
                },
                {
                    "id": "priority",
                    "header": "Priority",
                    "question": "What is the priority?",
                    "options": [
                        { "label": "High", "description": "Do it now" },
                        { "label": "Low", "description": "Do it later" }
                    ]
                }
            ]
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade passes questions to execute_hitl unchanged
        let InternalHitlParams::Request { questions } = &result;
        assert_eq!(questions.len(), 2);
        assert_eq!(questions[0].id, "approach");
        assert_eq!(questions[0].header, "Approach");
        assert_eq!(questions[0].question, "Which approach do you prefer?");
        assert_eq!(questions[0].options.as_ref().unwrap().len(), 2);
        assert_eq!(questions[1].id, "priority");
    }

    /// Scenario: Facade schema has additionalProperties false
    #[test]
    fn test_codex_request_user_input_schema_has_additional_properties_false() {
        // @step Given a CodexRequestUserInputFacade instance
        let facade = CodexRequestUserInputFacade;

        // @step When the tool definition schema is inspected
        let def = facade.definition();

        // @step Then the schema has additionalProperties set to false
        assert_eq!(def.parameters["additionalProperties"], false);

        // @step And the schema has "questions" in the required array
        assert_eq!(def.parameters["required"], json!(["questions"]));
    }

    /// Scenario: CodexRequestUserInputFacade tool name and provider
    #[test]
    fn test_codex_request_user_input_tool_name_and_provider() {
        // @step Given a CodexRequestUserInputFacade instance
        let facade = CodexRequestUserInputFacade;

        // @step Then the facade tool name is "request_user_input"
        assert_eq!(facade.tool_name(), "request_user_input");

        // @step And the facade provider is "codex"
        assert_eq!(facade.provider(), "codex");
    }

    /// Scenario: Freeform-only question without options maps correctly
    #[test]
    fn test_codex_request_user_input_freeform_only() {
        // @step Given a CodexRequestUserInputFacade instance
        let facade = CodexRequestUserInputFacade;

        // @step When the Codex model calls request_user_input with a question without options
        let input = json!({
            "questions": [
                {
                    "id": "feedback",
                    "header": "Feedback",
                    "question": "Any additional feedback?"
                }
            ]
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalHitlParams::Request with options None
        let InternalHitlParams::Request { questions } = &result;
        assert_eq!(questions.len(), 1);
        assert_eq!(questions[0].id, "feedback");
        assert!(questions[0].options.is_none());
    }

    /// Scenario: Validation rejects missing questions parameter
    #[test]
    fn test_codex_request_user_input_missing_questions() {
        // @step Given a CodexRequestUserInputFacade instance
        let facade = CodexRequestUserInputFacade;

        // @step When the Codex model calls request_user_input without questions
        let input = json!({});
        let result = facade.map_params(input);

        // @step Then the facade returns a validation error about missing questions
        assert!(result.is_err());
        if let Err(ToolError::Validation { tool, message }) = result {
            assert_eq!(tool, "request_user_input");
            assert!(message.contains("questions"));
        } else {
            panic!("Expected ToolError::Validation");
        }
    }

    /// Scenario: Facade definition has correct name and description
    #[test]
    fn test_codex_request_user_input_definition() {
        // @step Given a CodexRequestUserInputFacade instance
        let facade = CodexRequestUserInputFacade;

        // @step When the tool definition is requested
        let def = facade.definition();

        // @step Then the tool name is "request_user_input"
        assert_eq!(def.name, "request_user_input");

        // @step And the description mentions requesting structured input
        assert!(def.description.contains("structured input"));

        // @step And the questions items schema has id, header, question as required
        let items = &def.parameters["properties"]["questions"]["items"];
        let required = items["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v.as_str() == Some("id")));
        assert!(required.iter().any(|v| v.as_str() == Some("header")));
        assert!(required.iter().any(|v| v.as_str() == Some("question")));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod native_tool_tests;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod native_grep_and_schema_tests;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod shell_command_params_tests;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod list_dir_pagination_tests;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod grep_files_include_limit_tests;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod read_file_mode_tests;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod view_image_tests;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod shell_facade_tests;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod exec_command_facade_tests;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod write_stdin_tests;
