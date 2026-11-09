//! Rust (tree-sitter-rust 0.24).

use ast_grep_language::SupportLang;

use crate::ast_pipeline::facts::spec::*;

const COMMENTS: &[&str] = &["line_comment", "block_comment", "attribute_item"];

const TYPE_NAMES: &[&str] = &["type_identifier", "scoped_type_identifier"];

const FUNCTION: DefinitionSpec = DefinitionSpec {
    kind: "function_item",
    entity: Entity::Function,
    name: NameSpec::Field("name"),
    requires_field: None,
    parameters: Some("parameters"),
    non_parameters: COMMENTS,
    members: &[],
    container: None,
};

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
        container: None,
    }
}

pub static RUST: LanguageSpec = LanguageSpec {
    name: "rust",
    extensions: &["rs"],
    grammar: Grammar::Builtin(SupportLang::Rust),
    definitions: &[
        FUNCTION,
        DefinitionSpec {
            kind: "function_signature_item",
            ..FUNCTION
        },
        type_def("struct_item", TypeKind::Struct, &[]),
        type_def("union_item", TypeKind::Struct, &[]),
        type_def(
            "enum_item",
            TypeKind::Enum,
            &[MemberSpec {
                kind: "enum_variant",
                name: NameSpec::Field("name"),
            }],
        ),
        DefinitionSpec {
            container: Some(ContainerKind::Type),
            ..type_def("trait_item", TypeKind::Trait, &[])
        },
        type_def("type_item", TypeKind::TypeAlias, &[]),
    ],
    containers: &[
        ContainerSpec {
            kind: "impl_item",
            container: ContainerKind::Impl {
                self_type: "type",
                trait_field: Some("trait"),
            },
            name: None,
            requires_field: None,
        },
        ContainerSpec {
            kind: "mod_item",
            container: ContainerKind::Module,
            name: Some(NameSpec::Field("name")),
            requires_field: Some("body"),
        },
    ],
    module_declarations: &[ModuleDeclarationSpec {
        kind: "mod_item",
        name: NameSpec::Field("name"),
        without_field: "body",
        path_attribute: Some(("path", "string_content")),
    }],
    calls: &[CallSpec {
        kind: "call_expression",
        callee: Some("function"),
        receiver: None,
    }],
    paths: PathSpec {
        separator: "::",
        names: &[
            "identifier",
            "type_identifier",
            "crate",
            "super",
            "self",
            "primitive_type",
        ],
        scoped: &[
            ScopedSpec {
                kind: "scoped_identifier",
                path: "path",
                name: "name",
            },
            ScopedSpec {
                kind: "scoped_type_identifier",
                path: "path",
                name: "name",
            },
        ],
        members: &[ScopedSpec {
            kind: "field_expression",
            path: "value",
            name: "field",
        }],
        wrappers: &[("generic_function", "function"), ("generic_type", "type")],
    },
    self_receivers: &["self"],
    imports: &["use_declaration"],
    declarations: &[],
    bases: &[
        // impl Trait for Type: the container's trait is read from the
        // container itself (ContainerKind::Impl); supertraits are bases.
        BaseSpec {
            kind: "trait_item",
            relation: BaseRelation::Extends,
            fields: &["bounds"],
            each_child: true,
            through_kinds: TYPE_NAMES,
        },
    ],
    type_ref_fields: &[
        TypeRefField {
            field: "parameters",
            ref_kind: RefKind::Param,
            type_kinds: TYPE_NAMES,
            item_field: None,
        },
        TypeRefField {
            field: "return_type",
            ref_kind: RefKind::ReturnType,
            type_kinds: TYPE_NAMES,
            item_field: None,
        },
    ],
    decisions: &[
        DecisionSpec {
            kind: "if_expression",
            operators: &[],
        },
        DecisionSpec {
            kind: "match_arm",
            operators: &[],
        },
        DecisionSpec {
            kind: "while_expression",
            operators: &[],
        },
        DecisionSpec {
            kind: "for_expression",
            operators: &[],
        },
        DecisionSpec {
            kind: "binary_expression",
            operators: &["&&", "||"],
        },
    ],
    docs: &[DocSpec {
        kind: "line_comment",
        placement: Placement::PrecedingSiblings,
        requires_child: Some("doc_comment"),
        prefixes: &["///"],
    }],
    decorators: &[DecoratorSpec {
        kind: "attribute_item",
        placement: Placement::PrecedingSiblings,
    }],
    test_markers: &[
        TestMarker {
            path: "test",
            arguments: None,
            on_container: false,
        },
        TestMarker {
            path: "tokio::test",
            arguments: None,
            on_container: false,
        },
        TestMarker {
            path: "cfg",
            arguments: Some("(test)"),
            on_container: true,
        },
    ],
    test_paths: &[TestPath::Directory("tests")],
    variables: &[
        VariableSpec {
            kind: "const_item",
            name: NameSpec::Field("name"),
            constant: true,
            value: Some("value"),
        },
        VariableSpec {
            kind: "static_item",
            name: NameSpec::Field("name"),
            constant: false,
            value: Some("value"),
        },
    ],
    macros: Some(MacroSpec {
        invocation: "macro_invocation",
        token_tree: "token_tree",
        names: &["identifier"],
    }),
    visibility: Visibility::ChildKind(&["visibility_modifier"]),
    async_markers: &[Marker {
        token: "async",
        inside: Some("function_modifiers"),
    }],
    class_members_in_scope: false,
    self_receiver_texts: &[],
    parameter_names: Some(ParameterNames {
        fields: &["pattern"],
        kinds: &["identifier"],
        receiver_kinds: &["self_parameter"],
        first_is_receiver_in_type: false,
    }),
    body_siblings: &[],
    sibling_calls: None,
    method_owners: &[],
};
