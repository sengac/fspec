// Feature: spec/features/ast-index-class-import-crash.feature
use super::*;

#[test]
fn test_python_underscore_class_import_produces_typeref() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a Python file "src/app/parser.py" defining class "_OptionParser" and function "split_args"
    let parser_source = r#"
class _OptionParser:
    def __init__(self, ctx):
        self.ctx = ctx

def split_args(text):
    return text.split()
"#;
    write_test_file(project_dir, "src/app/parser.py", parser_source);

    // @step And a Python file "src/app/core.py" with "from app.parser import _OptionParser, split_args"
    let core_source = r#"
from app.parser import _OptionParser, split_args

def make_parser(ctx):
    parser = _OptionParser(ctx)
    args = split_args("hello world")
    return parser
"#;
    write_test_file(project_dir, "src/app/core.py", core_source);

    // @step And "src/app/core.py" has function "make_parser" that calls "_OptionParser(ctx)" and "split_args()"
    // @step When I extract entities from both files with known_files containing both paths
    // src/ is the import root of `app.parser`.
    let indexed = index(&project_dir.join("src"));
    let entities = &indexed.entities;

    // @step Then a TypeRef edge should exist from "make_parser" to "_OptionParser"
    // The constructor call resolves exactly to the class (it is neither
    // unresolved nor a Calls edge): every call of the tree resolves except
    // the method call on the untyped `text` receiver.
    assert!(
        find_nodes_by_name(entities, "Type", "_OptionParser").len() == 1,
        "one Type node _OptionParser"
    );
    assert_eq!(
        unresolved(&indexed, "calls", "not_found"),
        0,
        "{:?}",
        indexed.unresolved
    );
    assert_eq!(
        unresolved(&indexed, "calls", "ambiguous"),
        0,
        "{:?}",
        indexed.unresolved
    );
    assert_eq!(
        unresolved(&indexed, "calls", "external"),
        0,
        "{:?}",
        indexed.unresolved
    );

    // @step And a Calls edge should exist from "make_parser" to "split_args"
    let calls = named_edges(entities, "Calls");
    assert!(
        calls.contains(&(
            "app/core.py::make_parser".into(),
            "app/parser.py::split_args".into()
        )),
        "Should have Calls edge from make_parser to split_args. All Calls: {calls:?}"
    );

    // @step And no Calls edge should target "_OptionParser"
    let bad_calls = edges_touching(entities, "Calls", "_OptionParser");
    assert!(
        bad_calls.is_empty(),
        "Should NOT have Calls edge targeting _OptionParser (it's a Type, not Function). \
         Bad Calls: {bad_calls:?}"
    );
}

#[test]
fn test_java_class_with_comment_keyword_extracts_correct_name() {
    // @step Given a Java file "com/myapp/MyException.java" with content:
    let java_source = r#"package com.myapp;
// This is a class for custom exceptions
@SuppressWarnings("MemberName") // class name is part of the public API
public final class MyException extends RuntimeException {
    public MyException(String msg) { super(msg); }
}
"#;

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    write_test_file(temp_dir.path(), "com/myapp/MyException.java", java_source);

    // @step When I extract entities from the Java file
    let indexed = index(temp_dir.path());
    assert!(indexed.failed.is_empty(), "{:?}", indexed.failed);
    let entities = indexed.entities;

    // @step Then a Type node should exist with name "MyException"
    let exception_types = find_nodes_by_name(&entities, "Type", "MyException");
    assert!(
        !exception_types.is_empty(),
        "Should have Type node for MyException. All Types: {:?}",
        entities
            .iter()
            .filter(|e| matches!(e, GraphEntity::Node { node_type, .. } if node_type == "Type"))
            .collect::<Vec<_>>()
    );

    // @step And no Type node should exist with name "name"
    let bad_name_types = find_nodes_by_name(&entities, "Type", "name");
    // Filter to exact match — "name" not just substring
    let exact_bad = bad_name_types
        .iter()
        .filter(|e| {
            if let GraphEntity::Node { properties, .. } = e {
                properties.get("name").and_then(|v| v.as_str()) == Some("name")
            } else {
                false
            }
        })
        .count();
    assert_eq!(
        exact_bad,
        0,
        "Should NOT have Type node with name 'name' (extracted from comment). Types: {:?}",
        entities
            .iter()
            .filter(|e| matches!(e, GraphEntity::Node { node_type, .. } if node_type == "Type"))
            .collect::<Vec<_>>()
    );

    // @step And no Type node should exist with name "MemberName"
    let member_types = find_nodes_by_name(&entities, "Type", "MemberName");
    assert!(
        member_types.is_empty(),
        "Should NOT have Type node with name 'MemberName' (from annotation string)"
    );
}

#[test]
fn test_java_imported_class_constructor_produces_typeref() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a Java file "com/myapp/MyException.java" defining class "MyException"
    let exception_source = r#"package com.myapp;
public class MyException extends RuntimeException {
    public MyException(String msg) { super(msg); }
}
"#;
    write_test_file(project_dir, "com/myapp/MyException.java", exception_source);

    // @step And a Java file "com/myapp/Service.java" with "import com.myapp.MyException;"
    let service_source = r#"package com.myapp;
import com.myapp.MyException;

public class Service {
    public void doWork() {
        throw new MyException("failed");
    }
}
"#;
    write_test_file(project_dir, "com/myapp/Service.java", service_source);

    // @step And "com/myapp/Service.java" has method "doWork" that calls "new MyException(msg)"
    // @step When I extract entities from both files with known_files containing both paths
    let indexed = index(project_dir);
    let entities = &indexed.entities;
    assert!(
        named_edges(entities, "Imports").contains(&(
            "com/myapp/Service.java".into(),
            "com/myapp/MyException.java".into()
        )),
        "the import resolves: {:?}",
        named_edges(entities, "Imports")
    );

    // @step Then a TypeRef edge should exist from "doWork" to "MyException"
    // `new MyException(..)` constructs the imported class: it names the
    // class exactly and is neither a Calls edge nor an unresolved call.
    assert_eq!(
        find_nodes_by_name(entities, "Type", "MyException").len(),
        1,
        "one Type node MyException"
    );
    assert_eq!(
        unresolved(&indexed, "calls", "not_found"),
        0,
        "{:?}",
        indexed.unresolved
    );
    assert_eq!(
        unresolved(&indexed, "calls", "ambiguous"),
        0,
        "{:?}",
        indexed.unresolved
    );

    // @step And no Calls edge should target "MyException" as a Function
    let bad_calls = calls_to_types(entities);
    assert!(
        bad_calls.is_empty(),
        "Should NOT have Calls edge targeting MyException. Bad Calls: {bad_calls:?}"
    );

    // @step And indexing should complete without errors
    assert!(indexed.failed.is_empty(), "{:?}", indexed.failed);
    assert_eq!(indexed.files_parsed, 2);
}

#[test]
fn test_python_mixed_imports_function_calls_class_typeref() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a Python file "src/app/utils.py" defining function "parse_args" and class "Config"
    let utils_source = r#"
def parse_args(argv):
    return argv

class Config:
    def __init__(self):
        self.debug = False
"#;
    write_test_file(project_dir, "src/app/utils.py", utils_source);

    // @step And a Python file "src/app/main.py" with "from app.utils import parse_args, Config"
    let main_source = r#"
from app.utils import parse_args, Config

def run():
    args = parse_args(["--debug"])
    cfg = Config()
    return cfg
"#;
    write_test_file(project_dir, "src/app/main.py", main_source);

    // @step And "src/app/main.py" has function "run" that calls "parse_args()" and "Config()"
    // @step When I extract entities from both files with known_files containing both paths
    // src/ is the import root of `app.utils`.
    let indexed = index(&project_dir.join("src"));
    let entities = &indexed.entities;

    // @step Then a Calls edge should exist from "run" to "parse_args"
    let calls = named_edges(entities, "Calls");
    assert!(
        calls.contains(&("app/main.py::run".into(), "app/utils.py::parse_args".into())),
        "Should have Calls edge from run to parse_args. All Calls: {calls:?}"
    );

    // @step And a TypeRef edge should exist from "run" to "Config"
    // `Config()` constructs the imported class: it resolves exactly (no
    // unresolved call) and is not a Calls edge.
    assert_eq!(
        unresolved(&indexed, "calls", "not_found"),
        0,
        "{:?}",
        indexed.unresolved
    );
    assert_eq!(
        unresolved(&indexed, "calls", "ambiguous"),
        0,
        "{:?}",
        indexed.unresolved
    );
    assert!(
        edges_touching(entities, "Calls", "Config").is_empty(),
        "No Calls edge may target the class Config: {calls:?}"
    );
}
