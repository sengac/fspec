//! Python (tree-sitter-python 0.25).

use ast_grep_language::SupportLang;

use crate::ast_pipeline::facts::spec::*;

/// Named children of `parameters` that are not parameters: the `/` and `*`
/// separators, and comments.
const NON_PARAMETERS: &[&str] = &["positional_separator", "keyword_separator", "comment"];

pub static PYTHON: LanguageSpec = LanguageSpec {
    name: "python",
    extensions: &["py", "pyi"],
    grammar: Grammar::Builtin(SupportLang::Python),
    definitions: &[
        // `def` and `async def`, at module level, in classes and nested.
        DefinitionSpec {
            kind: "function_definition",
            entity: Entity::Function,
            name: NameSpec::Field("name"),
            requires_field: None,
            parameters: Some("parameters"),
            non_parameters: NON_PARAMETERS,
            members: &[],
            container: None,
        },
        DefinitionSpec {
            kind: "class_definition",
            entity: Entity::Type(TypeKind::Class),
            name: NameSpec::Field("name"),
            requires_field: None,
            parameters: None,
            non_parameters: &[],
            members: &[],
            container: Some(ContainerKind::Type),
        },
    ],
    containers: &[],
    module_declarations: &[],
    calls: &[CallSpec {
        kind: "call",
        callee: Some("function"),
        receiver: None,
    }],
    paths: PathSpec {
        separator: ".",
        names: &["identifier"],
        scoped: &[],
        members: &[ScopedSpec {
            kind: "attribute",
            path: "object",
            name: "attribute",
        }],
        wrappers: &[],
    },
    // `self` is a parameter name, not a syntax kind.
    self_receivers: &[],
    imports: &["import_statement", "import_from_statement"],
    declarations: &[],
    bases: &[BaseSpec {
        kind: "class_definition",
        relation: BaseRelation::Extends,
        fields: &["superclasses"],
        each_child: true,
        // Keyword arguments (`metaclass=M`) and splats are not bases.
        through_kinds: &["identifier", "attribute"],
    }],
    // The return annotation is a `type` node: every name inside it is a
    // type name. Parameter annotations are not declared: the names of
    // untyped parameters and of default values are identifiers too.
    type_ref_fields: &[
        TypeRefField {
            field: "parameters",
            ref_kind: RefKind::Param,
            type_kinds: &["identifier", "attribute"],
            item_field: Some("type"),
        },
        TypeRefField {
            field: "return_type",
            ref_kind: RefKind::ReturnType,
            type_kinds: &["identifier", "attribute"],
            item_field: None,
        },
    ],
    decisions: &[
        DecisionSpec {
            kind: "if_statement",
            operators: &[],
        },
        DecisionSpec {
            kind: "elif_clause",
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
            kind: "except_clause",
            operators: &[],
        },
        DecisionSpec {
            kind: "case_clause",
            operators: &[],
        },
        DecisionSpec {
            kind: "conditional_expression",
            operators: &[],
        },
        DecisionSpec {
            kind: "for_in_clause",
            operators: &[],
        },
        DecisionSpec {
            kind: "if_clause",
            operators: &[],
        },
        DecisionSpec {
            kind: "boolean_operator",
            operators: &["and", "or"],
        },
    ],
    docs: &[DocSpec {
        kind: "string",
        placement: Placement::BodyFirst {
            body: "body",
            statement: "expression_statement",
        },
        requires_child: None,
        prefixes: &[],
    }],
    decorators: &[DecoratorSpec {
        kind: "decorator",
        placement: Placement::ParentChildren("decorated_definition"),
    }],
    test_markers: &[],
    test_paths: &[
        TestPath::FilePrefix("test_"),
        TestPath::StemSuffix("_test"),
        TestPath::FilePrefix("conftest."),
        TestPath::Directory("tests"),
    ],
    variables: &[VariableSpec {
        kind: "assignment",
        // Only `name = ...`: attribute, subscript and tuple targets are
        // not variables of the module or class.
        name: NameSpec::Descend {
            field: "left",
            until: &["identifier"],
            through: &[],
        },
        constant: false,
        value: Some("right"),
    }],
    macros: None,
    // A leading underscore is the documented convention for non-public
    // names (PEP 8).
    visibility: Visibility::PrivatePrefix("_"),
    async_markers: &[Marker {
        token: "async",
        inside: None,
    }],
    class_members_in_scope: false,
    self_receiver_texts: &[],
    parameter_names: Some(ParameterNames {
        fields: &["name"],
        kinds: &["identifier"],
        receiver_kinds: &[],
        first_is_receiver_in_type: true,
    }),
    body_siblings: &[],
    sibling_calls: None,
    method_owners: &[],
};
