//! Java (tree-sitter-java 0.23).
//!
//! Calls are `method_invocation` nodes read through their `name` and
//! `object` fields (a call without `object` is a plain call). Not declared,
//! because the engine cannot express it: the implicit `public` of interface
//! members, `final` constants.

use ast_grep_language::SupportLang;

use crate::ast_pipeline::facts::spec::*;

/// Type names in signatures. `scoped_type_identifier` has no fields, so its
/// path is empty and it is dropped rather than read as its last segment.
const TYPE_NAMES: &[&str] = &["type_identifier", "scoped_type_identifier"];

/// A base written as a simple name.
const BASE_NAMES: &[&str] = &["type_identifier"];

const fn function(kind: &'static str) -> DefinitionSpec {
    DefinitionSpec {
        kind,
        entity: Entity::Function,
        name: NameSpec::Field("name"),
        requires_field: None,
        parameters: Some("parameters"),
        // `receiver_parameter` is the explicit `this` of a method.
        non_parameters: &["receiver_parameter", "line_comment", "block_comment"],
        members: &[],
        container: None,
    }
}

const fn type_def(
    kind: &'static str,
    type_kind: TypeKind,
    members: &'static [MemberSpec],
) -> DefinitionSpec {
    DefinitionSpec {
        kind,
        entity: Entity::Type(type_kind),
        name: NameSpec::Field("name"),
        requires_field: None,
        parameters: None,
        non_parameters: &[],
        members,
        container: Some(ContainerKind::Type),
    }
}

/// `class_declaration`-like kinds with bases at `fields`: once as written
/// (`Base`), once through a generic type (`Base<T>`, whose first
/// `type_identifier` child is the base).
const fn bases(
    kind: &'static str,
    relation: BaseRelation,
    fields: &'static [&'static str],
) -> BaseSpec {
    BaseSpec {
        kind,
        relation,
        fields,
        each_child: true,
        through_kinds: BASE_NAMES,
    }
}

pub static JAVA: LanguageSpec = LanguageSpec {
    name: "java",
    extensions: &["java"],
    grammar: Grammar::Builtin(SupportLang::Java),
    definitions: &[
        function("method_declaration"),
        function("constructor_declaration"),
        DefinitionSpec {
            parameters: None,
            ..function("compact_constructor_declaration")
        },
        // An annotation interface element is a method declaration (JLS 9.6.1).
        DefinitionSpec {
            parameters: None,
            ..function("annotation_type_element_declaration")
        },
        type_def("class_declaration", TypeKind::Class, &[]),
        type_def("interface_declaration", TypeKind::Interface, &[]),
        type_def("annotation_type_declaration", TypeKind::Interface, &[]),
        type_def(
            "enum_declaration",
            TypeKind::Enum,
            &[MemberSpec {
                kind: "enum_constant",
                name: NameSpec::Field("name"),
            }],
        ),
        type_def("record_declaration", TypeKind::Class, &[]),
    ],
    containers: &[],
    module_declarations: &[],
    calls: &[CallSpec {
        kind: "method_invocation",
        callee: Some("name"),
        receiver: Some("object"),
    }],
    paths: PathSpec {
        separator: ".",
        names: &["identifier", "type_identifier"],
        scoped: &[
            ScopedSpec {
                kind: "scoped_identifier",
                path: "scope",
                name: "name",
            },
            // The name of a field declarator: `dimensions` is absent unless
            // the C-style `int x[]` form is used, so the path is the name.
            ScopedSpec {
                kind: "variable_declarator",
                path: "dimensions",
                name: "name",
            },
        ],
        members: &[
            ScopedSpec {
                kind: "method_invocation",
                path: "object",
                name: "name",
            },
            ScopedSpec {
                kind: "field_access",
                path: "object",
                name: "field",
            },
        ],
        wrappers: &[],
    },
    self_receivers: &["this"],
    imports: &["import_declaration"],
    declarations: &["package_declaration"],
    bases: &[
        bases("class_declaration", BaseRelation::Extends, &["superclass"]),
        bases(
            "class_declaration",
            BaseRelation::Extends,
            &["superclass", "generic_type"],
        ),
        bases(
            "class_declaration",
            BaseRelation::Implements,
            &["interfaces", "type_list"],
        ),
        bases(
            "class_declaration",
            BaseRelation::Implements,
            &["interfaces", "type_list", "generic_type"],
        ),
        bases(
            "interface_declaration",
            BaseRelation::Extends,
            &["extends_interfaces", "type_list"],
        ),
        bases(
            "interface_declaration",
            BaseRelation::Extends,
            &["extends_interfaces", "type_list", "generic_type"],
        ),
        bases(
            "enum_declaration",
            BaseRelation::Implements,
            &["interfaces", "type_list"],
        ),
        bases(
            "enum_declaration",
            BaseRelation::Implements,
            &["interfaces", "type_list", "generic_type"],
        ),
        bases(
            "record_declaration",
            BaseRelation::Implements,
            &["interfaces", "type_list"],
        ),
        bases(
            "record_declaration",
            BaseRelation::Implements,
            &["interfaces", "type_list", "generic_type"],
        ),
    ],
    type_ref_fields: &[
        TypeRefField {
            field: "parameters",
            ref_kind: RefKind::Param,
            type_kinds: TYPE_NAMES,
            item_field: None,
        },
        TypeRefField {
            field: "type",
            ref_kind: RefKind::ReturnType,
            type_kinds: TYPE_NAMES,
            item_field: None,
        },
    ],
    decisions: &[
        DecisionSpec {
            kind: "if_statement",
            operators: &[],
        },
        DecisionSpec {
            kind: "for_statement",
            operators: &[],
        },
        DecisionSpec {
            kind: "enhanced_for_statement",
            operators: &[],
        },
        DecisionSpec {
            kind: "while_statement",
            operators: &[],
        },
        DecisionSpec {
            kind: "do_statement",
            operators: &[],
        },
        // `case` and `default` labels, in statement groups and rules.
        DecisionSpec {
            kind: "switch_label",
            operators: &[],
        },
        DecisionSpec {
            kind: "catch_clause",
            operators: &[],
        },
        DecisionSpec {
            kind: "ternary_expression",
            operators: &[],
        },
        DecisionSpec {
            kind: "binary_expression",
            operators: &["&&", "||"],
        },
    ],
    docs: &[DocSpec {
        kind: "block_comment",
        placement: Placement::PrecedingSiblings,
        requires_child: None,
        prefixes: &["/**"],
    }],
    decorators: &[
        DecoratorSpec {
            kind: "marker_annotation",
            placement: Placement::ChildChildren("modifiers"),
        },
        DecoratorSpec {
            kind: "annotation",
            placement: Placement::ChildChildren("modifiers"),
        },
        DecoratorSpec {
            kind: "marker_annotation",
            placement: Placement::Children,
        },
        DecoratorSpec {
            kind: "annotation",
            placement: Placement::Children,
        },
    ],
    test_markers: &[
        TestMarker {
            path: "Test",
            arguments: None,
            on_container: false,
        },
        TestMarker {
            path: "org.junit.Test",
            arguments: None,
            on_container: false,
        },
        TestMarker {
            path: "org.junit.jupiter.api.Test",
            arguments: None,
            on_container: false,
        },
        TestMarker {
            path: "org.testng.annotations.Test",
            arguments: None,
            on_container: false,
        },
        TestMarker {
            path: "ParameterizedTest",
            arguments: None,
            on_container: false,
        },
        TestMarker {
            path: "RepeatedTest",
            arguments: None,
            on_container: false,
        },
        TestMarker {
            path: "TestFactory",
            arguments: None,
            on_container: false,
        },
        TestMarker {
            path: "TestTemplate",
            arguments: None,
            on_container: false,
        },
    ],
    // Java has no test path convention (src/test/java is Maven's).
    test_paths: &[],
    variables: &[
        VariableSpec {
            kind: "field_declaration",
            name: NameSpec::PathField("declarator"),
            constant: false,
            value: None,
        },
        // Interface fields are implicitly static and final (JLS 9.3).
        VariableSpec {
            kind: "constant_declaration",
            name: NameSpec::PathField("declarator"),
            constant: true,
            value: None,
        },
    ],
    macros: None,
    visibility: Visibility::Modifier {
        modifiers: "modifiers",
        tokens: &["public"],
    },
    async_markers: &[],
    class_members_in_scope: true,
    self_receiver_texts: &[],
    parameter_names: None,
    body_siblings: &[],
    sibling_calls: None,
    method_owners: &[],
};
