//! The declared languages (KGRAPH-074). A language is a `LanguageSpec`
//! value in this list; the file walk, the fact engine and the resolver find
//! everything they need through it.

mod c;
mod cpp;
mod csharp;
mod dart;
mod go;
mod java;
mod javascript;
mod kotlin;
mod php;
mod python;
mod ruby;
mod rust;
mod scala;
mod swift;
mod tsx;
mod typescript;

use super::spec::LanguageSpec;

/// Every declared language.
pub static LANGUAGES: &[&LanguageSpec] = &[
    &rust::RUST,
    &python::PYTHON,
    &typescript::TYPESCRIPT,
    &tsx::TSX,
    &javascript::JAVASCRIPT,
    &go::GO,
    &java::JAVA,
    &kotlin::KOTLIN,
    &scala::SCALA,
    &csharp::CSHARP,
    &c::C,
    &cpp::CPP,
    &ruby::RUBY,
    &php::PHP,
    &swift::SWIFT,
    &dart::DART,
];

/// The language of a file, by its extension. `.h` is declared by the C++
/// table (product owner, 2026-11-07).
pub fn for_path(rel: &str) -> Option<&'static LanguageSpec> {
    let file = rel.rsplit('/').next().unwrap_or(rel);
    let (_, ext) = file.rsplit_once('.')?;
    LANGUAGES
        .iter()
        .copied()
        .find(|spec| spec.extensions.contains(&ext))
}

/// The language with this `name`.
pub fn by_name(name: &str) -> Option<&'static LanguageSpec> {
    LANGUAGES.iter().copied().find(|spec| spec.name == name)
}
