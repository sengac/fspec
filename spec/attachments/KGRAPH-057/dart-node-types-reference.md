# tree-sitter-dart Node Types Reference

Generated from `tree-sitter-dart` v0.1.0 (`nielsenko/tree-sitter-dart`).

## Node Types Relevant to AST Extraction

### Functions / Methods
| Node Kind | Example | Extract As |
|-----------|---------|------------|
| `function_signature` | `void main()` (top-level) | Function |
| `method_signature` | `void doSomething()` (in class) | Function |
| `constructor_signature` | `MyClass(this.x)` | Function |
| `constant_constructor_signature` | `const MyClass.empty()` | Function |
| `factory_constructor_signature` | `factory MyClass.create()` | Function |
| `redirecting_factory_constructor_signature` | `factory MyClass = _Impl` | Function |
| `getter_signature` | `int get count` | Function |
| `setter_signature` | `set count(int value)` | Function |
| `operator_signature` | `bool operator ==(Object other)` | Function |
| `function_expression` | `(x) => x + 1` (anonymous — skip) | — |
| `local_function_declaration` | `void helper() { ... }` (inside body — skip) | — |

### Types / Declarations
| Node Kind | Example | Extract As |
|-----------|---------|------------|
| `class_declaration` | `class Foo { }`, `abstract class Bar { }`, `sealed class Baz { }` | Type |
| `enum_declaration` | `enum Color { red, green, blue }` | Type |
| `mixin_declaration` | `mixin Musical { }` | Type |
| `extension_declaration` | `extension StringExt on String { }` | Type |
| `extension_type_declaration` | `extension type Id(int value) { }` | Type |
| `type_alias` | `typedef IntList = List<int>` | Type |
| `mixin_application_class` | `class C = S with M;` | Type |

### Imports
| Node Kind | Example | Notes |
|-----------|---------|-------|
| `import_or_export` | Container for import/export | Parent node |
| `library_import` | `import 'package:foo/bar.dart';` | Contains URI |
| `library_export` | `export 'src/widget.dart';` | Contains URI |
| `import_specification` | URI specification | Contains the `uri` child |
| `part_directive` | `part 'src/generated.dart';` | File part |
| `part_of_directive` | `part of 'library.dart';` | Reverse part |
| `uri` | `'dart:math'`, `'../foo.dart'` | The actual import path |
| `combinator` | `show Foo, Bar` / `hide Baz` | Filter (informational only) |

### Calls
| Node Kind | Example | Notes |
|-----------|---------|-------|
| `function_expression_invocation` | `foo(args)` | Direct function call |
| `method_invocation` → via `selector` | `obj.method(args)` | Method call on object |
| `new_expression` | `User(args)`, `User.named(args)` | Constructor invocation |
| `constructor_invocation` | Inside const expressions | Constructor call |
| `cascade_section` | `..method(args)` | Cascade call |
| `super` + selector | `super.method(args)` | Super call |

### Type References
| Node Kind | Example | Notes |
|-----------|---------|-------|
| `type_identifier` | `String`, `int`, `User` | Named type reference |
| `type_arguments` | `<String, int>` | Generic type args |
| `superclass` | `extends Animal` | Superclass clause |
| `interfaces` | `implements Comparable, Serializable` | Interface clause |
| `mixins` | `with TickerProviderMixin` | Mixin clause |
| `type_parameters` | `<T extends Comparable>` | Type param constraints |
| `void_type` | `void` | Built-in (skip) |
| `function_type` | `void Function(int)` | Function type (extract inner types) |

---

## All 204 Named Node Types

<details>
<summary>Full list (click to expand)</summary>

```
_declaration
_literal
_statement
additive_expression
annotation
annotation_arguments
annotation_open_paren
argument
argument_part
arguments
assert_statement
assertion
assignable_expression
assignment_expression
await_expression
binary_operator
bitwise_and_expression
bitwise_or_expression
bitwise_xor_expression
block
block_comment
break_statement
cascade_section
cascade_selector
cast_pattern
catch_clause
class_body
class_declaration
class_member
combinator
comment
conditional_assignable_selector
conditional_expression
configurable_uri
configuration_uri
const_object_expression
constant_constructor_signature
constant_pattern
constructor_invocation
constructor_param
constructor_signature
constructor_tearoff
continue_statement
decimal_floating_point_literal
decimal_integer_literal
declaration
do_statement
documentation_block_comment
dotted_identifier_list
empty_statement
enum_body
enum_constant
enum_declaration
equality_expression
escape_sequence
expression_statement
extension_body
extension_declaration
extension_type_declaration
extension_type_name
extension_type_representation
external
factory_constructor_signature
false
field_initializer
finally_clause
for_element
for_statement
formal_parameter
formal_parameter_list
function_body
function_expression
function_expression_body
function_signature
function_type
getter_signature
hex_integer_literal
identifier
identifier_dollar_escaped
identifier_list
if_element
if_null_expression
if_statement
import_or_export
import_specification
initialized_identifier
initialized_identifier_list
initialized_variable_definition
initializer_list_entry
initializers
interfaces
is_operator
label
labeled_statement
library_export
library_import
library_name
list_literal
list_pattern
local_function_declaration
local_variable_declaration
logical_and_expression
logical_or_expression
map_pattern
method_signature
mixin_application
mixin_application_class
mixin_declaration
mixins
multiplicative_expression
named_argument
named_parameter_types
native
negate_operator
new_expression
normal_parameter_type
null_assert_pattern
null_aware_element
object_pattern
operator_signature
optional_formal_parameters
optional_parameter_types
optional_positional_parameter_types
pair
parameter_type_list
parenthesized_expression
part_directive
part_of_directive
pattern_assignment
pattern_variable_declaration
postfix_expression
prefix_operator
qualified
raw_string_literal_double_quotes
raw_string_literal_double_quotes_multiple
raw_string_literal_single_quotes
raw_string_literal_single_quotes_multiple
record_field
record_literal
record_pattern
record_type
record_type_field
record_type_named_field
redirecting_factory_constructor_signature
redirection
relational_expression
relational_operator
rest_pattern
return_statement
script_tag
selector
set_or_map_literal
setter_signature
shift_expression
source_file
spread_element
static_final_declaration
static_final_declaration_list
static_member_shorthand
string_literal
string_literal_double_quotes
string_literal_double_quotes_multiple
string_literal_single_quotes
string_literal_single_quotes_multiple
super_formal_parameter
superclass
switch_block
switch_expression
switch_expression_case
switch_statement
switch_statement_case
switch_statement_default
symbol_literal
template_chars_double
template_chars_double_single
template_chars_raw_slash
template_chars_single
template_chars_single_single
template_substitution
throw_expression
true
try_statement
type_alias
type_arguments
type_cast
type_cast_expression
type_identifier
type_parameter
type_parameters
type_test
type_test_expression
typed_identifier
unary_expression
unconditional_assignable_selector
uri
uri_test
variable_pattern
variance_modifier
void_type
while_statement
yield_each_statement
yield_statement
```

</details>
