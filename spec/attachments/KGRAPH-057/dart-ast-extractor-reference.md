# Dart AST Extractor — Implementation Reference

## Dart Language Syntax for AST Extraction

This document captures the Dart-specific syntax patterns the AST extractor needs to handle for `ast_dart_extractor.rs`.

---

## 1. Functions to Extract

### Top-Level Functions
```dart
void main() { ... }
Future<void> runApp(Widget app) async { ... }
int calculateSum(int a, int b) => a + b;
T identity<T>(T value) => value;
```

### Methods (inside classes)
```dart
class MyClass {
  void doSomething() { ... }
  static Future<String> fetchData() async { ... }
  int get count => _items.length;        // getter
  set count(int value) { _count = value; } // setter
}
```

### Constructors
```dart
class User {
  User(this.name, this.age);                   // generative constructor
  User.fromJson(Map<String, dynamic> json);    // named constructor
  factory User.create(String name) { ... }     // factory constructor
  const User.empty() : name = '', age = 0;     // const constructor
}
```

### Extension Methods
```dart
extension StringExtensions on String {
  bool get isBlank => trim().isEmpty;
  String capitalize() => '${this[0].toUpperCase()}${substring(1)}';
}
```

### tree-sitter node kinds for functions:
- `function_signature` + `function_body` (top-level)
- `method_signature` (inside class)
- `constructor_signature` (constructors)
- `getter_signature`, `setter_signature`
- `function_expression` (lambdas/closures — skip these, not named)

---

## 2. Types to Extract

### Classes
```dart
class Animal { ... }
abstract class Shape { ... }
abstract interface class Printable { ... }  // Dart 3.0
sealed class Result { ... }                  // Dart 3.0
base class Base { ... }                      // Dart 3.0
final class Singleton { ... }               // Dart 3.0
```

### Enums
```dart
enum Color { red, green, blue }
enum Planet {
  mercury(3.303e+23, 2.4397e6),
  venus(4.869e+24, 6.0518e6);
  final double mass;
  final double radius;
  const Planet(this.mass, this.radius);
}
```

### Mixins
```dart
mixin Musical {
  bool canPlayPiano = false;
  void entertainMe() { ... }
}
mixin class Musician { ... }  // Dart 3.0 — both mixin and class
```

### Extensions and Extension Types
```dart
extension NumberParsing on String { ... }
extension type IdNumber(int id) { ... }      // Dart 3.3 extension types
```

### Typedefs
```dart
typedef IntList = List<int>;
typedef Compare<T> = int Function(T a, T b);
```

### tree-sitter node kinds for types:
- `class_definition` (includes abstract, sealed, base, final, interface modifiers)
- `enum_declaration`
- `mixin_declaration`
- `extension_declaration`
- `extension_type_declaration`
- `type_alias` (typedefs)

---

## 3. Import Statements → Imports Edges

### Dart import syntax
```dart
import 'dart:math';                           // SDK library (external — skip)
import 'package:flutter/material.dart';       // Package import (external — skip)
import 'package:my_app/models/user.dart';     // Package import (local if same package)
import '../models/user.dart';                 // Relative import (local)
import 'utils.dart';                          // Relative import (local)
export 'src/widget.dart';                     // Re-export (local if relative)
part 'src/generated.dart';                    // Part directive (local)
part of 'library.dart';                       // Part-of directive
```

### Resolution rules
- **Relative imports** (`'../foo.dart'`, `'./bar.dart'`, `'baz.dart'`): Always local → create Imports edge
- **`package:` imports**: Local only if the package name matches the project's `pubspec.yaml` `name` field. For simplicity, treat all `package:` imports as external unless we want to parse pubspec.yaml.
- **`dart:` imports**: Always external SDK → skip
- **`export` and `part`**: Treat like imports for edge purposes

### tree-sitter node kinds:
- `import_or_export` → contains `import_specification` or `library_import`
- The URI is inside a `string_literal` child

---

## 4. Call Expressions → Calls Edges

### Function calls
```dart
print('hello');                     // top-level function call
calculateSum(1, 2);                 // user function call
await fetchData();                  // async call
List.generate(10, (i) => i);       // static method call
```

### Method calls
```dart
myObject.doSomething();             // instance method
widget.build(context);              // method call
super.initState();                  // super call
this._validate();                   // explicit this
```

### Constructor invocations
```dart
final user = User('Alice', 30);     // constructor call
final user = User.fromJson(data);   // named constructor
final widget = const Text('hi');    // const constructor
```

### Cascade notation
```dart
canvas
  ..drawRect(rect)                   // each cascade is a separate call
  ..drawCircle(center, radius, paint);
```

### tree-sitter node kinds:
- `function_expression_invocation` — `foo(args)`
- `method_invocation` — `obj.method(args)`, `super.method(args)`
- `new_expression` — `User(args)`, `User.named(args)`
- `cascade_section` → contains identifier of called method

---

## 5. Type Annotations → TypeRef Edges

### In function signatures
```dart
String greet(String name, {int? age}) { ... }
Future<List<User>> fetchUsers() async { ... }
Stream<int> countDown(int from) async* { ... }
FutureOr<T> compute<T>(T Function() callback) { ... }
```

### In class definitions
```dart
class Dog extends Animal implements Comparable<Dog>, Serializable { ... }
class MyWidget extends StatefulWidget with TickerProviderStateMixin { ... }
mixin Swimmer on Animal { ... }   // type constraint
```

### Resolution
Extract type names from:
- Function parameter types
- Function return types
- `extends` / `implements` / `with` / `on` clauses
- Generic type arguments (inner types)

---

## 6. pubspec.yaml Dependency Extractor

### Format
```yaml
name: my_flutter_app
version: 1.0.0

dependencies:
  flutter:
    sdk: flutter
  provider: ^6.0.0
  http: ^1.1.0
  json_annotation: ^4.8.1

dev_dependencies:
  flutter_test:
    sdk: flutter
  build_runner: ^2.4.0
  json_serializable: ^6.7.1

dependency_overrides:
  some_package:
    path: ../local_package
```

### Extraction
- Parse `dependencies:` and `dev_dependencies:` sections
- For each key: create a Dependency entity with:
  - `name`: package name
  - `version`: version constraint string
  - `source`: `"pubspec.yaml"`
  - `isDev`: true for dev_dependencies

---

## 7. Expando Character

Dart uses `$` for string interpolation:
```dart
var greeting = 'Hello, $name!';
var result = 'Sum is ${a + b}';
```

So `$VAR` is NOT valid in all Dart contexts. Use `µ` (mu) as expando_char, consistent with Go, Python, Ruby, Kotlin, Swift in ast-grep.

---

## 8. Reference Extractors

Follow the pattern of these existing extractors:
- `ast_kotlin_extractor.rs` — closest language family (JVM, similar class/interface/enum model)
- `ast_swift_extractor.rs` — similar extension model, protocol/mixin concepts
- `ast_ts_extractor.rs` — reference for Calls/Imports/TypeRef edge extraction quality
