; Query: list-classes
; Description: Find all class declarations

(class_declaration
  name: (identifier) @name
  body: (class_body) @body) @class

; Query: find-class
; Description: Find a specific class by name (requires --name parameter)

(class_declaration
  name: (identifier) @name
  body: (class_body) @body) @class
