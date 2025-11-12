; Query: find-async-functions
; Description: Find all async function declarations

(function_declaration
  "async"
  name: (identifier) @name) @function.async
