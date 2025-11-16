# JSON AST Support Fix

## Problem

The AST research tool (`fspec research --tool=ast`) failed when analyzing JSON files with error:
```
Cannot detect language from file extension: src/schemas/generic-foundation.schema.json
```

Even though `@sengac/tree-sitter-json` was installed and loaded in `language-loader.ts`, JSON files were not supported.

## Root Causes Identified

### 1. Missing Language Detection (ast.ts)
**File:** `src/research-tools/ast.ts`
**Function:** `detectLanguage()`
**Issue:** No check for `.json` extension

**Before:**
```typescript
if (filePath.endsWith('.sh') || filePath.endsWith('.bash')) {
  return 'bash';
}
throw new Error(`Cannot detect language from file extension: ${filePath}`);
```

**After:**
```typescript
if (filePath.endsWith('.sh') || filePath.endsWith('.bash')) {
  return 'bash';
}
if (filePath.endsWith('.json')) {
  return 'json';
}
throw new Error(`Cannot detect language from file extension: ${filePath}`);
```

### 2. Missing JSON Query Files
**Directory:** `src/utils/ast-queries/json/`
**Issue:** No tree-sitter query files (.scm) for JSON operations

**Created:**
- `keys.scm` - Query to find all JSON object keys
- `properties.scm` - Query to find all key-value pairs

**keys.scm:**
```scheme
; Query to find all keys in a JSON object
(pair
  key: (string) @key)
```

**properties.scm:**
```scheme
; Query to find all key-value pairs in JSON objects
(pair
  key: (string) @property.key
  value: (_) @property.value)
```

### 3. Missing Operations in Query Executor
**File:** `src/utils/query-executor.ts`
**Issue:** New JSON operations not registered or handled

**Changes:**
1. Added operations to query file map:
   ```typescript
   'list-keys': 'keys.scm',
   'list-properties': 'properties.scm',
   ```

2. Added JSON operations to execute() method:
   ```typescript
   else if (operation === 'list-keys' || operation === 'list-properties') {
     // JSON operations use tree-sitter Query API
     return this.executeQuery(tree, language);
   }
   ```

3. Fixed executeQuery() to load queries for operations:
   ```typescript
   // If operation is specified (list-keys, list-properties, etc.), load the query
   if (!queryString && this.options.operation) {
     queryString = await this.loadQuery();
   }
   ```

### 4. Missing ESM __dirname
**File:** `src/utils/query-executor.ts`
**Issue:** `__dirname` not defined in ESM modules

**Fix:**
```typescript
import { fileURLToPath } from 'url';
import { dirname } from 'path';

// ESM __dirname equivalent
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
```

### 5. Missing Build Configuration
**File:** `vite.config.ts`
**Issue:** `.scm` query files not copied to `dist/` during build

**Fix:**
```typescript
// Bundle ast-queries directory (.scm query files)
cpSync(
  resolve(__dirname, 'src', 'utils', 'ast-queries'),
  resolve(__dirname, 'dist', 'ast-queries'),
  { recursive: true }
);
```

### 6. Missing Help Documentation
**File:** `src/research-tools/ast.ts`
**Function:** `getHelpConfig()`
**Issue:** Help text didn't mention JSON support or new operations

**Changes:**
1. Updated language count: "15 languages" → "16 languages"
2. Added JSON to language lists
3. Added operation examples:
   ```
   --operation=list-keys --file=src/schemas/schema.json
   --operation=list-properties --file=config.json
   ```
4. Updated operation description to mention JSON operations

## Testing

### Before Fix
```bash
$ fspec research --tool=ast --file src/schemas/generic-foundation.schema.json --operation list-keys
Error: Cannot detect language from file extension: src/schemas/generic-foundation.schema.json
```

### After Fix
```bash
$ fspec research --tool=ast --file src/schemas/generic-foundation.schema.json --operation list-keys
{
  "matches": [
    {
      "type": "string",
      "line": 2,
      "column": 2,
      "text": "\"$schema\""
    },
    {
      "type": "string",
      "line": 3,
      "column": 2,
      "text": "\"$id\""
    },
    ...
  ]
}
```

### Simple Test Case
```bash
$ cat > /tmp/test.json <<EOF
{
  "name": "test",
  "version": "1.0.0"
}
EOF

$ fspec research --tool=ast --file /tmp/test.json --operation list-keys
{
  "matches": [
    {
      "type": "string",
      "line": 2,
      "column": 2,
      "text": "\"name\""
    },
    {
      "type": "string",
      "line": 3,
      "column": 2,
      "text": "\"version\""
    }
  ]
}
```

## Files Changed

1. `src/research-tools/ast.ts` - Added JSON language detection + help docs
2. `src/utils/ast-queries/json/keys.scm` - NEW: JSON keys query
3. `src/utils/ast-queries/json/properties.scm` - NEW: JSON properties query
4. `src/utils/query-executor.ts` - Added JSON operations + ESM __dirname + query loading
5. `vite.config.ts` - Added ast-queries bundling

## Impact

- **JSON files now fully supported** for AST research during Example Mapping
- **16 languages supported** (was 15): JavaScript, TypeScript, Python, Go, Rust, Kotlin, Dart, Swift, C#, C, C++, Java, PHP, Ruby, Bash, **JSON**
- **New operations available**: `list-keys`, `list-properties`
- **Schema analysis enabled** for work units like EXMAP-012

## Related Work Unit

- **EXMAP-012**: Complete Big Picture Event Storm Schema Integration
  - Requires analyzing `src/schemas/generic-foundation.schema.json`
  - JSON AST support was blocker for AST research requirement
  - Can now proceed with ACDD workflow

## Future Enhancements

Potential JSON-specific operations:
- `find-required-fields` - Find all required properties in schema
- `find-nested-objects` - Find deeply nested object structures
- `find-array-items` - Find array item schemas
- `validate-schema-refs` - Find $ref pointers

Custom queries can already be used via `--query-file` flag.
