//! Ruby (tree-sitter-ruby 0.23).
//!
//! Calls are `call` nodes read through their `method` and `receiver`
//! fields (a call without a receiver is a plain call). A bare `helper`
//! without arguments or parentheses is an `identifier`, which may be a
//! local variable, so it is not a call fact.
//!
//! `require` and `require_relative` are calls, recorded as declarations
//! (every `call` is snapshotted; the resolver reads the ones that name
//! those methods with one string literal argument). They are not import
//! kinds because an import node is not walked into, and calls hold blocks
//! with definitions (`private def x`, `included do ... end`).

use ast_grep_language::SupportLang;

use crate::ast_pipeline::facts::spec::*;

const COMMENTS: &[&str] = &["comment"];

const CONSTANTS: &[&str] = &["constant", "scope_resolution"];

const fn method(kind: &'static str) -> DefinitionSpec {
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

const fn decision(kind: &'static str) -> DecisionSpec {
    DecisionSpec {
        kind,
        operators: &[],
    }
}

pub static RUBY: LanguageSpec = LanguageSpec {
    name: "ruby",
    extensions: &["rb", "gemspec"],
    grammar: Grammar::Builtin(SupportLang::Ruby),
    definitions: &[
        method("method"),
        method("singleton_method"),
        DefinitionSpec {
            kind: "class",
            entity: Entity::Type(TypeKind::Class),
            name: NameSpec::PathField("name"),
            requires_field: None,
            parameters: None,
            non_parameters: &[],
            members: &[],
            container: Some(ContainerKind::Type),
        },
        DefinitionSpec {
            kind: "module",
            entity: Entity::Type(TypeKind::Interface),
            name: NameSpec::PathField("name"),
            requires_field: None,
            parameters: None,
            non_parameters: &[],
            members: &[],
            container: Some(ContainerKind::Module),
        },
    ],
    containers: &[],
    module_declarations: &[],
    calls: &[CallSpec {
        kind: "call",
        callee: Some("method"),
        receiver: Some("receiver"),
    }],
    paths: PathSpec {
        separator: "::",
        names: &["constant"],
        scoped: &[ScopedSpec {
            kind: "scope_resolution",
            path: "scope",
            name: "name",
        }],
        members: &[],
        wrappers: &[],
    },
    self_receivers: &["self"],
    imports: &[],
    declarations: &["call"],
    bases: &[BaseSpec {
        kind: "class",
        relation: BaseRelation::Extends,
        fields: &["superclass"],
        each_child: true,
        through_kinds: CONSTANTS,
    }],
    type_ref_fields: &[],
    decisions: &[
        decision("if"),
        decision("unless"),
        decision("elsif"),
        decision("if_modifier"),
        decision("unless_modifier"),
        decision("while"),
        decision("until"),
        decision("while_modifier"),
        decision("until_modifier"),
        decision("for"),
        decision("when"),
        decision("rescue"),
        decision("conditional"),
        DecisionSpec {
            kind: "binary",
            operators: &["&&", "||", "and", "or"],
        },
    ],
    docs: &[DocSpec {
        kind: "comment",
        placement: Placement::PrecedingSiblings,
        requires_child: None,
        prefixes: &[],
    }],
    decorators: &[],
    test_markers: &[],
    test_paths: &[
        TestPath::Directory("test"),
        TestPath::Directory("spec"),
        TestPath::StemSuffix("_test"),
        TestPath::StemSuffix("_spec"),
    ],
    variables: &[VariableSpec {
        kind: "assignment",
        // Constant assignments (`X = 1`, `A::X = 1`) define a constant of
        // the lexical module even inside a block; a local (`x = 1`) has no
        // path, since outside a method it may still be a block's local.
        name: NameSpec::PathField("left"),
        constant: true,
        value: Some("right"),
    }],
    macros: None,
    // Visibility is set by `private` / `protected` calls that apply to the
    // methods after them; the table cannot read them.
    visibility: Visibility::Always,
    async_markers: &[],
    class_members_in_scope: true,
    self_receiver_texts: &[],
    parameter_names: None,
    body_siblings: &[],
    sibling_calls: None,
    method_owners: &[],
};
