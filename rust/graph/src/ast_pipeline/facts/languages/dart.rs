//! Dart (tree-sitter-dart 0.1, declared by codelet-tools).
//!
//! The grammar splits a function into a signature node and a sibling
//! `function_body` (top level: both children of `source_file`; members:
//! both children of `class_member`, which has no fields). A function
//! definition is its signature node: its span, source and line range cover
//! the signature only, and its body is outside it, so decisions in the body
//! are not counted and definitions in the body are not nested in it.
//!
//! Calls are not declared: the grammar has no call node (`f()` is an
//! `identifier` followed by a `selector` sibling).

use crate::ast_pipeline::facts::spec::*;

const TYPE_NAMES: &[&str] = &["type_identifier"];

const fn signature(
    kind: &'static str,
    name: NameSpec,
    parameters: Option<&'static str>,
) -> DefinitionSpec {
    DefinitionSpec {
        kind,
        entity: Entity::Function,
        name,
        requires_field: None,
        parameters,
        non_parameters: &["comment"],
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

const fn base(
    kind: &'static str,
    relation: BaseRelation,
    fields: &'static [&'static str],
    each_child: bool,
) -> BaseSpec {
    BaseSpec {
        kind,
        relation,
        fields,
        each_child,
        through_kinds: TYPE_NAMES,
    }
}

const fn decision(kind: &'static str) -> DecisionSpec {
    DecisionSpec {
        kind,
        operators: &[],
    }
}

const EXTENSION: ContainerKind = ContainerKind::Impl {
    self_type: "class",
    trait_field: None,
};

pub static DART: LanguageSpec = LanguageSpec {
    name: "dart",
    extensions: &["dart"],
    grammar: Grammar::Dart,
    definitions: &[
        signature(
            "function_signature",
            NameSpec::Field("name"),
            Some("parameters"),
        ),
        signature("getter_signature", NameSpec::Field("name"), None),
        signature(
            "setter_signature",
            NameSpec::Field("name"),
            Some("parameters"),
        ),
        signature(
            "operator_signature",
            NameSpec::Field("operator"),
            Some("parameters"),
        ),
        // A constructor's `name` field holds the class name first and, for a
        // named constructor `User.named`, the constructor's name last.
        signature(
            "constructor_signature",
            NameSpec::LastField("name"),
            Some("parameters"),
        ),
        signature(
            "constant_constructor_signature",
            NameSpec::LastField("name"),
            Some("parameters"),
        ),
        signature(
            "factory_constructor_signature",
            NameSpec::LastField("name"),
            Some("parameters"),
        ),
        signature(
            "redirecting_factory_constructor_signature",
            NameSpec::LastField("name"),
            Some("parameters"),
        ),
        type_def("class_declaration", TypeKind::Class, &[]),
        type_def("mixin_declaration", TypeKind::Trait, &[]),
        type_def(
            "enum_declaration",
            TypeKind::Enum,
            &[MemberSpec {
                kind: "enum_constant",
                name: NameSpec::Field("name"),
            }],
        ),
        type_def("extension_type_declaration", TypeKind::Class, &[]),
        DefinitionSpec {
            requires_field: Some("name"),
            container: Some(EXTENSION),
            ..type_def("extension_declaration", TypeKind::Extension, &[])
        },
        DefinitionSpec {
            container: None,
            name: NameSpec::ChildKind(&["type_identifier"]),
            ..type_def("type_alias", TypeKind::TypeAlias, &[])
        },
    ],
    // `extension on String { ... }` has no name: an implementation block
    // only.
    containers: &[ContainerSpec {
        kind: "extension_declaration",
        container: EXTENSION,
        name: None,
        requires_field: None,
    }],
    module_declarations: &[],
    calls: &[],
    paths: PathSpec {
        separator: ".",
        names: &["identifier", "type_identifier"],
        scoped: &[],
        members: &[],
        wrappers: &[],
    },
    self_receivers: &["this"],
    imports: &["import_or_export", "part_directive"],
    declarations: &[],
    bases: &[
        base(
            "class_declaration",
            BaseRelation::Extends,
            &["superclass", "type"],
            false,
        ),
        // `with` mixins are applied into the superclass chain.
        base(
            "class_declaration",
            BaseRelation::Extends,
            &["superclass", "mixins"],
            true,
        ),
        base(
            "class_declaration",
            BaseRelation::Implements,
            &["interfaces"],
            true,
        ),
        // `mixin M on B`: the `on` types are direct children.
        base("mixin_declaration", BaseRelation::Extends, &[], true),
        base(
            "mixin_declaration",
            BaseRelation::Implements,
            &["interfaces"],
            true,
        ),
        base("enum_declaration", BaseRelation::Extends, &["mixins"], true),
        base(
            "enum_declaration",
            BaseRelation::Implements,
            &["interfaces"],
            true,
        ),
        base(
            "extension_type_declaration",
            BaseRelation::Implements,
            &["interfaces"],
            true,
        ),
    ],
    type_ref_fields: &[
        TypeRefField {
            field: "return_type",
            ref_kind: RefKind::ReturnType,
            type_kinds: TYPE_NAMES,
            item_field: None,
        },
        TypeRefField {
            field: "parameters",
            ref_kind: RefKind::Param,
            type_kinds: TYPE_NAMES,
            item_field: None,
        },
    ],
    decisions: &[
        decision("if_statement"),
        decision("for_statement"),
        decision("while_statement"),
        decision("do_statement"),
        decision("switch_statement_case"),
        decision("switch_expression_case"),
        decision("catch_clause"),
        decision("conditional_expression"),
        decision("logical_and_expression"),
        decision("logical_or_expression"),
    ],
    docs: &[DocSpec {
        kind: "comment",
        placement: Placement::PrecedingSiblings,
        requires_child: None,
        prefixes: &["///", "/**"],
    }],
    decorators: &[
        // Types hold their annotations; a top-level function's annotations
        // precede its signature.
        DecoratorSpec {
            kind: "annotation",
            placement: Placement::Children,
        },
        DecoratorSpec {
            kind: "annotation",
            placement: Placement::PrecedingSiblings,
        },
    ],
    test_markers: &[],
    test_paths: &[
        TestPath::Directory("test"),
        TestPath::Directory("integration_test"),
        TestPath::StemSuffix("_test"),
    ],
    variables: &[
        // Top-level and member `final` / `const` declarations.
        VariableSpec {
            kind: "static_final_declaration",
            name: NameSpec::Field("name"),
            constant: true,
            value: Some("value"),
        },
        VariableSpec {
            kind: "initialized_identifier",
            name: NameSpec::Field("name"),
            constant: false,
            value: Some("value"),
        },
    ],
    macros: None,
    // Library privacy: a name starting with `_` is private to its library.
    visibility: Visibility::PrivatePrefix("_"),
    async_markers: &[],
    class_members_in_scope: true,
    self_receiver_texts: &[],
    parameter_names: None,
    body_siblings: &[
        ("function_signature", "function_body"),
        ("getter_signature", "function_body"),
        ("setter_signature", "function_body"),
        ("operator_signature", "function_body"),
        ("constructor_signature", "function_body"),
        ("factory_constructor_signature", "function_body"),
    ],
    sibling_calls: Some(SiblingCallSpec {
        selector: "selector",
        member_selectors: &[
            "unconditional_assignable_selector",
            "conditional_assignable_selector",
        ],
        call_part: "argument_part",
    }),
    method_owners: &[],
};
