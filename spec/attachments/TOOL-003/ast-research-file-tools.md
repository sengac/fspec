# AST Research: File Operation Tools

## ReadTool (codelet/tools/src/read.rs)
- **Tool name**: `read`
- **Args**: `file_path` (required), `offset` (optional), `limit` (optional)
- **Description**: Reads file with optional line offset and limit

## WriteTool (codelet/tools/src/write.rs)
- **Tool name**: `write`
- **Args**: `file_path` (required), `content` (required)
- **Description**: Writes content to file, creates parent directories

## EditTool (codelet/tools/src/edit.rs)
- **Tool name**: `edit`
- **Args**: `file_path` (required), `old_string` (required), `new_string` (required)
- **Description**: Replaces first occurrence of old_string with new_string

## Gemini CLI Tool Names (from design document)
According to Gemini CLI repository:
- `read_file` (not `read`)
- `write_file` (not `write`)
- `replace` (not `edit`)

## Facade Requirements
Create facades that:
1. Map Gemini tool names to internal tool names
2. Provide flat schemas (no nested objects)
3. Map parameters to internal Args types
