# Dart AST Extractor — False Positive Analysis

## Test Subject

**Project:** `fspec-mobile` — a Flutter/Dart cross-platform mobile app  
**Indexing Results:** 141 files, 776 functions, 204 types, 25 dependencies, 1,762 edges  
**Dead Code Report:** 31 dead files, 100 dead functions, 69 dead types  
**Verified Genuine Dead Code:** 0 dead files, ~17 dead functions, 2 dead types  
**False Positive Rate:** ~85%

---

## Root Cause Categories

### 1. `main.dart` Flagged as Dead File (Entry Point Not Recognized)

**File:** `lib/main.dart` (18 lines)  
**Why False Positive:** Flutter's entry point `main()` is invoked by the Flutter runtime, not by any Dart import. No file imports `main.dart`.  

**Root Cause:** The `orphan_files` query finds files with no incoming `Imports` edges. Framework entry points are never imported — they're invoked by the runtime.

**Fix:** The Dart extractor should detect `main.dart` (or files containing a top-level `void main()`) and mark them as entry points. Either:
- Exclude `main.dart` from orphan file results in the dead code filter
- Or create a synthetic incoming edge to mark it as "used by framework"
- Alternatively, the dead code dispatch layer (`ast_dispatch.rs`) could filter `main.dart` for Dart projects (similar to how test files are already excluded)

---

### 2. Platform Runner Files Flagged as Dead (30 files)

**Files:** All files under `macos/`, `ios/`, `linux/`, `windows/`, `android/`  
**Examples:** `AppDelegate.swift`, `main.cc`, `flutter_window.cpp`, `GeneratedPluginRegistrant.java`, `MainActivity.kt`, `build.gradle.kts`, `settings.gradle.kts`

**Root Cause:** These are native platform entry points compiled by Xcode/Gradle/CMake — they're never imported by Dart code. The extractor indexes them because they're non-test source files, but no Dart file references them.

**Fix:** Two options:
1. **Best:** Detect Flutter projects (presence of `pubspec.yaml` with `flutter` dependency) and auto-exclude platform directories (`ios/`, `android/`, `macos/`, `linux/`, `windows/`) from dead code analysis since they're all framework boilerplate
2. **Alternative:** Add a config mechanism for excluding directories from dead code analysis

---

### 3. `pubspec.yaml` Flagged as Dead File

**Root Cause:** `pubspec.yaml` is treated as a File node but nothing imports it. It's the package manifest.

**Fix:** The dead code filter already excludes files with no `language` set. Ensure `pubspec.yaml` either gets no language or is specifically excluded (config/manifest files should never appear in dead code).

---

### 4. Freezed `*Patterns` Extension Types Flagged as Dead (14 types)

**Types:** `AppErrorPatterns`, `ConnectionPatterns`, `WorkUnitPatterns`, `BoardColumnsPatterns`, `BoardDataPatterns`, `UserStoryPatterns`, `RulePatterns`, `ExamplePatterns`, `QuestionPatterns`, `WorkUnitDetailPatterns`, `StreamChunkPatterns`, `ToolCallWithResultPatterns`, `StreamDisplayItemPatterns`, `SessionStreamStatePatterns`

**Root Cause:** The Dart extractor classifies `extension ... on ...` declarations as types (via `extension_declaration` AST node kind → mapped to `typeKind: "class"`). These are Freezed-generated extensions that add `.map()`/`.when()` methods to sealed class hierarchies. They are **implicitly used** by Dart's exhaustive pattern matching — `extension` methods are automatically available on their target types without explicit import/reference.

**Fix:** Two options:
1. **Best:** Detect `extension ... on TargetType` declarations and create a synthetic `TypeRef` edge from the extension to the target type (or vice versa), connecting them in the graph so the extension inherits the reachability of its target
2. **Alternative:** Skip `extension_declaration` and `extension_type_declaration` entirely from the type extraction — extensions aren't standalone types, they're augmentations
3. **Pragmatic:** Change the `typeKind` for extensions from `"class"` to `"extension"` and exclude extensions from the `unreferenced_types` dead code query

---

### 5. Riverpod Provider Classes and Generated Code Flagged as Dead

**Types:** Various `*Provider` classes in `.g.dart` files, `ActiveSessions` class  
**Functions:** Generated `*FromJson`/`*ToJson` functions in `.g.dart` files

**Root Cause:** Riverpod code generation creates provider classes in `.g.dart` files. These are used implicitly via annotations (e.g., `@riverpod` on a class generates a corresponding provider). The extractor doesn't understand that `class ActiveSessions extends _$ActiveSessions` means the generated `activeSessionsProvider` in the `.g.dart` file is used.

Similarly, `json_serializable` generates `*FromJson`/`*ToJson` functions that are called from `factory X.fromJson()` constructors — the extractor may not trace these calls through generated code.

**Fix:**
1. Detect `.g.dart` and `.freezed.dart` files and either:
   - Mark them as generated (not candidates for dead code)
   - Or create synthetic edges linking them to their source `.dart` file (e.g., `user.g.dart` → `user.dart`)
2. For generated files, create an import edge from the source file to the generated file. In Dart, `part 'file.g.dart';` and `part of` directives create this relationship. The extractor already handles `part` directives in `extract_imports()` but may not correctly resolve the bidirectional relationship.

---

### 6. Test `main()` Functions Flagged as Dead (14 functions)

**Functions:** Every `main()` in test files  
**Root Cause:** Test files' `main()` functions are invoked by the Flutter test runner, not by imports. The dead code filter excludes test **files** but not test **functions**.

**Fix:** The dead code dispatch layer (`dispatch_ast_dead_code` in `ast_dispatch.rs`) already filters `isTest == true` for File entities. Apply the same filter to Function entities — check if the function's containing file is a test file and exclude from `uncalled_functions`.

**Implementation:** The `uncalled_functions` query returns functions with no incoming `Calls` edges. The dispatch layer needs to join back to the containing file to check `isTest`. Options:
- Add a `isTest` property to Function nodes during extraction (derived from the containing file)
- Or filter in the dispatch layer by looking up the function's file slug in the file path index

---

### 7. Test Fixture Class Methods Flagged as Dead (~70 functions)

**Functions:** Methods on fixture classes like `BoardFixtures.connectedInstance()`, `WebSocketFixtures.validAuthConnection()`, `ConnectionFixtures.connectedInstance()`, etc.

**Root Cause:** These methods are called from test files via `ClassName.methodName()` syntax. The Calls edge extractor uses `extract_call_names_from_body()` which extracts bare function names — but static method calls like `BoardFixtures.connectedInstance()` extract `connectedInstance` as the callee name, then `resolve_calls()` tries to find it as a local function or import. Since `connectedInstance` is a method on a class in another file, it's not found as a local function name.

The issue is that **static method calls** and **constructor calls** resolve through the class name, but the current call extraction only captures the method name portion. `BoardFixtures.connectedInstance()` extracts `connectedInstance` but can't resolve it to `test-fixtures-board_fixtures-dart::connectedInstance` because the resolution doesn't look up the class name in the import map.

**Fix:** Improve `extract_call_names_from_body()` or `resolve_calls()` to handle qualified calls:
1. When extracting call names, also capture the qualifier (e.g., `BoardFixtures` in `BoardFixtures.connectedInstance()`)
2. In `resolve_calls()`, when a callee like `connectedInstance` isn't found locally, check if it appears as `Qualifier.callee` in the body text, look up `Qualifier` in the import map or local types, and resolve to `target_file_slug::callee`

---

### 8. Test Fixture Types Flagged as Dead (~12 types)

**Types:** `FakeWebSocketManager`, `FakeRelayConnectionService`, `InMemoryConnectionRepository`, `BoardFixtures`, `DashboardFixtures`, `ConnectionFixtures`, etc.

**Root Cause:** Same as #7 — these classes are used from test files, but the type references happen via constructor calls (`InMemoryConnectionRepository()`) or static method calls (`BoardFixtures.connectedInstance()`). The TypeRef extraction only scans function signatures, not constructor invocations in function bodies.

**Fix:** Two options:
1. When extracting Calls edges from function bodies, also look for constructor invocations (PascalCase identifiers followed by `(`) and create TypeRef edges for them
2. Alternatively, extend TypeRef extraction to include constructor calls in addition to type annotations

---

### 9. Sealed Class Subtypes Flagged as Dead

**Types:** Various Freezed-generated private implementation classes like `_Connection`, `_BoardColumns`, etc.

**Root Cause:** Freezed generates private implementation classes (`_$ClassName`) that implement the public sealed class. These are never directly referenced by user code — they're created by the `freezed` code generator and used via the sealed class's factory constructors.

**Fix:** Same as #5 — detect `.freezed.dart` files and mark as generated/exclude from dead code analysis.

---

### 10. Widget State Classes Flagged as Dead

**Types:** `_ConnectionScreenState`, `_AddConnectionScreenState`, `_BoardScreenState`, `_DashboardScreenState`, `_SessionStreamScreenState`, `_InputBarState`, etc.

**Root Cause:** Flutter `StatefulWidget` creates a `State` subclass that's instantiated by `createState()` — but this is a framework convention. The state class name appears only in its own file's `createState() => _MyWidgetState()` call, which the extractor may not trace.

**Fix:** Detect the Flutter `StatefulWidget` pattern: if a file contains `class _XState extends State<X>`, the state class should be considered used. Either:
- Create a synthetic edge from the widget class to its state class
- Or detect private `State` subclass naming convention `_*State` and exclude from dead code

---

## Summary of Required Fixes

| Priority | Fix | Impact | Files to Modify |
|----------|-----|--------|-----------------|
| **P0** | Exclude test functions from dead code (check containing file `isTest`) | Eliminates 14 false positive functions | `ast_dispatch.rs` |
| **P0** | Exclude `.g.dart` and `.freezed.dart` generated files from dead code | Eliminates ~40 false positive types + ~30 false positive functions | `ast_dispatch.rs` or `ast_dart_extractor.rs` |
| **P0** | Exclude Flutter platform directories from dead code for Flutter projects | Eliminates 30 false positive files | `ast_dispatch.rs` |
| **P1** | Detect `main.dart` entry point | Eliminates 1 false positive file | `ast_dart_extractor.rs` |
| **P1** | Handle `extension on TargetType` — link to target type or skip from dead code | Eliminates 14 false positive types | `ast_dart_extractor.rs` |
| **P1** | Handle qualified/static method calls (`ClassName.method()`) in Calls resolution | Eliminates ~70 false positive functions | `edge_helpers.rs` or `ast_dart_extractor.rs` |
| **P2** | Recognize constructor invocations as TypeRef edges | Eliminates ~12 false positive types | `edge_helpers.rs` |
| **P2** | Detect StatefulWidget `_*State` pattern | Eliminates ~8 false positive types | `ast_dart_extractor.rs` |

---

## Confirmed Genuine Dead Code (for validation)

After fixes, these should STILL be reported as dead:

### Dead Functions (17)
- `WebSocketFixtures.respondWithAuthSuccess()` — `websocket_fixtures.dart:192`
- `WebSocketFixtures.respondWithAuthError()` — `websocket_fixtures.dart:197`
- `FakeWebSocketChannel.simulateError()` — `websocket_fixtures.dart:207`
- `FakeWebSocketChannel.dispose()` — `websocket_fixtures.dart:211`
- `FakeWebSocketSink.lastSentJson` — `websocket_fixtures.dart:135`
- `FakeWebSocketSink.hasAuthMessage()` — `websocket_fixtures.dart:141`
- `WebSocketFixtures.pongMessage()` — `websocket_fixtures.dart:106`
- `BoardFixtures.boardJsonResponse()` — `board_fixtures.dart:217`
- `WorkUnitDetailFixtures.detailJsonResponse()` — `work_unit_detail_fixtures.dart:206`
- `BoardFixtures.emptyBoard()` — `board_fixtures.dart:208`
- `DashboardFixtures.connectionWithStatusMessage()` — `dashboard_fixtures.dart:101`
- `DashboardFixtures.connectionWithRecentActivity()` — `dashboard_fixtures.dart:119`
- `StreamChunkFixtures.sessionStateChangeChunk()` — `stream_chunk_fixtures.dart:204`
- `StreamChunkFixtures.doneChunk()` — `stream_chunk_fixtures.dart:214`
- `QrCodeFixtures.expectedAllFields` — `qr_code_fixtures.dart:28`
- `QrCodeFixtures.expectedWithoutApiKey` — `qr_code_fixtures.dart:36`
- `QrCodeFixtures.expectedPartialFields` — `qr_code_fixtures.dart:44`
- `ImageFixtures.photoImage()` — `image_fixtures.dart:40`

### Dead Types (2)
- `FakeWebSocketChannel` — `websocket_fixtures.dart:158`
- `FakeWebSocketSink` — `websocket_fixtures.dart:118`
