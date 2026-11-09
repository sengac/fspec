//! Go (tree-sitter-go 0.25).
//!
//! A method (`func (r *T) M()`) has no enclosing container node: it is
//! declared at file level and is a file-level definition here; its
//! receiver type is not a container.

use ast_grep_language::SupportLang;

use crate::ast_pipeline::facts::spec::*;

const NON_PARAMETERS: &[&str] = &["comment"];

const TYPE_NAMES: &[&str] = &["type_identifier", "qualified_type"];

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

const fn type_def(kind: &'static str, type_kind: TypeKind) -> DefinitionSpec {
    DefinitionSpec {
        kind,
        entity: Entity::Type(type_kind),
        name: NameSpec::Field("name"),
        requires_field: None,
        parameters: None,
        non_parameters: &[],
        members: &[],
        container: None,
    }
}

pub static GO: LanguageSpec = LanguageSpec {
    name: "go",
    extensions: &["go"],
    grammar: Grammar::Builtin(SupportLang::Go),
    definitions: &[
        FUNCTION,
        DefinitionSpec {
            kind: "method_declaration",
            ..FUNCTION
        },
        // `type T struct{...}`, `type I interface{...}`, `type D int`: the
        // spec has one entity per node kind, so every type_spec is a
        // struct_kind Type.
        type_def("type_spec", TypeKind::Struct),
        // `type A = B`.
        type_def("type_alias", TypeKind::TypeAlias),
    ],
    // A function literal's body is a function scope: its `var`s are not
    // package variables.
    containers: &[ContainerSpec {
        kind: "func_literal",
        container: ContainerKind::Function,
        name: None,
        requires_field: None,
    }],
    module_declarations: &[],
    calls: &[CallSpec {
        kind: "call_expression",
        callee: Some("function"),
        receiver: None,
    }],
    paths: PathSpec {
        separator: ".",
        names: &["identifier", "type_identifier", "package_identifier"],
        scoped: &[ScopedSpec {
            kind: "qualified_type",
            path: "package",
            name: "name",
        }],
        members: &[ScopedSpec {
            kind: "selector_expression",
            path: "operand",
            name: "field",
        }],
        wrappers: &[("generic_type", "type")],
    },
    // The receiver is a parameter name, not a syntax kind.
    self_receivers: &[],
    imports: &["import_declaration"],
    declarations: &["package_clause"],
    // Go has no inheritance; embedding is not declared as a base.
    bases: &[],
    type_ref_fields: &[
        TypeRefField {
            field: "parameters",
            ref_kind: RefKind::Param,
            type_kinds: TYPE_NAMES,
            item_field: None,
        },
        TypeRefField {
            field: "result",
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
            kind: "expression_case",
            operators: &[],
        },
        DecisionSpec {
            kind: "type_case",
            operators: &[],
        },
        DecisionSpec {
            kind: "communication_case",
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
        prefixes: &["//"],
    }],
    decorators: &[],
    test_markers: &[],
    test_paths: &[TestPath::StemSuffix("_test")],
    variables: &[
        VariableSpec {
            kind: "const_spec",
            name: NameSpec::Field("name"),
            constant: true,
            value: Some("value"),
        },
        VariableSpec {
            kind: "var_spec",
            name: NameSpec::Field("name"),
            constant: false,
            value: Some("value"),
        },
    ],
    macros: None,
    // The Go rule: an identifier is exported iff its first character is a
    // Unicode upper-case letter.
    visibility: Visibility::UpperInitial,
    async_markers: &[],
    class_members_in_scope: false,
    self_receiver_texts: &[],
    parameter_names: Some(ParameterNames {
        fields: &["name"],
        kinds: &["identifier"],
        receiver_kinds: &[],
        first_is_receiver_in_type: false,
    }),
    body_siblings: &[],
    sibling_calls: None,
    method_owners: &[MethodOwnerSpec {
        kind: "method_declaration",
        field: "receiver",
        type_kinds: &["type_identifier"],
    }],
};
