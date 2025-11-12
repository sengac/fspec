; Query: list-functions
; Description: Find all function declarations, expressions, arrow functions, methods, and generators

[
  (function_declaration
    name: (identifier) @name) @function.declaration

  (function_expression
    name: (identifier)? @name) @function.expression

  (arrow_function) @function.arrow

  (method_definition
    name: (property_identifier) @name) @function.method

  (generator_function_declaration
    name: (identifier) @name) @function.generator
]
