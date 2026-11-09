//! Scala (tree-sitter-scala 0.24).
//!
//! `this` is an `identifier` in this grammar, so it cannot be declared as a
//! self receiver. Stable paths (`a.b.C` as a type, `stable_identifier`) have
//! no fields, so their path is empty and they are dropped.

use ast_grep_language::SupportLang;

use crate::ast_pipeline::facts::spec::*;

const TYPE_NAMES: &[&str] = &["type_identifier", "stable_type_identifier"];

/// A base: a simple name, or a generic type (read through its `type`).
const BASE_TYPES: &[&str] = &["type_identifier", "generic_type"];

const fn function(kind: &'static str) -> DefinitionSpec {
    DefinitionSpec {
        kind,
        entity: Entity::Function,
        name: NameSpec::Field("name"),
        requires_field: None,
        parameters: Some("parameters"),
        non_parameters: &["comment", "block_comment"],
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

/// `extends A(x) with B with C[T]`: every type of the clause, written
/// directly in it or as a compound type. Scala writes every base with
/// `extends`/`with`, so all are Extends.
const fn bases(kind: &'static str, fields: &'static [&'static str]) -> BaseSpec {
    BaseSpec {
        kind,
        relation: BaseRelation::Extends,
        fields,
        each_child: true,
        through_kinds: BASE_TYPES,
    }
}

const EXTEND: &[&str] = &["extend"];
const EXTEND_COMPOUND: &[&str] = &["extend", "compound_type"];

pub static SCALA: LanguageSpec = LanguageSpec {
    name: "scala",
    extensions: &["scala", "sc"],
    grammar: Grammar::Builtin(SupportLang::Scala),
    definitions: &[
        function("function_definition"),
        function("function_declaration"),
        type_def("class_definition", TypeKind::Class, &[]),
        type_def("object_definition", TypeKind::Class, &[]),
        type_def("package_object", TypeKind::Class, &[]),
        type_def("trait_definition", TypeKind::Trait, &[]),
        type_def(
            "enum_definition",
            TypeKind::Enum,
            &[
                MemberSpec {
                    kind: "simple_enum_case",
                    name: NameSpec::Field("name"),
                },
                MemberSpec {
                    kind: "full_enum_case",
                    name: NameSpec::Field("name"),
                },
            ],
        ),
        DefinitionSpec {
            container: None,
            ..type_def("type_definition", TypeKind::TypeAlias, &[])
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
        names: &["identifier", "type_identifier"],
        scoped: &[],
        members: &[ScopedSpec {
            kind: "field_expression",
            path: "value",
            name: "field",
        }],
        wrappers: &[("generic_function", "function"), ("generic_type", "type")],
    },
    self_receivers: &[],
    imports: &["import_declaration"],
    declarations: &["package_clause"],
    bases: &[
        bases("class_definition", EXTEND),
        bases("class_definition", EXTEND_COMPOUND),
        bases("object_definition", EXTEND),
        bases("object_definition", EXTEND_COMPOUND),
        bases("trait_definition", EXTEND),
        bases("trait_definition", EXTEND_COMPOUND),
        bases("enum_definition", EXTEND),
        bases("enum_definition", EXTEND_COMPOUND),
        bases("package_object", EXTEND),
        bases("package_object", EXTEND_COMPOUND),
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
            kind: "for_expression",
            operators: &[],
        },
        DecisionSpec {
            kind: "while_expression",
            operators: &[],
        },
        DecisionSpec {
            kind: "do_while_expression",
            operators: &[],
        },
        // Every `case` of a match and of a catch block.
        DecisionSpec {
            kind: "case_clause",
            operators: &[],
        },
        DecisionSpec {
            kind: "infix_expression",
            operators: &["&&", "||"],
        },
    ],
    docs: &[DocSpec {
        kind: "block_comment",
        placement: Placement::PrecedingSiblings,
        requires_child: None,
        prefixes: &["/**"],
    }],
    decorators: &[DecoratorSpec {
        kind: "annotation",
        placement: Placement::Children,
    }],
    test_markers: &[TestMarker {
        path: "Test",
        arguments: None,
        on_container: false,
    }],
    test_paths: &[],
    variables: &[
        VariableSpec {
            kind: "val_definition",
            name: NameSpec::PathField("pattern"),
            constant: true,
            value: Some("value"),
        },
        VariableSpec {
            kind: "var_definition",
            name: NameSpec::PathField("pattern"),
            constant: false,
            value: Some("value"),
        },
    ],
    macros: None,
    // Scala definitions are public unless marked private or protected;
    // the spec has no "public unless" rule, so every definition is public.
    visibility: Visibility::Always,
    async_markers: &[],
    class_members_in_scope: true,
    self_receiver_texts: &[],
    parameter_names: None,
    body_siblings: &[],
    sibling_calls: None,
    method_owners: &[],
};
