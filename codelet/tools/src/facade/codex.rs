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

use super::traits::{
    BashToolFacade, ExecToolFacade, FileToolFacade, HitlToolFacade, InternalBashParams,
    InternalExecParams, InternalFileParams, InternalHitlParams, InternalIndentationParams,
    InternalLsParams, InternalSearchParams, LsToolFacade, SearchToolFacade, ToolDefinition,
};
use super::param_extract::{extract_optional_bool, extract_optional_string, extract_optional_uint, extract_required_string};
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
        let timeout_ms = input.get("timeout_ms").and_then(Value::as_u64);
        Ok(InternalBashParams::Execute { command, cwd, timeout_ms })
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
        Ok(InternalLsParams::List { path: dir_path, offset, limit, depth })
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
            description: "View a local image from the filesystem (only use if given a full filepath \
                by the user, and the image isn't already attached to the thread context \
                within <image ...> tags).".to_string(),
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

        Ok(InternalSearchParams::Grep { pattern, path, include, limit })
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
        let timeout_ms = input.get("timeout_ms").and_then(Value::as_u64);
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
        let yield_time_ms = input.get("yield_time_ms").and_then(Value::as_u64);
        let max_output_tokens = input.get("max_output_tokens").and_then(Value::as_u64);
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
/// - `session_id` (required): Numeric ID of running session from exec_command
/// - `chars` (optional): Characters to write — empty or absent means poll
/// - `yield_time_ms` (optional): Wait time for output
/// - `max_output_tokens` (optional): Max output tokens
///
/// The facade converts session_id from Codex Number to unified exec String.
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
                        "type": "number",
                        "description": "ID of running session from exec_command"
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
        // session_id is required and must be a number → convert to string
        let session_id_val = input.get("session_id").ok_or_else(|| ToolError::Validation {
            tool: "write_stdin",
            message: "Missing required parameter: session_id".to_string(),
        })?;
        let session_id = match session_id_val {
            Value::Number(n) => n.to_string(),
            Value::Null => {
                return Err(ToolError::Validation {
                    tool: "write_stdin",
                    message: "Missing required parameter: session_id".to_string(),
                });
            }
            _ => {
                return Err(ToolError::Validation {
                    tool: "write_stdin",
                    message: "session_id must be a number".to_string(),
                });
            }
        };

        let chars = extract_optional_string(&input, "chars").unwrap_or_default();
        let yield_time_ms = input.get("yield_time_ms").and_then(Value::as_u64);
        let max_output_tokens = input.get("max_output_tokens").and_then(Value::as_u64);

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

        let questions_val = input.get("questions").ok_or_else(|| ToolError::Validation {
            tool: "request_user_input",
            message: "Missing required parameter: questions".to_string(),
        })?;

        let questions_arr = questions_val.as_array().ok_or_else(|| ToolError::Validation {
            tool: "request_user_input",
            message: "questions must be an array".to_string(),
        })?;

        let mut questions = Vec::new();
        for q in questions_arr {
            let id = q.get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let header = q.get("header")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let question = q.get("question")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();

            let options = q.get("options").and_then(|opts_val| {
                opts_val.as_array().map(|opts_arr| {
                    opts_arr.iter().map(|o| {
                        HitlOption {
                            label: o.get("label")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string(),
                            description: o.get("description")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string(),
                        }
                    }).collect::<Vec<_>>()
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
    use crate::facade::bash::GeminiRunShellCommandFacade;

    // =========================================================================
    // shell_command tests
    // =========================================================================

    /// Feature: spec/features/codex-native-tool-facades.feature
    ///
    /// Scenario: CodexShellCommandFacade maps shell_command to InternalBashParams
    #[test]
    fn test_codex_shell_command_facade() {
        // @step Given a CodexShellCommandFacade instance
        let facade = CodexShellCommandFacade;

        // @step When the Codex model calls shell_command with command "ls -la" and workdir "/tmp"
        let input = json!({
            "command": "ls -la",
            "workdir": "/tmp"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalBashParams::Execute with command "ls -la"
        assert_eq!(
            result,
            InternalBashParams::Execute {
                command: "ls -la".to_string(),
                cwd: Some("/tmp".to_string()),
                timeout_ms: None,
            }
        );

        // @step And the facade tool name is "shell_command"
        assert_eq!(facade.tool_name(), "shell_command");

        // @step And the facade provider is "codex"
        assert_eq!(facade.provider(), "codex");
    }

    /// Scenario: Codex facades validate required parameters (shell_command variant)
    #[test]
    fn test_codex_shell_command_missing_command() {
        // @step Given a CodexShellCommandFacade instance
        let facade = CodexShellCommandFacade;

        // @step When the Codex model calls shell_command with missing command field
        let input = json!({});
        let result = facade.map_params(input);

        // @step Then the facade returns a validation error
        assert!(result.is_err());

        // @step And the error identifies "shell_command" as the tool
        // @step And the error mentions "command" as the missing field
        if let Err(ToolError::Validation { tool, message }) = result {
            assert_eq!(tool, "shell_command");
            assert!(message.contains("command"));
        } else {
            panic!("Expected ToolError::Validation");
        }
    }

    #[test]
    fn test_codex_shell_command_empty_command() {
        let facade = CodexShellCommandFacade;
        let input = json!({
            "command": ""
        });

        let result = facade.map_params(input);
        assert!(result.is_err());
    }

    // =========================================================================
    // BUG-108: shell_command workdir and timeout_ms mapping tests
    // Feature: spec/features/codex-shell-command-params.feature
    // =========================================================================

    /// Scenario: CodexShellCommandFacade maps workdir to InternalBashParams cwd
    #[test]
    fn test_codex_shell_command_maps_workdir_to_cwd() {
        // @step Given a CodexShellCommandFacade instance
        let facade = CodexShellCommandFacade;

        // @step When the Codex model calls shell_command with command "make test" and workdir "/project"
        let input = json!({
            "command": "make test",
            "workdir": "/project"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalBashParams::Execute with command "make test" and cwd "/project"
        assert_eq!(
            result,
            InternalBashParams::Execute {
                command: "make test".to_string(),
                cwd: Some("/project".to_string()),
                timeout_ms: None,
            }
        );

        // @step And timeout_ms is None
        let InternalBashParams::Execute { timeout_ms, .. } = &result;
        assert_eq!(*timeout_ms, None);
    }

    /// Scenario: CodexShellCommandFacade maps timeout_ms to InternalBashParams
    #[test]
    fn test_codex_shell_command_maps_timeout_ms() {
        // @step Given a CodexShellCommandFacade instance
        let facade = CodexShellCommandFacade;

        // @step When the Codex model calls shell_command with command "sleep 100" and timeout_ms 5000
        let input = json!({
            "command": "sleep 100",
            "timeout_ms": 5000
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalBashParams::Execute with command "sleep 100" and timeout_ms 5000
        assert_eq!(
            result,
            InternalBashParams::Execute {
                command: "sleep 100".to_string(),
                cwd: None,
                timeout_ms: Some(5000),
            }
        );

        // @step And cwd is None
        let InternalBashParams::Execute { cwd, .. } = &result;
        assert_eq!(*cwd, None);
    }

    /// Scenario: CodexShellCommandFacade maps both workdir and timeout_ms
    #[test]
    fn test_codex_shell_command_maps_both_workdir_and_timeout() {
        // @step Given a CodexShellCommandFacade instance
        let facade = CodexShellCommandFacade;

        // @step When the Codex model calls shell_command with command "npm test" workdir "/app" and timeout_ms 30000
        let input = json!({
            "command": "npm test",
            "workdir": "/app",
            "timeout_ms": 30000
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalBashParams::Execute with command "npm test" cwd "/app" and timeout_ms 30000
        assert_eq!(
            result,
            InternalBashParams::Execute {
                command: "npm test".to_string(),
                cwd: Some("/app".to_string()),
                timeout_ms: Some(30000),
            }
        );
    }

    /// Scenario: CodexShellCommandFacade without optional params defaults to None
    #[test]
    fn test_codex_shell_command_without_optional_params() {
        // @step Given a CodexShellCommandFacade instance
        let facade = CodexShellCommandFacade;

        // @step When the Codex model calls shell_command with only command "echo hello"
        let input = json!({
            "command": "echo hello"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalBashParams::Execute with command "echo hello"
        assert_eq!(
            result,
            InternalBashParams::Execute {
                command: "echo hello".to_string(),
                cwd: None,
                timeout_ms: None,
            }
        );

        // @step And cwd is None
        // @step And timeout_ms is None
        let InternalBashParams::Execute { cwd, timeout_ms, .. } = &result;
        assert_eq!(*cwd, None);
        assert_eq!(*timeout_ms, None);
    }

    /// Scenario: Codex shell_command schema includes Codex-native approval params
    #[test]
    fn test_codex_shell_command_schema_has_approval_params() {
        // @step Given a CodexShellCommandFacade instance
        let facade = CodexShellCommandFacade;

        // @step When the tool definition schema is inspected
        let def = facade.definition();

        // @step Then the schema has a "login" property of type "boolean"
        assert_eq!(def.parameters["properties"]["login"]["type"], "boolean");

        // @step And the schema has a "sandbox_permissions" property of type "string"
        assert_eq!(
            def.parameters["properties"]["sandbox_permissions"]["type"],
            "string"
        );

        // @step And the schema has a "justification" property of type "string"
        assert_eq!(
            def.parameters["properties"]["justification"]["type"],
            "string"
        );

        // @step And the schema has a "prefix_rule" property of type "array"
        assert_eq!(
            def.parameters["properties"]["prefix_rule"]["type"],
            "array"
        );
    }

    /// Scenario: Codex-native approval params are silently ignored in map_params
    #[test]
    fn test_codex_shell_command_approval_params_ignored() {
        // @step Given a CodexShellCommandFacade instance
        let facade = CodexShellCommandFacade;

        // @step When the Codex model calls shell_command with command "ls" and login true and sandbox_permissions "use_default"
        let input = json!({
            "command": "ls",
            "login": true,
            "sandbox_permissions": "use_default",
            "justification": "testing",
            "prefix_rule": ["ls"]
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalBashParams::Execute with command "ls"
        // @step And cwd is None
        // @step And timeout_ms is None
        assert_eq!(
            result,
            InternalBashParams::Execute {
                command: "ls".to_string(),
                cwd: None,
                timeout_ms: None,
            }
        );
    }

    /// Scenario: Existing facades remain backward compatible with new InternalBashParams fields
    #[test]
    fn test_gemini_facade_backward_compatible_with_new_fields() {
        // @step Given a GeminiRunShellCommandFacade instance
        let facade = GeminiRunShellCommandFacade;

        // @step When the Gemini model calls run_shell_command with command "ls"
        let input = json!({
            "command": "ls"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalBashParams::Execute with command "ls"
        // @step And cwd is None
        // @step And timeout_ms is None
        assert_eq!(
            result,
            InternalBashParams::Execute {
                command: "ls".to_string(),
                cwd: None,
                timeout_ms: None,
            }
        );
    }

    // =========================================================================
    // read_file tests
    // =========================================================================

    /// Scenario: CodexReadFileFacade maps read_file to InternalFileParams::Read
    #[test]
    fn test_codex_read_file_facade_with_offset_limit() {
        // @step Given a CodexReadFileFacade instance
        let facade = CodexReadFileFacade;

        // @step When the Codex model calls read_file with file_path "/src/main.rs" offset 10 and limit 50
        let input = json!({
            "file_path": "/src/main.rs",
            "offset": 10,
            "limit": 50
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalFileParams::Read with file_path "/src/main.rs" offset 10 and limit 50
        assert_eq!(
            result,
            InternalFileParams::Read {
                file_path: "/src/main.rs".to_string(),
                offset: Some(10),
                limit: Some(50),
                mode: None,
                indentation: None,
            }
        );

        // @step And the facade tool name is "read_file"
        assert_eq!(facade.tool_name(), "read_file");

        // @step And the facade provider is "codex"
        assert_eq!(facade.provider(), "codex");
    }

    /// Scenario: Codex facades handle optional parameters gracefully
    #[test]
    fn test_codex_read_file_facade_without_offset_limit() {
        // @step Given a CodexReadFileFacade instance
        let facade = CodexReadFileFacade;

        // @step When the Codex model calls read_file with only file_path "/src/main.rs"
        let input = json!({
            "file_path": "/src/main.rs"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalFileParams::Read with offset None and limit None
        assert_eq!(
            result,
            InternalFileParams::Read {
                file_path: "/src/main.rs".to_string(),
                offset: None,
                limit: None,
                mode: None,
                indentation: None,
            }
        );
    }

    #[test]
    fn test_codex_read_file_missing_path() {
        let facade = CodexReadFileFacade;
        let input = json!({});

        let result = facade.map_params(input);
        assert!(result.is_err());
        if let Err(ToolError::Validation { tool, message }) = result {
            assert_eq!(tool, "read_file");
            assert!(message.contains("file_path"));
        }
    }

    #[test]
    fn test_codex_read_file_empty_path() {
        let facade = CodexReadFileFacade;
        let input = json!({
            "file_path": ""
        });

        let result = facade.map_params(input);
        assert!(result.is_err());
    }

    // =========================================================================
    // list_dir tests
    // =========================================================================

    /// Scenario: CodexListDirFacade maps list_dir to InternalLsParams::List
    #[test]
    fn test_codex_list_dir_facade_with_dir_path() {
        // @step Given a CodexListDirFacade instance
        let facade = CodexListDirFacade;

        // @step When the Codex model calls list_dir with dir_path "/src"
        let input = json!({
            "dir_path": "/src"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalLsParams::List with path "/src"
        assert_eq!(
            result,
            InternalLsParams::List {
                path: Some("/src".to_string()),
                offset: None,
                limit: None,
                depth: None,
            }
        );

        // @step And the facade tool name is "list_dir"
        assert_eq!(facade.tool_name(), "list_dir");

        // @step And the facade provider is "codex"
        assert_eq!(facade.provider(), "codex");
    }

    #[test]
    fn test_codex_list_dir_facade_with_empty_dir_path() {
        let facade = CodexListDirFacade;
        let input = json!({
            "dir_path": ""
        });

        let result = facade.map_params(input).unwrap();
        // Empty dir_path is treated as None (use default)
        assert_eq!(result, InternalLsParams::List { path: None, offset: None, limit: None, depth: None });
    }

    #[test]
    fn test_codex_list_dir_facade_with_null_dir_path() {
        let facade = CodexListDirFacade;
        let input = json!({
            "dir_path": null
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(result, InternalLsParams::List { path: None, offset: None, limit: None, depth: None });
    }

    // =========================================================================
    // BUG-110: list_dir offset, limit, and depth mapping tests
    // Feature: spec/features/codex-list-dir-pagination.feature
    // =========================================================================

    /// Scenario: CodexListDirFacade maps offset and limit to InternalLsParams
    #[test]
    fn test_codex_list_dir_maps_offset_and_limit() {
        // @step Given a CodexListDirFacade instance
        let facade = CodexListDirFacade;

        // @step When the Codex model calls list_dir with dir_path "/src" offset 5 and limit 10
        let input = json!({
            "dir_path": "/src",
            "offset": 5,
            "limit": 10
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalLsParams::List with path "/src" offset 5 and limit 10
        assert_eq!(
            result,
            InternalLsParams::List {
                path: Some("/src".to_string()),
                offset: Some(5),
                limit: Some(10),
                depth: None,
            }
        );

        // @step Then depth is None
        let InternalLsParams::List { depth, .. } = &result;
        assert_eq!(*depth, None);
    }

    /// Scenario: CodexListDirFacade backward compatible without optional params
    #[test]
    fn test_codex_list_dir_backward_compatible_no_pagination() {
        // @step Given a CodexListDirFacade instance
        let facade = CodexListDirFacade;

        // @step When the Codex model calls list_dir with only dir_path "/src"
        let input = json!({
            "dir_path": "/src"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalLsParams::List with offset None limit None and depth None
        assert_eq!(
            result,
            InternalLsParams::List {
                path: Some("/src".to_string()),
                offset: None,
                limit: None,
                depth: None,
            }
        );
    }

    /// Scenario: CodexListDirFacade schema includes offset limit and depth properties
    #[test]
    fn test_codex_list_dir_schema_has_pagination_params() {
        // @step Given a CodexListDirFacade instance
        let facade = CodexListDirFacade;

        // @step When the tool definition schema is inspected
        let def = facade.definition();

        // @step Then the schema has an "offset" property of type "integer"
        assert_eq!(def.parameters["properties"]["offset"]["type"], "integer");

        // @step Then the schema has a "limit" property of type "integer"
        assert_eq!(def.parameters["properties"]["limit"]["type"], "integer");

        // @step Then the schema has a "depth" property of type "integer"
        assert_eq!(def.parameters["properties"]["depth"]["type"], "integer");

        // @step Then only "dir_path" is in the required array
        assert_eq!(def.parameters["required"], json!(["dir_path"]));
    }

    /// Scenario: CodexListDirFacade maps depth to InternalLsParams
    #[test]
    fn test_codex_list_dir_maps_depth() {
        // @step Given a CodexListDirFacade instance
        let facade = CodexListDirFacade;

        // @step When the Codex model calls list_dir with dir_path "/src" and depth 3
        let input = json!({
            "dir_path": "/src",
            "depth": 3
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalLsParams::List with path "/src" and depth 3
        assert_eq!(
            result,
            InternalLsParams::List {
                path: Some("/src".to_string()),
                offset: None,
                limit: None,
                depth: Some(3),
            }
        );

        // @step Then offset is None and limit is None
        let InternalLsParams::List { offset, limit, .. } = &result;
        assert_eq!(*offset, None);
        assert_eq!(*limit, None);
    }

    /// Scenario: Other facades provide None for offset limit and depth
    #[test]
    fn test_zai_facade_provides_none_for_pagination_params() {
        use crate::facade::zai::ZAIListDirFacade;

        // @step Given a ZAIListDirFacade instance
        let facade = ZAIListDirFacade;

        // @step When the ZAI model calls list_dir with path "/src"
        let input = json!({
            "path": "/src"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalLsParams::List with offset None limit None and depth None
        assert_eq!(
            result,
            InternalLsParams::List {
                path: Some("/src".to_string()),
                offset: None,
                limit: None,
                depth: None,
            }
        );
    }

    #[test]
    fn test_codex_list_dir_uses_dir_path_not_path() {
        // Verify that `dir_path` is used, not `path`
        let facade = CodexListDirFacade;

        // If model sends `path` (wrong field name), it should NOT be picked up
        let input = json!({
            "path": "/src"
        });
        let result = facade.map_params(input).unwrap();
        assert_eq!(result, InternalLsParams::List { path: None, offset: None, limit: None, depth: None });

        // But `dir_path` should work
        let input = json!({
            "dir_path": "/src"
        });
        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalLsParams::List {
                path: Some("/src".to_string()),
                offset: None,
                limit: None,
                depth: None,
            }
        );
    }

    // =========================================================================
    // grep_files tests
    // =========================================================================

    /// Scenario: CodexGrepFilesFacade maps grep_files to InternalSearchParams::Grep
    #[test]
    fn test_codex_grep_files_facade() {
        // @step Given a CodexGrepFilesFacade instance
        let facade = CodexGrepFilesFacade;

        // @step When the Codex model calls grep_files with pattern "TODO" and path "/src"
        let input = json!({
            "pattern": "TODO",
            "path": "/src"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalSearchParams::Grep with pattern "TODO" and path "/src"
        assert_eq!(
            result,
            InternalSearchParams::Grep {
                pattern: "TODO".to_string(),
                path: Some("/src".to_string()),
                include: None,
                limit: None,
            }
        );

        // @step And the facade tool name is "grep_files"
        assert_eq!(facade.tool_name(), "grep_files");

        // @step And the facade provider is "codex"
        assert_eq!(facade.provider(), "codex");
    }

    #[test]
    fn test_codex_grep_files_facade_no_path() {
        let facade = CodexGrepFilesFacade;
        let input = json!({
            "pattern": "TODO"
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalSearchParams::Grep {
                pattern: "TODO".to_string(),
                path: None,
                include: None,
                limit: None,
            }
        );
    }

    /// Feature: spec/features/codex-grep-files-include-limit.feature
    ///
    /// Scenario: CodexGrepFilesFacade maps include param to InternalSearchParams::Grep
    #[test]
    fn test_codex_grep_files_facade_maps_include_param() {
        // @step Given a CodexGrepFilesFacade instance
        let facade = CodexGrepFilesFacade;

        // @step When the Codex model calls grep_files with pattern "TODO", include "*.rs", and path "/src"
        let input = json!({
            "pattern": "TODO",
            "include": "*.rs",
            "path": "/src"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalSearchParams::Grep with pattern "TODO", path "/src", and include "*.rs"
        assert_eq!(
            result,
            InternalSearchParams::Grep {
                pattern: "TODO".to_string(),
                path: Some("/src".to_string()),
                include: Some("*.rs".to_string()),
                limit: None,
            }
        );
    }

    /// Scenario: CodexGrepFilesFacade maps limit param to InternalSearchParams::Grep
    #[test]
    fn test_codex_grep_files_facade_maps_limit_param() {
        // @step Given a CodexGrepFilesFacade instance
        let facade = CodexGrepFilesFacade;

        // @step When the Codex model calls grep_files with pattern "." and limit 10
        let input = json!({
            "pattern": ".",
            "limit": 10
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalSearchParams::Grep with pattern "." and limit 10
        assert_eq!(
            result,
            InternalSearchParams::Grep {
                pattern: ".".to_string(),
                path: None,
                include: None,
                limit: Some(10),
            }
        );
    }

    /// Scenario: CodexGrepFilesFacade maps both include and limit params
    #[test]
    fn test_codex_grep_files_facade_maps_include_and_limit() {
        // @step Given a CodexGrepFilesFacade instance
        let facade = CodexGrepFilesFacade;

        // @step When the Codex model calls grep_files with pattern ".", include "*.rs", and limit 10
        let input = json!({
            "pattern": ".",
            "include": "*.rs",
            "limit": 10
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalSearchParams::Grep with include "*.rs" and limit 10
        assert_eq!(
            result,
            InternalSearchParams::Grep {
                pattern: ".".to_string(),
                path: None,
                include: Some("*.rs".to_string()),
                limit: Some(10),
            }
        );
    }

    /// Scenario: CodexGrepFilesFacade remains backward compatible without include or limit
    #[test]
    fn test_codex_grep_files_facade_backward_compatible() {
        // @step Given a CodexGrepFilesFacade instance
        let facade = CodexGrepFilesFacade;

        // @step When the Codex model calls grep_files with only pattern "TODO"
        let input = json!({
            "pattern": "TODO"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalSearchParams::Grep with include None and limit None
        assert_eq!(
            result,
            InternalSearchParams::Grep {
                pattern: "TODO".to_string(),
                path: None,
                include: None,
                limit: None,
            }
        );
    }

    /// Feature: spec/features/codex-grep-files-include-limit.feature
    ///
    /// Scenario: InternalSearchParams::Grep includes optional include and limit fields
    #[test]
    fn test_internal_search_params_grep_has_include_and_limit_fields() {
        // @step Given the InternalSearchParams::Grep enum variant
        // Construct it with all fields to prove they exist

        // @step Then it has an optional "include" field of type Option<String>
        let with_include = InternalSearchParams::Grep {
            pattern: "test".to_string(),
            path: None,
            include: Some("*.rs".to_string()),
            limit: None,
        };
        if let InternalSearchParams::Grep { include, .. } = &with_include {
            assert_eq!(include, &Some("*.rs".to_string()));
        }

        // @step And it has an optional "limit" field of type Option<usize>
        let with_limit = InternalSearchParams::Grep {
            pattern: "test".to_string(),
            path: None,
            include: None,
            limit: Some(100),
        };
        if let InternalSearchParams::Grep { limit, .. } = &with_limit {
            assert_eq!(limit, &Some(100));
        }
    }

    #[test]
    fn test_codex_grep_files_facade_missing_pattern() {
        let facade = CodexGrepFilesFacade;
        let input = json!({
            "path": "/src"
        });

        let result = facade.map_params(input);
        assert!(result.is_err());
        if let Err(ToolError::Validation { tool, message }) = result {
            assert_eq!(tool, "grep_files");
            assert!(message.contains("pattern"));
        }
    }

    // =========================================================================
    // Tool naming tests
    // =========================================================================

    /// BUG-107: Verify only native Codex CLI tools are exposed (no glob)
    ///
    /// Feature: spec/features/codex-native-tool-facades.feature
    ///
    /// Scenario: Codex agent does not expose non-native glob tool
    #[test]
    fn test_codex_does_not_expose_non_native_glob() {
        // @step Given a Codex agent built with create_rig_agent
        // Collect all facade tool names that would be registered
        let codex_facade_names = vec![
            CodexShellCommandFacade.tool_name(),
            CodexReadFileFacade.tool_name(),
            CodexListDirFacade.tool_name(),
            CodexGrepFilesFacade.tool_name(),
            CodexViewImageFacade.tool_name(),
        ];

        // @step When the agent tool definitions are inspected
        // @step Then the tool list does not contain "glob"
        assert!(
            !codex_facade_names.contains(&"glob"),
            "Codex facades should NOT include 'glob' - it is not a native Codex CLI tool"
        );

        // @step And the tool list contains "shell_command"
        assert!(codex_facade_names.contains(&"shell_command"));
        // @step And the tool list contains "read_file"
        assert!(codex_facade_names.contains(&"read_file"));
        // @step And the tool list contains "list_dir"
        assert!(codex_facade_names.contains(&"list_dir"));
        // @step And the tool list contains "grep_files"
        assert!(codex_facade_names.contains(&"grep_files"));
        // @step And the tool list contains "apply_patch"
        // Note: apply_patch is a standalone tool (not a facade), tested in codex provider tests
    }

    #[test]
    fn test_codex_tools_use_correct_names() {
        assert_eq!(CodexShellCommandFacade.tool_name(), "shell_command");
        assert_eq!(CodexReadFileFacade.tool_name(), "read_file");
        assert_eq!(CodexListDirFacade.tool_name(), "list_dir");
        assert_eq!(CodexGrepFilesFacade.tool_name(), "grep_files");
        assert_eq!(CodexViewImageFacade.tool_name(), "view_image");
    }

    #[test]
    fn test_codex_tools_provider_name() {
        assert_eq!(CodexShellCommandFacade.provider(), "codex");
        assert_eq!(CodexReadFileFacade.provider(), "codex");
        assert_eq!(CodexListDirFacade.provider(), "codex");
        assert_eq!(CodexGrepFilesFacade.provider(), "codex");
        assert_eq!(CodexViewImageFacade.provider(), "codex");
    }

    // =========================================================================
    // Schema validation tests
    // =========================================================================

    /// Scenario: Codex tool schemas use additionalProperties false
    #[test]
    fn test_all_codex_schemas_have_additional_properties_false() {
        // @step Given all Codex facade instances
        let facades: Vec<(&str, serde_json::Value)> = vec![
            ("shell_command", CodexShellCommandFacade.definition().parameters),
            ("read_file", CodexReadFileFacade.definition().parameters),
            ("list_dir", CodexListDirFacade.definition().parameters),
            ("grep_files", CodexGrepFilesFacade.definition().parameters),
            ("view_image", CodexViewImageFacade.definition().parameters),
        ];

        // @step When their tool definitions are inspected
        // @step Then each schema has additionalProperties set to false
        for (name, params) in &facades {
            assert_eq!(
                params["additionalProperties"], false,
                "{name} should have additionalProperties: false"
            );
        }

        // @step And each schema has the correct required fields
        let shell_params = &facades[0].1;
        assert_eq!(shell_params["required"], json!(["command"]));

        let read_params = &facades[1].1;
        assert_eq!(read_params["required"], json!(["file_path"]));

        let list_params = &facades[2].1;
        assert_eq!(list_params["required"], json!(["dir_path"]));

        let grep_params = &facades[3].1;
        assert_eq!(grep_params["required"], json!(["pattern"]));
    }

    #[test]
    fn test_codex_list_dir_schema_uses_dir_path_not_path() {
        let facade = CodexListDirFacade;
        let def = facade.definition();

        // Verify schema uses dir_path, not path
        assert!(def.parameters["properties"]["dir_path"].is_object());
        assert!(def.parameters["properties"].get("path").is_none());
    }

    #[test]
    fn test_codex_grep_files_schema_has_include_param() {
        let facade = CodexGrepFilesFacade;
        let def = facade.definition();

        // Verify schema has `include` parameter (Codex-specific)
        assert!(def.parameters["properties"]["include"].is_object());
    }

    #[test]
    fn test_codex_shell_command_schema_has_workdir_and_timeout() {
        let facade = CodexShellCommandFacade;
        let def = facade.definition();

        // Verify schema has Codex-specific optional params
        assert!(def.parameters["properties"]["workdir"].is_object());
        assert!(def.parameters["properties"]["timeout_ms"].is_object());
    }

    // =========================================================================
    // BUG-111: GrepArgs glob field tests
    // Feature: spec/features/codex-grep-files-include-limit.feature
    // =========================================================================

    /// Scenario: GrepArgs struct supports optional glob field
    #[test]
    fn test_grep_args_supports_optional_glob_field() {
        use crate::grep::GrepArgs;

        // @step Given a GrepArgs with pattern "TODO" and glob "*.rs"
        let args = GrepArgs {
            pattern: "TODO".to_string(),
            path: None,
            output_mode: None,
            glob: Some("*.rs".to_string()),
            limit: None,
        };

        // @step When the GrepTool call() method executes
        // Verify the struct has the glob field and it can be set
        assert_eq!(args.glob.as_deref(), Some("*.rs"));

        // @step Then the glob filter is passed through to the execute() method
        // The call() method serializes args to Value and passes glob through
        let value = serde_json::to_value(&args).unwrap();
        assert_eq!(value["glob"], "*.rs");
    }

    /// Scenario: SearchToolFacadeWrapper passes include as glob to GrepTool
    #[test]
    fn test_wrapper_grep_args_construction_with_include() {
        use crate::grep::GrepArgs;

        // @step Given a SearchToolFacadeWrapper with a CodexGrepFilesFacade
        let facade = CodexGrepFilesFacade;

        // @step When the wrapper receives InternalSearchParams::Grep with include "*.rs"
        let input = json!({
            "pattern": "TODO",
            "include": "*.rs",
            "path": "/src"
        });
        let params = facade.map_params(input).unwrap();

        // @step Then the wrapper passes "glob" = "*.rs" in the GrepTool execute args
        // Verify the internal params carry include, and show how wrapper would construct GrepArgs
        if let InternalSearchParams::Grep { pattern, path, include, limit } = params {
            let grep_args = GrepArgs {
                pattern,
                path,
                output_mode: None,
                glob: include,
                limit,
            };
            assert_eq!(grep_args.glob.as_deref(), Some("*.rs"));
        } else {
            panic!("Expected InternalSearchParams::Grep");
        }
    }

    /// Scenario: SearchToolFacadeWrapper applies limit to cap grep results
    #[test]
    fn test_wrapper_grep_args_construction_with_limit() {
        use crate::grep::GrepArgs;

        // @step Given a SearchToolFacadeWrapper with a CodexGrepFilesFacade
        let facade = CodexGrepFilesFacade;

        // @step When the wrapper receives InternalSearchParams::Grep with limit 10
        let input = json!({
            "pattern": "TODO",
            "limit": 10
        });
        let params = facade.map_params(input).unwrap();

        // @step Then the wrapper caps the grep output to at most 10 result lines
        // Verify the internal params carry limit, and show how wrapper would construct GrepArgs
        if let InternalSearchParams::Grep { pattern, path, include, limit } = params {
            let grep_args = GrepArgs {
                pattern,
                path,
                output_mode: None,
                glob: include,
                limit,
            };
            assert_eq!(grep_args.limit, Some(10));
        } else {
            panic!("Expected InternalSearchParams::Grep");
        }
    }

    /// Feature: spec/features/codex-grep-files-include-limit.feature
    ///
    /// Scenario: ZAI grep facade constructs InternalSearchParams::Grep with None defaults
    #[test]
    fn test_zai_grep_facade_uses_none_defaults_for_include_and_limit() {
        use crate::facade::zai::ZAIGrepFilesFacade;

        // @step Given a ZAIGrepFilesFacade instance
        let facade = ZAIGrepFilesFacade;

        // @step When the ZAI model calls grep_files with pattern "TODO" and path "src"
        let input = json!({
            "pattern": "TODO",
            "path": "src"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalSearchParams::Grep with include None and limit None
        assert_eq!(
            result,
            InternalSearchParams::Grep {
                pattern: "TODO".to_string(),
                path: Some("src".to_string()),
                include: None,
                limit: None,
            }
        );
    }

    /// Scenario: Gemini grep facade constructs InternalSearchParams::Grep with None defaults
    #[test]
    fn test_gemini_grep_facade_uses_none_defaults_for_include_and_limit() {
        use crate::facade::search::GeminiSearchFileContentFacade;

        // @step Given a GeminiSearchFileContentFacade instance
        let facade = GeminiSearchFileContentFacade;

        // @step When Gemini sends parameters {pattern: 'TODO', dir_path: 'src'} to tool 'search_file_content'
        let input = json!({
            "pattern": "TODO",
            "dir_path": "src"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalSearchParams::Grep with include None and limit None
        assert_eq!(
            result,
            InternalSearchParams::Grep {
                pattern: "TODO".to_string(),
                path: Some("src".to_string()),
                include: None,
                limit: None,
            }
        );
    }

    // =========================================================================
    // BUG-109: read_file mode and indentation params tests
    // Feature: spec/features/codex-read-file-mode-indentation.feature
    // =========================================================================

    /// Scenario: CodexReadFileFacade maps mode indentation to InternalFileParams::Read
    #[test]
    fn test_codex_read_file_maps_mode_indentation() {
        use crate::facade::traits::InternalIndentationParams;

        // @step Given a CodexReadFileFacade instance
        let facade = CodexReadFileFacade;

        // @step When the Codex model calls read_file with file_path "/src/main.rs" mode "indentation" and indentation {anchor_line: 50, max_levels: 2}
        let input = json!({
            "file_path": "/src/main.rs",
            "mode": "indentation",
            "indentation": {
                "anchor_line": 50,
                "max_levels": 2
            }
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalFileParams::Read with file_path "/src/main.rs"
        // @step And mode is Some("indentation")
        // @step And indentation anchor_line is Some(50)
        // @step And indentation max_levels is Some(2)
        // @step And indentation include_siblings is None
        // @step And indentation include_header is None
        // @step And indentation max_lines is None
        assert_eq!(
            result,
            InternalFileParams::Read {
                file_path: "/src/main.rs".to_string(),
                offset: None,
                limit: None,
                mode: Some("indentation".to_string()),
                indentation: Some(InternalIndentationParams {
                    anchor_line: Some(50),
                    max_levels: Some(2),
                    include_siblings: None,
                    include_header: None,
                    max_lines: None,
                }),
            }
        );
    }

    /// Scenario: CodexReadFileFacade maps mode slice without indentation
    #[test]
    fn test_codex_read_file_maps_mode_slice() {
        // @step Given a CodexReadFileFacade instance
        let facade = CodexReadFileFacade;

        // @step When the Codex model calls read_file with file_path "/src/main.rs" and mode "slice"
        let input = json!({
            "file_path": "/src/main.rs",
            "mode": "slice"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalFileParams::Read with file_path "/src/main.rs"
        // @step And mode is Some("slice")
        // @step And indentation is None
        assert_eq!(
            result,
            InternalFileParams::Read {
                file_path: "/src/main.rs".to_string(),
                offset: None,
                limit: None,
                mode: Some("slice".to_string()),
                indentation: None,
            }
        );
    }

    /// Scenario: CodexReadFileFacade backward compatible without mode or indentation
    #[test]
    fn test_codex_read_file_backward_compatible_no_mode_no_indentation() {
        // @step Given a CodexReadFileFacade instance
        let facade = CodexReadFileFacade;

        // @step When the Codex model calls read_file with only file_path "/src/main.rs"
        let input = json!({
            "file_path": "/src/main.rs"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalFileParams::Read with file_path "/src/main.rs"
        // @step And mode is None
        // @step And indentation is None
        assert_eq!(
            result,
            InternalFileParams::Read {
                file_path: "/src/main.rs".to_string(),
                offset: None,
                limit: None,
                mode: None,
                indentation: None,
            }
        );
    }

    /// Scenario: Codex read_file schema includes mode and indentation properties
    #[test]
    fn test_codex_read_file_schema_has_mode_and_indentation() {
        // @step Given a CodexReadFileFacade instance
        let facade = CodexReadFileFacade;

        // @step When the tool definition schema is inspected
        let def = facade.definition();

        // @step Then the schema has a "mode" property of type "string"
        assert_eq!(def.parameters["properties"]["mode"]["type"], "string");

        // @step And the schema has an "indentation" property of type "object"
        assert_eq!(
            def.parameters["properties"]["indentation"]["type"],
            "object"
        );

        // @step And the indentation object has an "anchor_line" property of type "integer"
        assert_eq!(
            def.parameters["properties"]["indentation"]["properties"]["anchor_line"]["type"],
            "integer"
        );

        // @step And the indentation object has a "max_levels" property of type "integer"
        assert_eq!(
            def.parameters["properties"]["indentation"]["properties"]["max_levels"]["type"],
            "integer"
        );

        // @step And the indentation object has an "include_siblings" property of type "boolean"
        assert_eq!(
            def.parameters["properties"]["indentation"]["properties"]["include_siblings"]["type"],
            "boolean"
        );

        // @step And the indentation object has an "include_header" property of type "boolean"
        assert_eq!(
            def.parameters["properties"]["indentation"]["properties"]["include_header"]["type"],
            "boolean"
        );

        // @step And the indentation object has a "max_lines" property of type "integer"
        assert_eq!(
            def.parameters["properties"]["indentation"]["properties"]["max_lines"]["type"],
            "integer"
        );

        // @step And the indentation object has additionalProperties false
        assert_eq!(
            def.parameters["properties"]["indentation"]["additionalProperties"],
            false
        );
    }

    /// Scenario: Other facades provide None for mode and indentation fields
    #[test]
    fn test_zai_facade_provides_none_mode_and_indentation() {
        use crate::facade::zai::ZAIReadFileFacade;

        // @step Given a ZAIReadFileFacade instance
        let facade = ZAIReadFileFacade;

        // @step When the ZAI model calls read_file with file_path "/src/main.rs"
        let input = json!({
            "file_path": "/src/main.rs"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalFileParams::Read with mode None and indentation None
        assert_eq!(
            result,
            InternalFileParams::Read {
                file_path: "/src/main.rs".to_string(),
                offset: None,
                limit: None,
                mode: None,
                indentation: None,
            }
        );
    }

    /// Scenario: CodexReadFileFacade extracts all indentation boolean and integer fields
    #[test]
    fn test_codex_read_file_extracts_all_indentation_fields() {
        use crate::facade::traits::InternalIndentationParams;

        // @step Given a CodexReadFileFacade instance
        let facade = CodexReadFileFacade;

        // @step When the Codex model calls read_file with file_path "/src/main.rs" mode "indentation" and indentation {include_siblings: true, include_header: true, max_lines: 100}
        let input = json!({
            "file_path": "/src/main.rs",
            "mode": "indentation",
            "indentation": {
                "include_siblings": true,
                "include_header": true,
                "max_lines": 100
            }
        });
        let result = facade.map_params(input).unwrap();

        // @step Then indentation include_siblings is Some(true)
        // @step And indentation include_header is Some(true)
        // @step And indentation max_lines is Some(100)
        // @step And indentation anchor_line is None
        // @step And indentation max_levels is None
        assert_eq!(
            result,
            InternalFileParams::Read {
                file_path: "/src/main.rs".to_string(),
                offset: None,
                limit: None,
                mode: Some("indentation".to_string()),
                indentation: Some(InternalIndentationParams {
                    anchor_line: None,
                    max_levels: None,
                    include_siblings: Some(true),
                    include_header: Some(true),
                    max_lines: Some(100),
                }),
            }
        );
    }

    // =========================================================================
    // view_image facade tests
    // Feature: spec/features/codex-view-image.feature
    // =========================================================================

    /// Scenario: CodexViewImageFacade maps view_image path to InternalFileParams::Read
    #[test]
    fn test_codex_view_image_facade_maps_path_to_read() {
        // @step Given a CodexViewImageFacade instance
        let facade = CodexViewImageFacade;

        // @step When the Codex model calls view_image with path "/tmp/screenshot.png"
        let input = json!({
            "path": "/tmp/screenshot.png"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalFileParams::Read with file_path "/tmp/screenshot.png"
        assert_eq!(
            result,
            InternalFileParams::Read {
                file_path: "/tmp/screenshot.png".to_string(),
                offset: None,
                limit: None,
                mode: None,
                indentation: None,
            }
        );

        // @step And the facade tool name is "view_image"
        assert_eq!(facade.tool_name(), "view_image");

        // @step And the facade provider is "codex"
        assert_eq!(facade.provider(), "codex");
    }

    /// Scenario: CodexViewImageFacade accepts detail param for compatibility
    #[test]
    fn test_codex_view_image_facade_accepts_detail_param() {
        // @step Given a CodexViewImageFacade instance
        let facade = CodexViewImageFacade;

        // @step When the Codex model calls view_image with path and detail "original"
        let input = json!({
            "path": "/tmp/image.jpg",
            "detail": "original"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalFileParams::Read ignoring the detail param
        assert_eq!(
            result,
            InternalFileParams::Read {
                file_path: "/tmp/image.jpg".to_string(),
                offset: None,
                limit: None,
                mode: None,
                indentation: None,
            }
        );
    }

    /// Scenario: CodexViewImageFacade rejects missing path
    #[test]
    fn test_codex_view_image_facade_missing_path() {
        // @step Given a CodexViewImageFacade instance
        let facade = CodexViewImageFacade;

        // @step When view_image is called with no path parameter
        let input = json!({});
        let result = facade.map_params(input);

        // @step Then the facade returns a validation error for tool "view_image" mentioning "path"
        assert!(result.is_err());
        if let Err(ToolError::Validation { tool, message }) = result {
            assert_eq!(tool, "view_image");
            assert!(message.contains("path"));
        } else {
            panic!("Expected ToolError::Validation");
        }
    }

    /// Scenario: CodexViewImageFacade rejects empty path
    #[test]
    fn test_codex_view_image_facade_empty_path() {
        // @step Given a CodexViewImageFacade instance
        let facade = CodexViewImageFacade;

        // @step When view_image is called with an empty path
        let input = json!({
            "path": ""
        });
        let result = facade.map_params(input);

        // @step Then the facade returns an error
        assert!(result.is_err());
    }

    /// Scenario: Tool definition matches Codex CLI spec
    #[test]
    fn test_codex_view_image_schema_matches_codex_spec() {
        // @step Given a CodexViewImageFacade instance
        let facade = CodexViewImageFacade;

        // @step When the tool definition is requested
        let def = facade.definition();

        // @step Then the tool name is "view_image"
        assert_eq!(def.name, "view_image");

        // @step And the description mentions viewing a local image
        assert!(def.description.contains("View a local image"));

        // @step And the parameters schema has a required "path" property of type string
        assert_eq!(def.parameters["properties"]["path"]["type"], "string");
        assert_eq!(def.parameters["required"], json!(["path"]));

        // @step And additionalProperties is false
        assert_eq!(def.parameters["additionalProperties"], false);

        // @step And a "detail" property exists for model compatibility
        assert!(def.parameters["properties"]["detail"].is_object());
    }

    // =========================================================================
    // BUG-114: shell facade tests (ExecToolFacade)
    // Feature: spec/features/codex-shell-exec-facades.feature
    // =========================================================================

    /// Scenario: CodexShellFacade maps shell command array to InternalExecParams::Run
    #[test]
    fn test_codex_shell_facade_maps_command_array() {
        // @step Given a CodexShellFacade instance
        let facade = CodexShellFacade;

        // @step When the Codex model calls shell with command ["ls", "-la"] and workdir "/tmp"
        let input = json!({
            "command": ["ls", "-la"],
            "workdir": "/tmp"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalExecParams::Run with command as JSON array ["ls", "-la"]
        // @step And tty is false
        // @step And workdir is "/tmp"
        assert_eq!(
            result,
            InternalExecParams::Run {
                command: json!(["ls", "-la"]),
                workdir: Some("/tmp".to_string()),
                tty: false,
                yield_time_ms: None,
                max_output_tokens: None,
                timeout_secs: None,
            }
        );

        // @step And the facade tool name is "shell"
        assert_eq!(facade.tool_name(), "shell");

        // @step And the facade provider is "codex"
        assert_eq!(facade.provider(), "codex");
    }

    /// Scenario: CodexShellFacade converts timeout_ms to timeout_secs
    #[test]
    fn test_codex_shell_facade_converts_timeout() {
        // @step Given a CodexShellFacade instance
        let facade = CodexShellFacade;

        // @step When the Codex model calls shell with command ["git", "status"] and timeout_ms 5000
        let input = json!({
            "command": ["git", "status"],
            "timeout_ms": 5000
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalExecParams::Run with timeout_secs 5
        if let InternalExecParams::Run { timeout_secs, .. } = &result {
            assert_eq!(*timeout_secs, Some(5));
        } else {
            panic!("Expected InternalExecParams::Run");
        }
    }

    /// Scenario: CodexShellFacade without optional params defaults to None
    #[test]
    fn test_codex_shell_facade_defaults_to_none() {
        // @step Given a CodexShellFacade instance
        let facade = CodexShellFacade;

        // @step When the Codex model calls shell with only command ["echo", "hello"]
        let input = json!({
            "command": ["echo", "hello"]
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalExecParams::Run with workdir None
        // @step And timeout_secs is None
        // @step And yield_time_ms is None
        assert_eq!(
            result,
            InternalExecParams::Run {
                command: json!(["echo", "hello"]),
                workdir: None,
                tty: false,
                yield_time_ms: None,
                max_output_tokens: None,
                timeout_secs: None,
            }
        );
    }

    /// Scenario: CodexShellFacade validates required command parameter
    #[test]
    fn test_codex_shell_facade_missing_command() {
        // @step Given a CodexShellFacade instance
        let facade = CodexShellFacade;

        // @step When the Codex model calls shell with missing command field
        let input = json!({});
        let result = facade.map_params(input);

        // @step Then the facade returns a validation error for tool "shell" mentioning "command"
        assert!(result.is_err());
        if let Err(ToolError::Validation { tool, message }) = result {
            assert_eq!(tool, "shell");
            assert!(message.contains("command"));
        } else {
            panic!("Expected ToolError::Validation");
        }
    }

    /// Scenario: CodexShellFacade rejects empty command array
    #[test]
    fn test_codex_shell_facade_empty_command_array() {
        let facade = CodexShellFacade;
        let input = json!({
            "command": []
        });
        let result = facade.map_params(input);
        assert!(result.is_err());
    }

    /// Scenario: CodexShellFacade rejects non-array command
    #[test]
    fn test_codex_shell_facade_non_array_command() {
        let facade = CodexShellFacade;
        let input = json!({
            "command": "ls -la"
        });
        let result = facade.map_params(input);
        assert!(result.is_err());
    }

    /// Scenario: CodexShellFacade schema has additionalProperties false
    #[test]
    fn test_codex_shell_facade_schema() {
        // @step Given a CodexShellFacade instance
        let facade = CodexShellFacade;

        // @step When the tool definition schema is inspected
        let def = facade.definition();

        // @step Then the schema has additionalProperties set to false
        assert_eq!(def.parameters["additionalProperties"], false);

        // @step And the required array contains only "command"
        assert_eq!(def.parameters["required"], json!(["command"]));

        // @step And command property type is "array" with items type "string"
        assert_eq!(def.parameters["properties"]["command"]["type"], "array");
        assert_eq!(
            def.parameters["properties"]["command"]["items"]["type"],
            "string"
        );
    }

    // =========================================================================
    // BUG-114: exec_command facade tests (ExecToolFacade)
    // Feature: spec/features/codex-shell-exec-facades.feature
    // =========================================================================

    /// Scenario: CodexExecCommandFacade maps exec_command with PTY to InternalExecParams::Run
    #[test]
    fn test_codex_exec_command_facade_maps_pty() {
        // @step Given a CodexExecCommandFacade instance
        let facade = CodexExecCommandFacade;

        // @step When the Codex model calls exec_command with cmd "python3" tty true and yield_time_ms 5000
        let input = json!({
            "cmd": "python3",
            "tty": true,
            "yield_time_ms": 5000
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalExecParams::Run with command as string "python3"
        // @step And tty is true
        // @step And yield_time_ms is 5000
        assert_eq!(
            result,
            InternalExecParams::Run {
                command: Value::String("python3".to_string()),
                workdir: None,
                tty: true,
                yield_time_ms: Some(5000),
                max_output_tokens: None,
                timeout_secs: None,
            }
        );

        // @step And the facade tool name is "exec_command"
        assert_eq!(facade.tool_name(), "exec_command");

        // @step And the facade provider is "codex"
        assert_eq!(facade.provider(), "codex");
    }

    /// Scenario: CodexExecCommandFacade defaults to tty false when not specified
    #[test]
    fn test_codex_exec_command_facade_defaults_tty_false() {
        // @step Given a CodexExecCommandFacade instance
        let facade = CodexExecCommandFacade;

        // @step When the Codex model calls exec_command with only cmd "ls"
        let input = json!({
            "cmd": "ls"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalExecParams::Run with tty false
        // @step And yield_time_ms is None
        // @step And max_output_tokens is None
        assert_eq!(
            result,
            InternalExecParams::Run {
                command: Value::String("ls".to_string()),
                workdir: None,
                tty: false,
                yield_time_ms: None,
                max_output_tokens: None,
                timeout_secs: None,
            }
        );
    }

    /// Scenario: CodexExecCommandFacade maps all optional params
    #[test]
    fn test_codex_exec_command_facade_all_params() {
        // @step Given a CodexExecCommandFacade instance
        let facade = CodexExecCommandFacade;

        // @step When the Codex model calls exec_command with cmd "python3" workdir "/app" tty true yield_time_ms 10000 and max_output_tokens 4096
        let input = json!({
            "cmd": "python3",
            "workdir": "/app",
            "tty": true,
            "yield_time_ms": 10000,
            "max_output_tokens": 4096
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalExecParams::Run with command "python3"
        // @step And workdir is "/app"
        // @step And tty is true
        // @step And yield_time_ms is 10000
        // @step And max_output_tokens is 4096
        assert_eq!(
            result,
            InternalExecParams::Run {
                command: Value::String("python3".to_string()),
                workdir: Some("/app".to_string()),
                tty: true,
                yield_time_ms: Some(10000),
                max_output_tokens: Some(4096),
                timeout_secs: None,
            }
        );
    }

    /// Scenario: CodexExecCommandFacade validates required cmd parameter
    #[test]
    fn test_codex_exec_command_facade_missing_cmd() {
        // @step Given a CodexExecCommandFacade instance
        let facade = CodexExecCommandFacade;

        // @step When the Codex model calls exec_command with missing cmd field
        let input = json!({});
        let result = facade.map_params(input);

        // @step Then the facade returns a validation error for tool "exec_command" mentioning "cmd"
        assert!(result.is_err());
        if let Err(ToolError::Validation { tool, message }) = result {
            assert_eq!(tool, "exec_command");
            assert!(message.contains("cmd"));
        } else {
            panic!("Expected ToolError::Validation");
        }
    }

    /// Scenario: CodexExecCommandFacade silently ignores Codex-native approval params
    #[test]
    fn test_codex_exec_command_facade_ignores_approval_params() {
        // @step Given a CodexExecCommandFacade instance
        let facade = CodexExecCommandFacade;

        // @step When the Codex model calls exec_command with cmd "ls" and login true and shell "/bin/bash"
        let input = json!({
            "cmd": "ls",
            "login": true,
            "shell": "/bin/bash"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalExecParams::Run with command "ls"
        // @step And tty is false
        assert_eq!(
            result,
            InternalExecParams::Run {
                command: Value::String("ls".to_string()),
                workdir: None,
                tty: false,
                yield_time_ms: None,
                max_output_tokens: None,
                timeout_secs: None,
            }
        );
    }

    /// Scenario: CodexExecCommandFacade schema has additionalProperties false
    #[test]
    fn test_codex_exec_command_facade_schema() {
        // @step Given a CodexExecCommandFacade instance
        let facade = CodexExecCommandFacade;

        // @step When the tool definition schema is inspected
        let def = facade.definition();

        // @step Then the schema has additionalProperties set to false
        assert_eq!(def.parameters["additionalProperties"], false);

        // @step And the required array contains only "cmd"
        assert_eq!(def.parameters["required"], json!(["cmd"]));

        // @step And the schema has properties for cmd workdir shell tty yield_time_ms max_output_tokens and login
        assert!(def.parameters["properties"]["cmd"].is_object());
        assert!(def.parameters["properties"]["workdir"].is_object());
        assert!(def.parameters["properties"]["shell"].is_object());
        assert!(def.parameters["properties"]["tty"].is_object());
        assert!(def.parameters["properties"]["yield_time_ms"].is_object());
        assert!(def.parameters["properties"]["max_output_tokens"].is_object());
        assert!(def.parameters["properties"]["login"].is_object());
    }

    /// Scenario: CodexExecCommandFacade rejects empty cmd
    #[test]
    fn test_codex_exec_command_facade_empty_cmd() {
        let facade = CodexExecCommandFacade;
        let input = json!({
            "cmd": ""
        });
        let result = facade.map_params(input);
        assert!(result.is_err());
    }

    /// Feature: spec/features/codex-shell-exec-facades.feature
    ///
    /// Scenario: Both facades are registered in Codex create_rig_agent
    #[test]
    fn test_both_facades_tool_names_and_schemas() {
        // @step Given a CodexShellFacade and CodexExecCommandFacade instance
        let shell_facade = CodexShellFacade;
        let exec_facade = CodexExecCommandFacade;

        // @step When the Codex tool name list is inspected
        let shell_def = shell_facade.definition();
        let exec_def = exec_facade.definition();

        // @step Then "shell" is present and maps command as array type
        assert_eq!(shell_def.name, "shell");
        assert_eq!(shell_def.parameters["properties"]["command"]["type"], "array");

        // @step And "exec_command" is present and maps cmd as string type
        assert_eq!(exec_def.name, "exec_command");
        assert_eq!(exec_def.parameters["properties"]["cmd"]["type"], "string");
    }

    // =========================================================================
    // BUG-115: write_stdin facade tests
    // Feature: spec/features/codex-write-stdin-facade.feature
    // =========================================================================

    /// Scenario: CodexWriteStdinFacade maps non-empty chars to InternalExecParams::Write
    #[test]
    fn test_codex_write_stdin_maps_non_empty_chars_to_write() {
        // @step Given a CodexWriteStdinFacade instance
        let facade = CodexWriteStdinFacade;

        // @step When the Codex model calls write_stdin with session_id 4237 and chars "print(42)\n"
        let input = json!({
            "session_id": 4237,
            "chars": "print(42)\n"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalExecParams::Write with session_id "4237" and input "print(42)\n"
        // @step And yield_time_ms is None
        // @step And max_output_tokens is None
        assert_eq!(
            result,
            InternalExecParams::Write {
                session_id: "4237".to_string(),
                input: "print(42)\n".to_string(),
                yield_time_ms: None,
                max_output_tokens: None,
            }
        );
    }

    /// Scenario: CodexWriteStdinFacade passes all optional params through
    #[test]
    fn test_codex_write_stdin_passes_optional_params() {
        // @step Given a CodexWriteStdinFacade instance
        let facade = CodexWriteStdinFacade;

        // @step When the Codex model calls write_stdin with session_id 4237 chars "exit()\n" yield_time_ms 5000 and max_output_tokens 1024
        let input = json!({
            "session_id": 4237,
            "chars": "exit()\n",
            "yield_time_ms": 5000,
            "max_output_tokens": 1024
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalExecParams::Write with session_id "4237" and input "exit()\n"
        // @step And yield_time_ms is 5000
        // @step And max_output_tokens is 1024
        assert_eq!(
            result,
            InternalExecParams::Write {
                session_id: "4237".to_string(),
                input: "exit()\n".to_string(),
                yield_time_ms: Some(5000),
                max_output_tokens: Some(1024),
            }
        );
    }

    /// Scenario: CodexWriteStdinFacade maps empty chars to InternalExecParams::Poll
    #[test]
    fn test_codex_write_stdin_maps_empty_chars_to_poll() {
        // @step Given a CodexWriteStdinFacade instance
        let facade = CodexWriteStdinFacade;

        // @step When the Codex model calls write_stdin with session_id 4237 and chars ""
        let input = json!({
            "session_id": 4237,
            "chars": ""
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalExecParams::Poll with session_id "4237"
        // @step And yield_time_ms is None
        // @step And max_output_tokens is None
        assert_eq!(
            result,
            InternalExecParams::Poll {
                session_id: "4237".to_string(),
                yield_time_ms: None,
                max_output_tokens: None,
            }
        );
    }

    /// Scenario: CodexWriteStdinFacade maps absent chars to InternalExecParams::Poll
    #[test]
    fn test_codex_write_stdin_maps_absent_chars_to_poll() {
        // @step Given a CodexWriteStdinFacade instance
        let facade = CodexWriteStdinFacade;

        // @step When the Codex model calls write_stdin with session_id 4237 and no chars field
        let input = json!({
            "session_id": 4237
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalExecParams::Poll with session_id "4237"
        assert_eq!(
            result,
            InternalExecParams::Poll {
                session_id: "4237".to_string(),
                yield_time_ms: None,
                max_output_tokens: None,
            }
        );
    }

    /// Scenario: CodexWriteStdinFacade converts numeric session_id to string
    #[test]
    fn test_codex_write_stdin_converts_numeric_session_id() {
        // @step Given a CodexWriteStdinFacade instance
        let facade = CodexWriteStdinFacade;

        // @step When the Codex model calls write_stdin with session_id 99 and chars "hello"
        let input = json!({
            "session_id": 99,
            "chars": "hello"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalExecParams::Write with session_id "99" and input "hello"
        assert_eq!(
            result,
            InternalExecParams::Write {
                session_id: "99".to_string(),
                input: "hello".to_string(),
                yield_time_ms: None,
                max_output_tokens: None,
            }
        );
    }

    /// Scenario: CodexWriteStdinFacade validates required session_id parameter
    #[test]
    fn test_codex_write_stdin_validates_required_session_id() {
        // @step Given a CodexWriteStdinFacade instance
        let facade = CodexWriteStdinFacade;

        // @step When the Codex model calls write_stdin without session_id
        let input = json!({
            "chars": "hello"
        });
        let result = facade.map_params(input);

        // @step Then the facade returns a validation error for tool "write_stdin" mentioning "session_id"
        assert!(result.is_err());
        if let Err(ToolError::Validation { tool, message }) = result {
            assert_eq!(tool, "write_stdin");
            assert!(message.contains("session_id"));
        } else {
            panic!("Expected ToolError::Validation");
        }
    }

    /// Scenario: CodexWriteStdinFacade rejects null session_id
    #[test]
    fn test_codex_write_stdin_rejects_null_session_id() {
        // @step Given a CodexWriteStdinFacade instance
        let facade = CodexWriteStdinFacade;

        // @step When the Codex model calls write_stdin with session_id null
        let input = json!({
            "session_id": null
        });
        let result = facade.map_params(input);

        // @step Then the facade returns a validation error for tool "write_stdin" mentioning "session_id"
        assert!(result.is_err());
        if let Err(ToolError::Validation { tool, message }) = result {
            assert_eq!(tool, "write_stdin");
            assert!(message.contains("session_id"));
        } else {
            panic!("Expected ToolError::Validation");
        }
    }

    /// Scenario: CodexWriteStdinFacade schema has additionalProperties false
    #[test]
    fn test_codex_write_stdin_schema() {
        // @step Given a CodexWriteStdinFacade instance
        let facade = CodexWriteStdinFacade;

        // @step When the tool definition schema is inspected
        let def = facade.definition();

        // @step Then the schema has additionalProperties set to false
        assert_eq!(def.parameters["additionalProperties"], false);

        // @step And the schema has "session_id" in the required array
        assert_eq!(def.parameters["required"], json!(["session_id"]));

        // @step And the tool name is "write_stdin"
        assert_eq!(def.name, "write_stdin");
        assert_eq!(facade.tool_name(), "write_stdin");

        // @step And the provider is "codex"
        assert_eq!(facade.provider(), "codex");
    }

    /// Scenario: write_stdin facade is registered in Codex create_rig_agent
    #[test]
    fn test_codex_write_stdin_facade_tool_name_and_schema() {
        // @step Given a CodexWriteStdinFacade instance registered via ExecToolFacadeWrapper
        let facade = CodexWriteStdinFacade;

        // @step When the Codex agent tool list is inspected
        let def = facade.definition();

        // @step Then the tool list contains "write_stdin"
        assert_eq!(def.name, "write_stdin");
        assert!(def.parameters["properties"]["session_id"].is_object());
        assert!(def.parameters["properties"]["chars"].is_object());
        assert!(def.parameters["properties"]["yield_time_ms"].is_object());
        assert!(def.parameters["properties"]["max_output_tokens"].is_object());
    }

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
