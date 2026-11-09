//! Language tables (KGRAPH-074): what the fact engine collects for a
//! language is declared here as data. The engine in `extract.rs` has no
//! per-language code path; a new language is a new `LanguageSpec` value.

use ast_grep_language::SupportLang;

/// A declared language.
pub struct LanguageSpec {
    /// The `language` property written on File, Function and Type nodes.
    pub name: &'static str,
    /// File extensions (without the dot) parsed with this table.
    pub extensions: &'static [&'static str],
    /// The tree-sitter grammar.
    pub grammar: Grammar,
    /// Node kinds that are definitions.
    pub definitions: &'static [DefinitionSpec],
    /// Node kinds that add a segment to the container chain of the
    /// definitions inside them without being definitions themselves.
    pub containers: &'static [ContainerSpec],
    /// Node kinds that declare a module stored in another file (Rust `mod x;`).
    pub module_declarations: &'static [ModuleDeclarationSpec],
    /// Call node kinds.
    pub calls: &'static [CallSpec],
    /// How a callee, base or type expression becomes a path.
    pub paths: PathSpec,
    /// Receiver node kinds that denote the enclosing type's instance.
    pub self_receivers: &'static [&'static str],
    /// Import node kinds, recorded as written (a syntax snapshot).
    pub imports: &'static [&'static str],
    /// Node kinds recorded as written for the resolver (package and
    /// namespace declarations).
    pub declarations: &'static [&'static str],
    /// Base and implemented types.
    pub bases: &'static [BaseSpec],
    /// Fields of a function definition whose type names are type references.
    pub type_ref_fields: &'static [TypeRefField],
    /// Decision nodes for cyclomatic complexity and conditional calls.
    pub decisions: &'static [DecisionSpec],
    /// Doc comments of a definition.
    pub docs: &'static [DocSpec],
    /// Decorators and attributes of a definition.
    pub decorators: &'static [DecoratorSpec],
    /// Decorators that make the definition, or every definition inside the
    /// decorated container, a test.
    pub test_markers: &'static [TestMarker],
    /// File path conventions that make every definition of a file a test.
    pub test_paths: &'static [TestPath],
    /// Module-level and class-level variables.
    pub variables: &'static [VariableSpec],
    /// Macro invocations whose token trees hold unparsed calls.
    pub macros: Option<MacroSpec>,
    /// How a definition is known to be public.
    pub visibility: Visibility,
    /// Tokens or node kinds that mark a function async.
    pub async_markers: &'static [Marker],
    /// Whether a plain name inside a type's body can name the type's own
    /// methods (Java, C#, Kotlin, C++), or only names of enclosing
    /// functions and the file (Python, Rust).
    pub class_members_in_scope: bool,
    /// Definitions owned by a type named in one of their fields.
    pub method_owners: &'static [MethodOwnerSpec],
    /// Receiver texts that denote the enclosing type's instance when its
    /// node kind is shared with ordinary names (PHP `$this`).
    pub self_receiver_texts: &'static [&'static str],
    /// How a parameter's name is read; `None`: each parameter's text.
    pub parameter_names: Option<ParameterNames>,
    /// (definition kind, body kind): the definition's body is its next
    /// named sibling of that kind (Dart's signature and body are siblings).
    pub body_siblings: &'static [(&'static str, &'static str)],
    /// Calls written as a name followed by selector siblings (Dart).
    pub sibling_calls: Option<SiblingCallSpec>,
}

/// A call that is not a node: a name (or self receiver) followed by
/// member selectors and a selector holding the argument list.
pub struct SiblingCallSpec {
    /// The selector node kind.
    pub selector: &'static str,
    /// Kinds inside a selector that select a member (`.name`, `?.name`).
    pub member_selectors: &'static [&'static str],
    /// The kind inside a selector that holds the arguments.
    pub call_part: &'static str,
}

/// The names of a definition's parameters, without their types.
pub struct ParameterNames {
    /// Fields of a parameter holding its name (every occurrence counts, Go
    /// declares `a, b int` as two names); without them, the parameter.
    pub fields: &'static [&'static str],
    /// The name is that node's text when its kind is one of these, else
    /// the text of its first descendant of these kinds.
    pub kinds: &'static [&'static str],
    /// Parameter kinds that are the receiver, not a parameter (`&self`).
    pub receiver_kinds: &'static [&'static str],
    /// The language binds a method's first parameter to the instance
    /// (Python): for a function defined directly in a type's body, the
    /// first parameter is the receiver.
    pub first_is_receiver_in_type: bool,
}

/// A tree-sitter grammar: one of ast-grep's languages, or Dart (whose
/// grammar codelet-tools declares).
#[derive(Clone, Copy, Debug)]
pub enum Grammar {
    Builtin(SupportLang),
    Dart,
}

/// What a definition node becomes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
pub enum Entity {
    Function,
    Type(TypeKind),
}

/// The schema's `Type.typeKind` values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
pub enum TypeKind {
    Struct,
    Class,
    Interface,
    Enum,
    Trait,
    TypeAlias,
    Extension,
}

impl TypeKind {
    pub fn schema_name(self) -> &'static str {
        match self {
            TypeKind::Struct => "struct_kind",
            TypeKind::Class => "class",
            TypeKind::Interface => "interface",
            TypeKind::Enum => "enum_kind",
            TypeKind::Trait => "trait_kind",
            TypeKind::TypeAlias => "type_alias",
            TypeKind::Extension => "extension",
        }
    }
}

/// How the name of a node is found.
#[derive(Clone, Copy, Debug)]
pub enum NameSpec {
    /// The text of a field.
    Field(&'static str),
    /// Follow `field` from the node while the reached node's kind is not in
    /// `until`; the name is the text of the first node whose kind is. When
    /// `through` is not empty, the chain must pass a node of one of those
    /// kinds, or the node has no name (a C++ field that is not a method).
    Descend {
        field: &'static str,
        until: &'static [&'static str],
        through: &'static [&'static str],
    },
    /// The first named child whose kind is one of these.
    ChildKind(&'static [&'static str]),
    /// The path of a field (see `PathSpec`), whose last segment is the name.
    PathField(&'static str),
    /// The text of the last occurrence of a repeated field (Dart's named
    /// constructor `User.fromJson` has two `name` fields).
    LastField(&'static str),
}

pub struct DefinitionSpec {
    pub kind: &'static str,
    pub entity: Entity,
    pub name: NameSpec,
    /// Only when the node has this field (Rust `mod_item` with a body).
    pub requires_field: Option<&'static str>,
    /// The field holding the parameter list.
    pub parameters: Option<&'static str>,
    /// Named children of the parameter list that are not parameters.
    pub non_parameters: &'static [&'static str],
    /// The node kinds whose name is a member of a Type (enum variants).
    pub members: &'static [MemberSpec],
    /// When set, the definition is a container for the definitions inside it.
    pub container: Option<ContainerKind>,
}

pub struct MemberSpec {
    pub kind: &'static str,
    pub name: NameSpec,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerKind {
    /// A module or namespace block.
    Module,
    /// A type body (class, trait, interface): functions inside are methods.
    Type,
    /// An implementation block for a type named by a field (Rust `impl`).
    Impl {
        self_type: &'static str,
        trait_field: Option<&'static str>,
    },
    /// A function body: definitions inside are nested definitions.
    Function,
}

pub struct ContainerSpec {
    pub kind: &'static str,
    pub container: ContainerKind,
    /// The container's segment in the chain. For `Impl`, the self type's
    /// last path segment is used when this is `None`.
    pub name: Option<NameSpec>,
    pub requires_field: Option<&'static str>,
}

pub struct ModuleDeclarationSpec {
    pub kind: &'static str,
    pub name: NameSpec,
    /// Only when the node lacks this field (Rust `mod x;` has no body).
    pub without_field: &'static str,
    /// The decorator path whose string value overrides the file path, and
    /// the node kind of that string's content.
    pub path_attribute: Option<(&'static str, &'static str)>,
}

pub struct CallSpec {
    pub kind: &'static str,
    /// The field holding the callee expression; with `receiver`, the field
    /// holding the method name. `None`: the call's first named child.
    pub callee: Option<&'static str>,
    /// The field holding the receiver when the call node itself holds the
    /// receiver and the method name (Java `method_invocation`, Ruby `call`).
    /// A call without that field is a plain call of the name.
    pub receiver: Option<&'static str>,
}

/// A definition kind whose type is named in one of its fields rather than
/// by an enclosing node (Go methods: the receiver's type). The definition
/// sits in an implementation container of that type.
pub struct MethodOwnerSpec {
    pub kind: &'static str,
    pub field: &'static str,
    /// The first node of these kinds inside the field names the type.
    pub type_kinds: &'static [&'static str],
}

/// Turning an expression into a path of names.
pub struct PathSpec {
    /// The separator a path is written with (`::`, `.`).
    pub separator: &'static str,
    /// Kinds whose text is one segment.
    pub names: &'static [&'static str],
    /// Kinds made of a path and a final name.
    pub scoped: &'static [ScopedSpec],
    /// Kinds made of a receiver expression and a member name (method calls).
    pub members: &'static [ScopedSpec],
    /// Kinds whose path is the path of one of their fields (generic wrappers).
    pub wrappers: &'static [(&'static str, &'static str)],
}

pub struct ScopedSpec {
    pub kind: &'static str,
    /// Field holding the path or receiver; absent for a leading `::name`.
    pub path: &'static str,
    pub name: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum BaseRelation {
    Extends,
    Implements,
}

/// Where the bases of a definition or container are.
pub struct BaseSpec {
    /// The definition or container kind that has the bases.
    pub kind: &'static str,
    pub relation: BaseRelation,
    /// Field path from the node to the base types. Every listed field is
    /// followed in order; at the last step every named child of the reached
    /// node (or the node itself when `each_child` is false) is one base.
    pub fields: &'static [&'static str],
    pub each_child: bool,
    /// When set, the node at the end of `fields` must be of one of these
    /// kinds, otherwise it is searched among the node's named children.
    pub through_kinds: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
pub enum RefKind {
    Param,
    ReturnType,
    /// A call whose callee is a type: the function constructs it.
    Constructor,
}

impl RefKind {
    pub fn schema_name(self) -> &'static str {
        match self {
            RefKind::Param => "param",
            RefKind::ReturnType => "return_type",
            RefKind::Constructor => "constructor",
        }
    }
}

pub struct TypeRefField {
    /// A field of the function, or (grammars without fields) the kind of
    /// its named children to search.
    pub field: &'static str,
    pub ref_kind: RefKind,
    /// Kinds inside the field that are type names.
    pub type_kinds: &'static [&'static str],
    /// When set, only this field of each named child of `field` is searched
    /// (C# parameters: the `type` of each `parameter`, not its name).
    pub item_field: Option<&'static str>,
}

pub struct DecisionSpec {
    pub kind: &'static str,
    /// Only when the node's `operator` field text is one of these.
    pub operators: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// Consecutive named siblings right before the definition (Rust `///`,
    /// `#[...]`, Go and C comments). A definition inside a declared wrapper
    /// takes the wrapper's preceding siblings.
    PrecedingSiblings,
    /// Named children of the definition.
    Children,
    /// Named children of the definition's parent when the parent is of the
    /// given kind (Python `decorated_definition`).
    ParentChildren(&'static str),
    /// Children of a named child of the definition (Java `modifiers`).
    ChildChildren(&'static str),
    /// The first statement of the definition's body field when it is of the
    /// given kind (Python docstrings).
    BodyFirst {
        body: &'static str,
        statement: &'static str,
    },
}

pub struct DocSpec {
    pub kind: &'static str,
    pub placement: Placement,
    /// Only nodes that have a child of this kind (Rust `doc_comment`).
    pub requires_child: Option<&'static str>,
    /// Only comments whose text starts with one of these.
    pub prefixes: &'static [&'static str],
}

pub struct DecoratorSpec {
    pub kind: &'static str,
    pub placement: Placement,
}

/// A decorator whose path, and optional argument text, is exactly this.
pub struct TestMarker {
    /// The decorator's path as written, without the decorator syntax
    /// (`test`, `tokio::test`, `cfg`).
    pub path: &'static str,
    /// The argument text, e.g. `(test)` for `#[cfg(test)]`.
    pub arguments: Option<&'static str>,
    /// Whether the marker on a container marks every definition inside it.
    pub on_container: bool,
}

/// A path convention of test files.
pub enum TestPath {
    /// Any directory component equals this.
    Directory(&'static str),
    /// The file name starts with this.
    FilePrefix(&'static str),
    /// The file name without its extension ends with this.
    StemSuffix(&'static str),
    /// A dot-separated part of the file name other than the first and the
    /// last equals this (`a.test.ts` has the part `test`).
    NamePart(&'static str),
}

pub struct VariableSpec {
    pub kind: &'static str,
    pub name: NameSpec,
    pub constant: bool,
    /// The field holding the value.
    pub value: Option<&'static str>,
}

pub struct MacroSpec {
    pub invocation: &'static str,
    pub token_tree: &'static str,
    /// Token kinds that name a call when followed by a token tree that
    /// starts with `(`.
    pub names: &'static [&'static str],
}

/// How a definition is public.
pub enum Visibility {
    /// It has a named child of one of these kinds.
    ChildKind(&'static [&'static str]),
    /// One of these tokens is a child of the definition or of a child of
    /// the given modifier kind.
    Modifier {
        modifiers: &'static str,
        tokens: &'static [&'static str],
    },
    /// The parent is of one of these kinds (TypeScript `export_statement`).
    ParentKind(&'static [&'static str]),
    /// The language rule is that names starting with this are private.
    PrivatePrefix(&'static str),
    /// The language rule is that names starting with an upper-case letter
    /// are exported (Go).
    UpperInitial,
    /// Public unless one of these tokens is inside a child of the given
    /// modifier kind (C `static`).
    Unless {
        modifiers: &'static str,
        tokens: &'static [&'static str],
    },
    /// Every definition is public.
    Always,
}

/// A token or node kind among the children of a definition, or inside one
/// of its children of the given kind.
pub struct Marker {
    pub token: &'static str,
    pub inside: Option<&'static str>,
}
