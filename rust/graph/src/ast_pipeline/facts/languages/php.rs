//! PHP (tree-sitter-php 0.24, the `php_only` grammar ast-grep uses).
//!
//! Calls: `function_call_expression`, and member and static calls
//! (`$this->f()`, `$u->f()`, `User::make()`) read through their `name` and
//! `object`/`scope` fields; `$this` is the self receiver, by its text.
//!
//! A plain name inside a method never names a method of the class (PHP
//! requires `$this->` / `self::`), so class members are not in scope.
//!
//! `namespace A\B;` is a declaration; a braced `namespace A\B { ... }` is
//! also a Module container.

use ast_grep_language::SupportLang;

use crate::ast_pipeline::facts::spec::*;

const COMMENTS: &[&str] = &["comment"];

const TYPE_NAMES: &[&str] = &["named_type"];

const fn function(kind: &'static str) -> DefinitionSpec {
    DefinitionSpec {
        kind,
        entity: Entity::Function,
        name: NameSpec::Field("name"),
        requires_field: None,
        parameters: Some("parameters"),
        non_parameters: COMMENTS,
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
) -> BaseSpec {
    BaseSpec {
        kind,
        relation,
        fields,
        each_child: true,
        through_kinds: &["name"],
    }
}

const fn decision(kind: &'static str) -> DecisionSpec {
    DecisionSpec {
        kind,
        operators: &[],
    }
}

pub static PHP: LanguageSpec = LanguageSpec {
    name: "php",
    extensions: &["php"],
    grammar: Grammar::Builtin(SupportLang::Php),
    definitions: &[
        function("function_definition"),
        function("method_declaration"),
        type_def("class_declaration", TypeKind::Class, &[]),
        type_def("interface_declaration", TypeKind::Interface, &[]),
        type_def("trait_declaration", TypeKind::Trait, &[]),
        type_def(
            "enum_declaration",
            TypeKind::Enum,
            &[MemberSpec {
                kind: "enum_case",
                name: NameSpec::Field("name"),
            }],
        ),
    ],
    containers: &[ContainerSpec {
        kind: "namespace_definition",
        container: ContainerKind::Module,
        name: Some(NameSpec::Field("name")),
        requires_field: Some("body"),
    }],
    module_declarations: &[],
    calls: &[
        CallSpec {
            kind: "function_call_expression",
            callee: Some("function"),
            receiver: None,
        },
        CallSpec {
            kind: "member_call_expression",
            callee: Some("name"),
            receiver: Some("object"),
        },
        CallSpec {
            kind: "nullsafe_member_call_expression",
            callee: Some("name"),
            receiver: Some("object"),
        },
        CallSpec {
            kind: "scoped_call_expression",
            callee: Some("name"),
            receiver: Some("scope"),
        },
    ],
    paths: PathSpec {
        separator: "\\",
        // A `named_type` is one segment: its text, `User` or `\App\User`.
        names: &["name", "named_type"],
        scoped: &[],
        members: &[],
        wrappers: &[],
    },
    self_receivers: &[],
    imports: &[
        "namespace_use_declaration",
        "include_expression",
        "include_once_expression",
        "require_expression",
        "require_once_expression",
    ],
    declarations: &["namespace_definition"],
    bases: &[
        base("class_declaration", BaseRelation::Extends, &["base_clause"]),
        base(
            "class_declaration",
            BaseRelation::Implements,
            &["class_interface_clause"],
        ),
        base(
            "interface_declaration",
            BaseRelation::Extends,
            &["base_clause"],
        ),
        base(
            "enum_declaration",
            BaseRelation::Implements,
            &["class_interface_clause"],
        ),
        // `use Loggable;` inside a class body.
        BaseSpec {
            kind: "class_declaration",
            relation: BaseRelation::Implements,
            fields: &["body", "use_declaration"],
            each_child: true,
            through_kinds: &["name"],
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
        decision("if_statement"),
        decision("else_if_clause"),
        decision("while_statement"),
        decision("do_statement"),
        decision("for_statement"),
        decision("foreach_statement"),
        decision("case_statement"),
        decision("catch_clause"),
        decision("conditional_expression"),
        decision("match_conditional_expression"),
        DecisionSpec {
            kind: "binary_expression",
            operators: &["&&", "||", "and", "or"],
        },
    ],
    docs: &[DocSpec {
        kind: "comment",
        placement: Placement::PrecedingSiblings,
        requires_child: None,
        prefixes: &["/**"],
    }],
    decorators: &[DecoratorSpec {
        kind: "attribute_list",
        placement: Placement::Children,
    }],
    test_markers: &[TestMarker {
        path: "Test",
        arguments: None,
        on_container: false,
    }],
    test_paths: &[
        TestPath::Directory("test"),
        TestPath::Directory("tests"),
        TestPath::StemSuffix("Test"),
    ],
    variables: &[VariableSpec {
        kind: "const_element",
        name: NameSpec::ChildKind(&["name"]),
        constant: true,
        value: None,
    }],
    macros: None,
    // Members are public unless `private` / `protected`; the table cannot
    // say "public unless".
    visibility: Visibility::Always,
    async_markers: &[],
    class_members_in_scope: false,
    self_receiver_texts: &["$this"],
    parameter_names: None,
    body_siblings: &[],
    sibling_calls: None,
    method_owners: &[],
};
