//! C (tree-sitter-c 0.24). `.h` files are declared by the C++ table
//! (product owner, 2026-11-07).
//!
//! A function's name is reached through its declarator chain
//! (`function_definition` -> `pointer_declarator` -> `function_declarator`
//! -> `identifier`). Prototypes (`declaration` with a `function_declarator`)
//! are not definitions: the table cannot tell them from variable
//! declarations by name alone.

use ast_grep_language::SupportLang;

use crate::ast_pipeline::facts::spec::*;

const COMMENTS: &[&str] = &["comment"];

const TYPE_NAMES: &[&str] = &["type_identifier"];

const fn type_def(
    kind: &'static str,
    type_kind: TypeKind,
    members: &'static [MemberSpec],
) -> DefinitionSpec {
    DefinitionSpec {
        kind,
        entity: Entity::Type(type_kind),
        name: NameSpec::Field("name"),
        // `struct Point p;` names the type; only a body defines it.
        requires_field: Some("body"),
        parameters: None,
        non_parameters: &[],
        members,
        container: Some(ContainerKind::Type),
    }
}

pub static C: LanguageSpec = LanguageSpec {
    name: "c",
    extensions: &["c"],
    grammar: Grammar::Builtin(SupportLang::C),
    definitions: &[
        DefinitionSpec {
            kind: "function_definition",
            entity: Entity::Function,
            name: NameSpec::Descend {
                field: "declarator",
                until: &["identifier"],
                through: &[],
            },
            requires_field: None,
            // The parameter list is a field of the function_declarator, not
            // of the definition.
            parameters: None,
            non_parameters: COMMENTS,
            members: &[],
            container: None,
        },
        type_def("struct_specifier", TypeKind::Struct, &[]),
        type_def("union_specifier", TypeKind::Struct, &[]),
        type_def(
            "enum_specifier",
            TypeKind::Enum,
            &[MemberSpec {
                kind: "enumerator",
                name: NameSpec::Field("name"),
            }],
        ),
        DefinitionSpec {
            kind: "type_definition",
            entity: Entity::Type(TypeKind::TypeAlias),
            name: NameSpec::Descend {
                field: "declarator",
                until: &["type_identifier"],
                through: &[],
            },
            requires_field: None,
            parameters: None,
            non_parameters: &[],
            members: &[],
            container: None,
        },
    ],
    containers: &[],
    module_declarations: &[],
    calls: &[CallSpec {
        kind: "call_expression",
        callee: Some("function"),
        receiver: None,
    }],
    paths: PathSpec {
        separator: ".",
        names: &["identifier", "field_identifier", "type_identifier"],
        scoped: &[],
        members: &[ScopedSpec {
            kind: "field_expression",
            path: "argument",
            name: "field",
        }],
        // Variable declarators: `int *p = 0;`, `int a[3];`.
        wrappers: &[
            ("init_declarator", "declarator"),
            ("pointer_declarator", "declarator"),
            ("array_declarator", "declarator"),
        ],
    },
    self_receivers: &[],
    imports: &["preproc_include"],
    declarations: &[],
    bases: &[],
    type_ref_fields: &[
        TypeRefField {
            field: "type",
            ref_kind: RefKind::ReturnType,
            type_kinds: TYPE_NAMES,
            item_field: None,
        },
        TypeRefField {
            field: "declarator",
            ref_kind: RefKind::Param,
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
            kind: "while_statement",
            operators: &[],
        },
        DecisionSpec {
            kind: "do_statement",
            operators: &[],
        },
        DecisionSpec {
            kind: "case_statement",
            operators: &[],
        },
        DecisionSpec {
            kind: "conditional_expression",
            operators: &[],
        },
        DecisionSpec {
            kind: "binary_expression",
            operators: &["&&", "||"],
        },
    ],
    docs: &[DocSpec {
        kind: "comment",
        placement: Placement::PrecedingSiblings,
        requires_child: None,
        prefixes: &[],
    }],
    decorators: &[
        DecoratorSpec {
            kind: "attribute_specifier",
            placement: Placement::Children,
        },
        DecoratorSpec {
            kind: "attribute_declaration",
            placement: Placement::Children,
        },
    ],
    test_markers: &[],
    test_paths: &[
        TestPath::Directory("test"),
        TestPath::Directory("tests"),
        TestPath::StemSuffix("_test"),
    ],
    variables: &[
        // A declaration whose declarator is a function_declarator (a
        // prototype) has no path, so it is not a variable.
        VariableSpec {
            kind: "declaration",
            name: NameSpec::PathField("declarator"),
            constant: false,
            value: None,
        },
        VariableSpec {
            kind: "field_declaration",
            name: NameSpec::PathField("declarator"),
            constant: false,
            value: None,
        },
    ],
    macros: None,
    // `static` makes a function file-local; the table has no way to say
    // "public unless static".
    visibility: Visibility::Unless {
        modifiers: "storage_class_specifier",
        tokens: &["static"],
    },
    async_markers: &[],
    class_members_in_scope: false,
    self_receiver_texts: &[],
    parameter_names: None,
    body_siblings: &[],
    sibling_calls: None,
    method_owners: &[],
};
