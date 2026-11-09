//! Kotlin (tree-sitter-kotlin-sg 0.4).
//!
//! The grammar has no fields except on `if_expression`. Calls are
//! `call_expression` nodes whose callee is their first named child (a plain
//! name; a `navigation_expression` callee has no fields, so it is an
//! expression callee). Type references are the type names among the
//! `function_value_parameters` and the return `user_type`/`nullable_type`
//! children. Not declared: parameter counts, property names. Interfaces, enum classes and classes are all
//! `class_declaration` (the keyword is an anonymous token), so all three
//! are classes. Secondary constructors have no name node.

use ast_grep_language::SupportLang;

use crate::ast_pipeline::facts::spec::*;

const BASE_NAMES: &[&str] = &["type_identifier"];

const fn type_def(
    kind: &'static str,
    type_kind: TypeKind,
    members: &'static [MemberSpec],
) -> DefinitionSpec {
    DefinitionSpec {
        kind,
        entity: Entity::Type(type_kind),
        name: NameSpec::ChildKind(&["type_identifier"]),
        requires_field: None,
        parameters: None,
        non_parameters: &[],
        members,
        container: Some(ContainerKind::Type),
    }
}

/// A supertype of `kind` at `fields`.
const fn base(
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

/// A constructor call names the superclass (only a class is constructed).
const SUPERCLASS: &[&str] = &[
    "delegation_specifier",
    "constructor_invocation",
    "user_type",
];
/// A supertype without a constructor call names an interface.
const INTERFACE: &[&str] = &["delegation_specifier", "user_type"];
/// Only an interface is delegated (`I by x`).
const DELEGATED: &[&str] = &["delegation_specifier", "explicit_delegation", "user_type"];

pub static KOTLIN: LanguageSpec = LanguageSpec {
    name: "kotlin",
    extensions: &["kt", "kts"],
    grammar: Grammar::Builtin(SupportLang::Kotlin),
    definitions: &[
        DefinitionSpec {
            kind: "function_declaration",
            entity: Entity::Function,
            name: NameSpec::ChildKind(&["simple_identifier"]),
            requires_field: None,
            parameters: None,
            non_parameters: &[],
            members: &[],
            container: None,
        },
        type_def(
            "class_declaration",
            TypeKind::Class,
            &[MemberSpec {
                kind: "enum_entry",
                name: NameSpec::ChildKind(&["simple_identifier"]),
            }],
        ),
        type_def("object_declaration", TypeKind::Class, &[]),
        // A companion without a name has no name node: it is not a
        // definition, and its members are members of the enclosing class.
        type_def("companion_object", TypeKind::Class, &[]),
        DefinitionSpec {
            container: None,
            ..type_def("type_alias", TypeKind::TypeAlias, &[])
        },
    ],
    containers: &[],
    module_declarations: &[],
    calls: &[CallSpec {
        kind: "call_expression",
        callee: None,
        receiver: None,
    }],
    paths: PathSpec {
        separator: ".",
        names: &["simple_identifier", "type_identifier"],
        scoped: &[],
        members: &[],
        wrappers: &[],
    },
    self_receivers: &["this_expression"],
    imports: &["import_header"],
    declarations: &["package_header"],
    bases: &[
        base("class_declaration", BaseRelation::Extends, SUPERCLASS),
        base("class_declaration", BaseRelation::Implements, INTERFACE),
        base("class_declaration", BaseRelation::Implements, DELEGATED),
        base("object_declaration", BaseRelation::Extends, SUPERCLASS),
        base("object_declaration", BaseRelation::Implements, INTERFACE),
        base("object_declaration", BaseRelation::Implements, DELEGATED),
        base("companion_object", BaseRelation::Extends, SUPERCLASS),
        base("companion_object", BaseRelation::Implements, INTERFACE),
        base("companion_object", BaseRelation::Implements, DELEGATED),
    ],
    type_ref_fields: &[
        TypeRefField {
            field: "function_value_parameters",
            ref_kind: RefKind::Param,
            type_kinds: &["type_identifier"],
            item_field: None,
        },
        TypeRefField {
            field: "user_type",
            ref_kind: RefKind::ReturnType,
            type_kinds: &["type_identifier"],
            item_field: None,
        },
        TypeRefField {
            field: "nullable_type",
            ref_kind: RefKind::ReturnType,
            type_kinds: &["type_identifier"],
            item_field: None,
        },
    ],
    decisions: &[
        DecisionSpec {
            kind: "if_expression",
            operators: &[],
        },
        DecisionSpec {
            kind: "for_statement",
            operators: &[],
        },
        DecisionSpec {
            kind: "while_statement",
            operators: &[],
        },
        DecisionSpec {
            kind: "do_while_statement",
            operators: &[],
        },
        DecisionSpec {
            kind: "when_entry",
            operators: &[],
        },
        DecisionSpec {
            kind: "catch_block",
            operators: &[],
        },
        DecisionSpec {
            kind: "conjunction_expression",
            operators: &[],
        },
        DecisionSpec {
            kind: "disjunction_expression",
            operators: &[],
        },
    ],
    docs: &[DocSpec {
        kind: "multiline_comment",
        placement: Placement::PrecedingSiblings,
        requires_child: None,
        prefixes: &["/**"],
    }],
    decorators: &[DecoratorSpec {
        kind: "annotation",
        placement: Placement::ChildChildren("modifiers"),
    }],
    test_markers: &[
        TestMarker {
            path: "Test",
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
    ],
    test_paths: &[],
    variables: &[],
    macros: None,
    // Kotlin declarations are public unless marked private, protected or
    // internal; the spec has no "public unless" rule, so every definition
    // is public.
    visibility: Visibility::Always,
    async_markers: &[Marker {
        token: "suspend",
        inside: Some("modifiers"),
    }],
    class_members_in_scope: true,
    self_receiver_texts: &[],
    parameter_names: None,
    body_siblings: &[],
    sibling_calls: None,
    method_owners: &[],
};
