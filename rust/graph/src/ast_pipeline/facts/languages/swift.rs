//! Swift (tree-sitter-swift 0.7).
//!
//! Calls are `call_expression` nodes whose callee is their first named
//! child (a plain name; a `navigation_expression` callee is an expression
//! callee). Parameters are direct `parameter` children of a function (no field), so
//! parameter counts and parameter types are not read.
//!
//! `class`, `struct`, `enum`, `actor` and `extension` are all
//! `class_declaration`, told apart by the `declaration_kind` token: the
//! table gives them one type kind (`class`). A type's name is its
//! `type_identifier`; an extension's name is a `user_type` (the extended
//! type), so an extension is not a definition and not a container: its
//! members are definitions of the enclosing scope. (Declaring it a Type
//! named after the extended type would make that name ambiguous wherever
//! the type has an extension.)

use ast_grep_language::SupportLang;

use crate::ast_pipeline::facts::spec::*;

const fn function(kind: &'static str, name: NameSpec) -> DefinitionSpec {
    DefinitionSpec {
        kind,
        entity: Entity::Function,
        name,
        requires_field: None,
        parameters: None,
        non_parameters: &[],
        members: &[],
        container: None,
    }
}

const fn type_def(kind: &'static str, type_kind: TypeKind) -> DefinitionSpec {
    DefinitionSpec {
        kind,
        entity: Entity::Type(type_kind),
        name: NameSpec::ChildKind(&["type_identifier"]),
        requires_field: None,
        parameters: None,
        non_parameters: &[],
        members: &[],
        container: Some(ContainerKind::Type),
    }
}

const fn inheritance(kind: &'static str) -> BaseSpec {
    BaseSpec {
        kind,
        relation: BaseRelation::Extends,
        fields: &["inheritance_specifier", "inherits_from"],
        each_child: false,
        through_kinds: &["user_type"],
    }
}

const fn decision(kind: &'static str) -> DecisionSpec {
    DecisionSpec {
        kind,
        operators: &[],
    }
}

pub static SWIFT: LanguageSpec = LanguageSpec {
    name: "swift",
    extensions: &["swift"],
    grammar: Grammar::Builtin(SupportLang::Swift),
    definitions: &[
        function("function_declaration", NameSpec::Field("name")),
        function("protocol_function_declaration", NameSpec::Field("name")),
        // `init` is the name token itself.
        function("init_declaration", NameSpec::Field("name")),
        function("deinit_declaration", NameSpec::ChildKind(&["deinit"])),
        function("subscript_declaration", NameSpec::ChildKind(&["subscript"])),
        type_def("class_declaration", TypeKind::Class),
        type_def("protocol_declaration", TypeKind::Trait),
        DefinitionSpec {
            container: None,
            ..type_def("typealias_declaration", TypeKind::TypeAlias)
        },
    ],
    containers: &[],
    module_declarations: &[],
    calls: &[CallSpec {
        kind: "call_expression",
        callee: None,
        receiver: None,
    }],
    paths: PathSpec {
        separator: ".",
        // A `user_type` is one segment: its text (`Drawable`,
        // `Foundation.Data`, `Array<Int>`).
        names: &["simple_identifier", "type_identifier", "user_type"],
        scoped: &[],
        members: &[],
        wrappers: &[],
    },
    self_receivers: &["self_expression"],
    imports: &["import_declaration"],
    declarations: &[],
    // A class's first inherited type may be its superclass or a protocol;
    // the syntax does not say which.
    bases: &[
        inheritance("class_declaration"),
        inheritance("protocol_declaration"),
    ],
    type_ref_fields: &[],
    decisions: &[
        decision("if_statement"),
        decision("guard_statement"),
        decision("for_statement"),
        decision("while_statement"),
        decision("repeat_while_statement"),
        decision("switch_entry"),
        decision("catch_block"),
        decision("ternary_expression"),
        decision("conjunction_expression"),
        decision("disjunction_expression"),
    ],
    docs: &[
        DocSpec {
            kind: "comment",
            placement: Placement::PrecedingSiblings,
            requires_child: None,
            prefixes: &["///"],
        },
        DocSpec {
            kind: "multiline_comment",
            placement: Placement::PrecedingSiblings,
            requires_child: None,
            prefixes: &["/**"],
        },
    ],
    decorators: &[DecoratorSpec {
        kind: "attribute",
        placement: Placement::ChildChildren("modifiers"),
    }],
    // Swift Testing: `@Test` functions, `@Suite` types.
    test_markers: &[
        TestMarker {
            path: "Test",
            arguments: None,
            on_container: false,
        },
        TestMarker {
            path: "Suite",
            arguments: None,
            on_container: true,
        },
    ],
    test_paths: &[
        TestPath::Directory("Tests"),
        TestPath::StemSuffix("Tests"),
        TestPath::StemSuffix("Test"),
    ],
    variables: &[],
    macros: None,
    visibility: Visibility::Modifier {
        modifiers: "modifiers",
        tokens: &["public", "open"],
    },
    async_markers: &[Marker {
        token: "async",
        inside: None,
    }],
    class_members_in_scope: true,
    self_receiver_texts: &[],
    parameter_names: None,
    body_siblings: &[],
    sibling_calls: None,
    method_owners: &[],
};
