//! C++ (tree-sitter-cpp 0.23). Declares `.h` as well: headers are always
//! parsed with the C++ grammar (product owner, 2026-11-07).
//!
//! A function's name is reached through its declarator chain. An
//! out-of-class definition `double Shape::area() {}` is named by its
//! qualified declarator as written (`Shape::area`) and sits in the
//! container it is written in: the table cannot attach it to the class.
//! Method declarations without a body (`virtual double area();`, a
//! `field_declaration`) and prototypes are not definitions: the table
//! cannot tell them from member and variable declarations.

use ast_grep_language::SupportLang;

use crate::ast_pipeline::facts::spec::*;

const COMMENTS: &[&str] = &["comment"];

const TYPE_NAMES: &[&str] = &["type_identifier", "qualified_identifier", "template_type"];

const fn type_def(
    kind: &'static str,
    type_kind: TypeKind,
    members: &'static [MemberSpec],
) -> DefinitionSpec {
    DefinitionSpec {
        kind,
        entity: Entity::Type(type_kind),
        name: NameSpec::PathField("name"),
        // `class A;` and `struct A a;` name the type; only a body defines it.
        requires_field: Some("body"),
        parameters: None,
        non_parameters: &[],
        members,
        container: Some(ContainerKind::Type),
    }
}

const fn alias_def(kind: &'static str, name: NameSpec) -> DefinitionSpec {
    DefinitionSpec {
        kind,
        entity: Entity::Type(TypeKind::TypeAlias),
        name,
        requires_field: None,
        parameters: None,
        non_parameters: &[],
        members: &[],
        container: None,
    }
}

const fn bases_of(kind: &'static str) -> BaseSpec {
    BaseSpec {
        kind,
        relation: BaseRelation::Extends,
        fields: &["base_class_clause"],
        each_child: true,
        through_kinds: &["type_identifier", "qualified_identifier", "template_type"],
    }
}

const fn decision(kind: &'static str) -> DecisionSpec {
    DecisionSpec {
        kind,
        operators: &[],
    }
}

pub static CPP: LanguageSpec = LanguageSpec {
    name: "cpp",
    extensions: &["cpp", "cc", "cxx", "hpp", "hh", "hxx", "h"],
    grammar: Grammar::Builtin(SupportLang::Cpp),
    definitions: &[
        DefinitionSpec {
            kind: "function_definition",
            entity: Entity::Function,
            name: NameSpec::Descend {
                field: "declarator",
                until: &[
                    "identifier",
                    "field_identifier",
                    "qualified_identifier",
                    "destructor_name",
                    "operator_name",
                    "template_function",
                ],
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
        DefinitionSpec {
            kind: "field_declaration",
            entity: Entity::Function,
            name: NameSpec::Descend {
                field: "declarator",
                until: &[
                    "identifier",
                    "field_identifier",
                    "qualified_identifier",
                    "destructor_name",
                    "operator_name",
                    "template_function",
                ],
                through: &["function_declarator"],
            },
            requires_field: None,
            // The parameter list is a field of the function_declarator, not
            // of the definition.
            parameters: None,
            non_parameters: COMMENTS,
            members: &[],
            container: None,
        },
        type_def("class_specifier", TypeKind::Class, &[]),
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
        alias_def(
            "type_definition",
            NameSpec::Descend {
                field: "declarator",
                until: &["type_identifier"],
                through: &[],
            },
        ),
        alias_def("alias_declaration", NameSpec::Field("name")),
    ],
    containers: &[ContainerSpec {
        kind: "namespace_definition",
        container: ContainerKind::Module,
        name: Some(NameSpec::Field("name")),
        // The members of an anonymous namespace are names of the file.
        requires_field: Some("name"),
    }],
    module_declarations: &[],
    calls: &[CallSpec {
        kind: "call_expression",
        callee: Some("function"),
        receiver: None,
    }],
    paths: PathSpec {
        separator: "::",
        names: &[
            "identifier",
            "field_identifier",
            "type_identifier",
            "namespace_identifier",
        ],
        scoped: &[ScopedSpec {
            kind: "qualified_identifier",
            path: "scope",
            name: "name",
        }],
        members: &[ScopedSpec {
            kind: "field_expression",
            path: "argument",
            name: "field",
        }],
        wrappers: &[
            ("template_function", "name"),
            ("template_type", "name"),
            // Variable declarators: `int *p = 0;`, `int a[3];`.
            ("init_declarator", "declarator"),
            ("pointer_declarator", "declarator"),
            ("array_declarator", "declarator"),
        ],
    },
    self_receivers: &["this"],
    imports: &["preproc_include"],
    declarations: &[],
    bases: &[
        bases_of("class_specifier"),
        bases_of("struct_specifier"),
        bases_of("union_specifier"),
    ],
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
        decision("if_statement"),
        decision("for_statement"),
        decision("for_range_loop"),
        decision("while_statement"),
        decision("do_statement"),
        decision("case_statement"),
        decision("catch_clause"),
        decision("conditional_expression"),
        DecisionSpec {
            kind: "binary_expression",
            operators: &["&&", "||", "and", "or"],
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
            kind: "attribute_declaration",
            placement: Placement::Children,
        },
        DecoratorSpec {
            kind: "attribute_specifier",
            placement: Placement::Children,
        },
    ],
    test_markers: &[],
    test_paths: &[
        TestPath::Directory("test"),
        TestPath::Directory("tests"),
        TestPath::StemSuffix("_test"),
        TestPath::StemSuffix("_unittest"),
    ],
    variables: &[
        // A declaration whose declarator is a function_declarator (a
        // prototype or a method declaration) has no path, so it is not a
        // variable.
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
            value: Some("default_value"),
        },
    ],
    macros: None,
    // Access is given by `public:` / `private:` labels before the member;
    // the table cannot read a label sibling.
    visibility: Visibility::Always,
    async_markers: &[],
    class_members_in_scope: true,
    self_receiver_texts: &[],
    parameter_names: None,
    body_siblings: &[],
    sibling_calls: None,
    method_owners: &[],
};
