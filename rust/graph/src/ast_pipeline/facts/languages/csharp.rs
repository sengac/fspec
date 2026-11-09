//! C# (tree-sitter-c-sharp 0.23).
//!
//! Properties, indexers and events are not functions: they are accessed,
//! not called. Generic names (`List<T>`, `M<T>()`) have no fields, so a
//! generic call or type reference is dropped; a generic base is read
//! through its first `identifier`.

use ast_grep_language::SupportLang;

use crate::ast_pipeline::facts::spec::*;

const fn function(kind: &'static str, name: &'static str) -> DefinitionSpec {
    DefinitionSpec {
        kind,
        entity: Entity::Function,
        name: NameSpec::Field(name),
        requires_field: None,
        parameters: Some("parameters"),
        non_parameters: &["comment", "attribute_list"],
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

/// A base: a name, a qualified name, or a record's `Base(args)`.
const BASE_TYPES: &[&str] = &[
    "identifier",
    "qualified_name",
    "primary_constructor_base_type",
];

/// The bases of `kind`, as written and through a generic name (`Base<T>`,
/// whose `identifier` child is the base).
const fn bases(kind: &'static str, relation: BaseRelation, generic: bool) -> BaseSpec {
    BaseSpec {
        kind,
        relation,
        fields: if generic {
            &["base_list", "generic_name"]
        } else {
            &["base_list"]
        },
        each_child: true,
        through_kinds: if generic { &["identifier"] } else { BASE_TYPES },
    }
}

pub static CSHARP: LanguageSpec = LanguageSpec {
    name: "csharp",
    extensions: &["cs"],
    grammar: Grammar::Builtin(SupportLang::CSharp),
    definitions: &[
        function("method_declaration", "name"),
        function("constructor_declaration", "name"),
        function("destructor_declaration", "name"),
        function("local_function_statement", "name"),
        function("operator_declaration", "operator"),
        type_def("class_declaration", TypeKind::Class, &[]),
        type_def("struct_declaration", TypeKind::Struct, &[]),
        type_def("interface_declaration", TypeKind::Interface, &[]),
        type_def(
            "enum_declaration",
            TypeKind::Enum,
            &[MemberSpec {
                kind: "enum_member_declaration",
                name: NameSpec::Field("name"),
            }],
        ),
        // `record` and `record struct`.
        type_def("record_declaration", TypeKind::Class, &[]),
        DefinitionSpec {
            container: None,
            ..type_def("delegate_declaration", TypeKind::TypeAlias, &[])
        },
    ],
    containers: &[],
    module_declarations: &[],
    calls: &[CallSpec {
        kind: "invocation_expression",
        callee: Some("function"),
        receiver: None,
    }],
    paths: PathSpec {
        separator: ".",
        names: &["identifier"],
        scoped: &[ScopedSpec {
            kind: "qualified_name",
            path: "qualifier",
            name: "name",
        }],
        members: &[ScopedSpec {
            kind: "member_access_expression",
            path: "expression",
            name: "name",
        }],
        wrappers: &[("primary_constructor_base_type", "type")],
    },
    self_receivers: &["this"],
    imports: &["using_directive"],
    declarations: &["namespace_declaration", "file_scoped_namespace_declaration"],
    // The base list does not say which base is the class: the bases of a
    // class or record are Extends; a struct only implements interfaces;
    // an interface extends interfaces. An enum's base list is its
    // underlying integral type, not a base.
    bases: &[
        bases("class_declaration", BaseRelation::Extends, false),
        bases("class_declaration", BaseRelation::Extends, true),
        bases("record_declaration", BaseRelation::Extends, false),
        bases("record_declaration", BaseRelation::Extends, true),
        bases("struct_declaration", BaseRelation::Implements, false),
        bases("struct_declaration", BaseRelation::Implements, true),
        bases("interface_declaration", BaseRelation::Extends, false),
        bases("interface_declaration", BaseRelation::Extends, true),
    ],
    // Parameter types are not declared: inside a parameter list the
    // parameter names are `identifier`s too.
    type_ref_fields: &[
        TypeRefField {
            field: "parameters",
            ref_kind: RefKind::Param,
            type_kinds: &["identifier", "qualified_name"],
            item_field: Some("type"),
        },
        TypeRefField {
            field: "returns",
            ref_kind: RefKind::ReturnType,
            type_kinds: &["identifier", "qualified_name"],
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
            kind: "foreach_statement",
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
            kind: "switch_section",
            operators: &[],
        },
        DecisionSpec {
            kind: "switch_expression_arm",
            operators: &[],
        },
        DecisionSpec {
            kind: "catch_clause",
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
        prefixes: &["///", "/**"],
    }],
    decorators: &[DecoratorSpec {
        kind: "attribute_list",
        placement: Placement::Children,
    }],
    test_markers: &[
        // xUnit
        TestMarker {
            path: "Fact",
            arguments: None,
            on_container: false,
        },
        TestMarker {
            path: "Theory",
            arguments: None,
            on_container: false,
        },
        // NUnit
        TestMarker {
            path: "Test",
            arguments: None,
            on_container: false,
        },
        TestMarker {
            path: "TestCase",
            arguments: None,
            on_container: false,
        },
        TestMarker {
            path: "TestCaseSource",
            arguments: None,
            on_container: false,
        },
        TestMarker {
            path: "TestFixture",
            arguments: None,
            on_container: true,
        },
        // MSTest
        TestMarker {
            path: "TestMethod",
            arguments: None,
            on_container: false,
        },
        TestMarker {
            path: "DataTestMethod",
            arguments: None,
            on_container: false,
        },
        TestMarker {
            path: "TestClass",
            arguments: None,
            on_container: true,
        },
    ],
    test_paths: &[],
    // A field's name is inside a field-less `variable_declaration`.
    variables: &[],
    macros: None,
    visibility: Visibility::Modifier {
        modifiers: "modifier",
        tokens: &["public"],
    },
    async_markers: &[Marker {
        token: "async",
        inside: Some("modifier"),
    }],
    class_members_in_scope: true,
    self_receiver_texts: &[],
    parameter_names: None,
    body_siblings: &[],
    sibling_calls: None,
    method_owners: &[],
};
