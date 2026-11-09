# BUG-369 AST research

## fspec astgrep --pattern 'Uuid::parse_str($A).unwrap_or_else($B)' --lang rust --path rust/sessions/src
rust/sessions/src/handle_impl.rs:55:5:Uuid::parse_str(id.value.as_str()).unwrap_or_else(|_| Uuid::nil())


The exec_stdin.rs lock sites are lines 15 (`if let Ok(mut guard) = self.exec_stdin_request.write()`) and 62 (`self.exec_stdin_request.read().ok()?`).
