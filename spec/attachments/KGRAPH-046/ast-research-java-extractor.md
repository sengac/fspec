# AST Research: Java Extractor Current State

## Current extract_java function signature
`pub fn extract_java(source: &str, rel_path: &str, _known_files: &HashSet<String>)`

## Current functions
- `extract_methods` - uses KindMatcher for method_declaration, constructor_declaration  
- `extract_types` - uses KindMatcher for class/interface/enum/record

## Missing (needed for edges)
- `extract_imports` - parse `import com.package.Class;` statements
- `extract_calls` - scan method bodies for call expressions
- `extract_type_refs` - parse type annotations in method signatures

## Java-specific import patterns
- `import com.myapp.service.UserService;` → resolve to com/myapp/service/UserService.java
- `import java.util.List;` → SKIP (standard library)
- `import static` → SKIP
