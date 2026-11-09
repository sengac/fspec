//! JavaScript (tree-sitter-javascript 0.25), for `.js`, `.jsx`, `.mjs` and
//! `.cjs`. Decision nodes, docs, decorators and test paths are those of
//! TypeScript.

use ast_grep_language::SupportLang;

use super::typescript::{DECISIONS, DECORATORS, DOCS, TEST_PATHS};
use crate::ast_pipeline::facts::spec::*;

const NON_PARAMETERS: &[&str] = &["comment"];

const FUNCTION: DefinitionSpec = DefinitionSpec {
    kind: "function_declaration",
    entity: Entity::Function,
    name: NameSpec::Field("name"),
    requires_field: None,
    parameters: Some("parameters"),
    non_parameters: NON_PARAMETERS,
    members: &[],
    container: None,
};

const fn scope(
    kind: &'static str,
    container: ContainerKind,
    name: Option<NameSpec>,
) -> ContainerSpec {
    ContainerSpec {
        kind,
        container,
        name,
        requires_field: None,
    }
}

pub static JAVASCRIPT: LanguageSpec = LanguageSpec {
    name: "javascript",
    extensions: &["js", "jsx", "mjs", "cjs"],
    grammar: Grammar::Builtin(SupportLang::JavaScript),
    // Arrow functions and function expressions have no name of their own.
    definitions: &[
        FUNCTION,
        DefinitionSpec {
            kind: "generator_function_declaration",
            ..FUNCTION
        },
        DefinitionSpec {
            kind: "method_definition",
            ..FUNCTION
        },
        DefinitionSpec {
            kind: "class_declaration",
            entity: Entity::Type(TypeKind::Class),
            name: NameSpec::Field("name"),
            requires_field: None,
            parameters: None,
            non_parameters: &[],
            members: &[],
            container: Some(ContainerKind::Type),
        },
    ],
    // See typescript.rs: class expressions, unnamed function bodies and
    // object literals are scopes.
    containers: &[
        scope("class", ContainerKind::Type, Some(NameSpec::Field("name"))),
        scope("arrow_function", ContainerKind::Function, None),
        scope("function_expression", ContainerKind::Function, None),
        scope("generator_function", ContainerKind::Function, None),
        scope("object", ContainerKind::Module, None),
    ],
    module_declarations: &[],
    calls: &[CallSpec {
        kind: "call_expression",
        callee: Some("function"),
        receiver: None,
    }],
    paths: PathSpec {
        separator: ".",
        names: &["identifier"],
        scoped: &[],
        members: &[ScopedSpec {
            kind: "member_expression",
            path: "object",
            name: "property",
        }],
        wrappers: &[],
    },
    self_receivers: &["this"],
    imports: &["import_statement"],
    declarations: &["export_statement"],
    bases: &[BaseSpec {
        kind: "class_declaration",
        relation: BaseRelation::Extends,
        fields: &["class_heritage"],
        each_child: true,
        through_kinds: &[],
    }],
    type_ref_fields: &[],
    decisions: DECISIONS,
    docs: DOCS,
    decorators: DECORATORS,
    test_markers: &[],
    test_paths: TEST_PATHS,
    variables: &[
        VariableSpec {
            kind: "variable_declarator",
            name: NameSpec::Descend {
                field: "name",
                until: &["identifier"],
                through: &[],
            },
            constant: false,
            value: Some("value"),
        },
        VariableSpec {
            kind: "field_definition",
            name: NameSpec::Field("property"),
            constant: false,
            value: Some("value"),
        },
    ],
    macros: None,
    visibility: Visibility::ParentKind(&["export_statement"]),
    async_markers: &[Marker {
        token: "async",
        inside: None,
    }],
    class_members_in_scope: false,
    self_receiver_texts: &[],
    parameter_names: Some(ParameterNames {
        fields: &["pattern"],
        kinds: &["identifier"],
        receiver_kinds: &[],
        first_is_receiver_in_type: false,
    }),
    body_siblings: &[],
    sibling_calls: None,
    method_owners: &[],
};
