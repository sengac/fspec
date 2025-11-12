; Query: list-exports
; Description: Find all export statements

(export_statement) @export

; Query: find-exports-default
; Description: Find default export statement

(export_statement
  "default") @export.default
