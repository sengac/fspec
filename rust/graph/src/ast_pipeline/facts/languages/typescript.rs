//! TypeScript (tree-sitter-typescript 0.23, `typescript` grammar). The
//! `tsx` grammar has the same node kinds for everything declared here;
//! `tsx.rs` uses the same table with its own grammar and extensions.

use ast_grep_language::SupportLang;

use crate::ast_pipeline::facts::spec::*;

const NON_PARAMETERS: &[&str] = &["comment"];

const fn function(kind: &'static str) -> DefinitionSpec {
    DefinitionSpec {
        kind,
        entity: Entity::Function,
        name: NameSpec::Field("name"),
        requires_field: None,
        parameters: Some("parameters"),
        non_parameters: NON_PARAMETERS,
        members: &[],
        container: None,
    }
}

const fn type_def(
    kind: &'static str,
    type_kind: TypeKind,
    container: Option<ContainerKind>,
) -> DefinitionSpec {
    DefinitionSpec {
        kind,
        entity: Entity::Type(type_kind),
        name: NameSpec::Field("name"),
        requires_field: None,
        parameters: None,
        non_parameters: &[],
        members: &[],
        container,
    }
}

/// Function and method declarations. Overload and ambient signatures
/// (`function_signature`, `method_signature`) are not declared: an overload
/// set would make every call to the implementation ambiguous.
/// Arrow functions and function expressions have no name of their own.
const DEFINITIONS: &[DefinitionSpec] = &[
    function("function_declaration"),
    function("generator_function_declaration"),
    function("method_definition"),
    function("abstract_method_signature"),
    type_def(
        "class_declaration",
        TypeKind::Class,
        Some(ContainerKind::Type),
    ),
    type_def(
        "abstract_class_declaration",
        TypeKind::Class,
        Some(ContainerKind::Type),
    ),
    type_def(
        "interface_declaration",
        TypeKind::Interface,
        Some(ContainerKind::Type),
    ),
    type_def("type_alias_declaration", TypeKind::TypeAlias, None),
    type_def("enum_declaration", TypeKind::Enum, None),
];

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

/// Scopes that are not definitions: namespaces, class expressions, the
/// bodies of unnamed functions (so that what they declare is not taken as
/// module-level), and object literals (whose methods are not module-level
/// functions).
const CONTAINERS: &[ContainerSpec] = &[
    scope(
        "internal_module",
        ContainerKind::Module,
        Some(NameSpec::Field("name")),
    ),
    scope(
        "module",
        ContainerKind::Module,
        Some(NameSpec::Field("name")),
    ),
    scope("class", ContainerKind::Type, Some(NameSpec::Field("name"))),
    scope("arrow_function", ContainerKind::Function, None),
    scope("function_expression", ContainerKind::Function, None),
    scope("generator_function", ContainerKind::Function, None),
    scope("object", ContainerKind::Module, None),
];

const EXTENDS: BaseSpec = BaseSpec {
    kind: "class_declaration",
    relation: BaseRelation::Extends,
    fields: &["class_heritage", "extends_clause"],
    each_child: true,
    through_kinds: &[],
};

const IMPLEMENTS: BaseSpec = BaseSpec {
    kind: "class_declaration",
    relation: BaseRelation::Implements,
    fields: &["class_heritage", "implements_clause"],
    each_child: true,
    through_kinds: &[],
};

const BASES: &[BaseSpec] = &[
    EXTENDS,
    IMPLEMENTS,
    BaseSpec {
        kind: "abstract_class_declaration",
        ..EXTENDS
    },
    BaseSpec {
        kind: "abstract_class_declaration",
        ..IMPLEMENTS
    },
    BaseSpec {
        kind: "interface_declaration",
        relation: BaseRelation::Extends,
        fields: &["extends_type_clause"],
        each_child: true,
        through_kinds: &[],
    },
];

const TYPE_NAMES: &[&str] = &["type_identifier", "nested_type_identifier"];

/// Decision nodes of TypeScript and JavaScript.
pub(super) const DECISIONS: &[DecisionSpec] = &[
    DecisionSpec {
        kind: "if_statement",
        operators: &[],
    },
    DecisionSpec {
        kind: "for_statement",
        operators: &[],
    },
    DecisionSpec {
        kind: "for_in_statement",
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
        kind: "switch_case",
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
];

/// JSDoc comments right before a declaration.
pub(super) const DOCS: &[DocSpec] = &[DocSpec {
    kind: "comment",
    placement: Placement::PrecedingSiblings,
    requires_child: None,
    prefixes: &["/**"],
}];

/// Class decorators are children of the class; member decorators precede
/// the member in the class body; `@d export class` puts them before the
/// class inside the export statement.
pub(super) const DECORATORS: &[DecoratorSpec] = &[
    DecoratorSpec {
        kind: "decorator",
        placement: Placement::Children,
    },
    DecoratorSpec {
        kind: "decorator",
        placement: Placement::PrecedingSiblings,
    },
];

/// `a.test.ts`, `a.spec.ts`, and Jest's `__tests__` directories.
pub(super) const TEST_PATHS: &[TestPath] = &[
    TestPath::NamePart("test"),
    TestPath::NamePart("spec"),
    TestPath::Directory("__tests__"),
];

/// A table of the TypeScript node kinds parsed with `grammar`.
pub(super) const fn spec(extensions: &'static [&'static str], grammar: Grammar) -> LanguageSpec {
    LanguageSpec {
        name: "typescript",
        extensions,
        grammar,
        definitions: DEFINITIONS,
        containers: CONTAINERS,
        module_declarations: &[],
        calls: &[CallSpec {
            kind: "call_expression",
            callee: Some("function"),
            receiver: None,
        }],
        paths: PathSpec {
            separator: ".",
            names: &["identifier", "type_identifier"],
            scoped: &[
                ScopedSpec {
                    kind: "nested_type_identifier",
                    path: "module",
                    name: "name",
                },
                ScopedSpec {
                    kind: "nested_identifier",
                    path: "object",
                    name: "property",
                },
            ],
            members: &[ScopedSpec {
                kind: "member_expression",
                path: "object",
                name: "property",
            }],
            wrappers: &[("generic_type", "name")],
        },
        self_receivers: &["this"],
        imports: &["import_statement"],
        // Export statements, for named, default and re-exports.
        declarations: &["export_statement"],
        bases: BASES,
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
        decisions: DECISIONS,
        docs: DOCS,
        decorators: DECORATORS,
        test_markers: &[],
        test_paths: TEST_PATHS,
        variables: &[
            VariableSpec {
                kind: "variable_declarator",
                // Destructuring patterns are not one variable.
                name: NameSpec::Descend {
                    field: "name",
                    until: &["identifier"],
                    through: &[],
                },
                constant: false,
                value: Some("value"),
            },
            VariableSpec {
                kind: "public_field_definition",
                name: NameSpec::Field("name"),
                constant: false,
                value: Some("value"),
            },
        ],
        macros: None,
        // ES modules: a name is visible to other modules when exported.
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
    }
}

pub static TYPESCRIPT: LanguageSpec = spec(
    &["ts", "mts", "cts"],
    Grammar::Builtin(SupportLang::TypeScript),
);
