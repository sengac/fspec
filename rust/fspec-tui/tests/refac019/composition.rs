//! Feature: spec/features/resolve-whole-item-composition-without-false-source-contracts.feature

use super::*;

#[test]
fn source_composition_rejects_ambiguous_or_broken_macro_chains() -> TestResult {
    // @step Given isolated whole-item composition fixtures with valid and invalid chains
    let temporary = codelet_test_helpers::setup_temp_directory()?;
    let root = temporary.path().join("mod.rs");
    let stage = temporary.path().join("stage.rs");
    let terminal = "macro_rules! compose_finish { ($($collected:tt)*) => { pub enum Ordered { $($collected)* Last } }; }\n";
    std::fs::write(&stage, "macro_rules! compose_first { ($($collected:tt)*) => { compose_finish! { $($collected)* First, } }; }\n")?;
    std::fs::write(
        &root,
        format!("// 日本語\ninclude!(\"stage.rs\");\n{terminal}\ncompose_first! {{}}\n"),
    )?;
    // @step When includes and accumulator stages are resolved
    let valid = read_source(&root)?;
    let parsed = syn::parse_file(&valid)?;
    // @step Then valid members retain order and malformed disconnected cyclic or lossy chains fail
    let names = parsed
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Enum(item) => Some(
                item.variants
                    .iter()
                    .map(|variant| variant.ident.to_string())
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .ok_or("composed enum missing")?;
    assert_eq!(names, ["First", "Last"]);
    for broken in [
        "macro_rules! compose_first { ($($collected:tt)*) => { compose_finish! { First, } }; }",
        "macro_rules! compose_first { ($($collected:tt)*) => { compose_finish! { $($collected)* $($collected)* First, } }; }",
        "macro_rules! compose_first { ($($collected:tt)*) => { compose_first! { $($collected)* First, } }; }",
        "macro_rules! compose_first { ($($collected:tt)*) => { compose_finish! { First, $($collected)* } }; }",
    ] {
        std::fs::write(&stage, broken)?;
        assert!(read_source(&root).is_err(), "malformed stage must not resolve: {broken}");
    }
    std::fs::write(&stage, "macro_rules! compose_unused { ($($collected:tt)*) => { pub enum Unused { $($collected)* } }; }")?;
    std::fs::write(
        &root,
        format!("include!(\"stage.rs\");\n{terminal}\ncompose_finish! {{}}"),
    )?;
    assert!(
        read_source(&root).is_err(),
        "unused definitions must not satisfy source checks"
    );
    std::fs::write(&stage, "include!(\"mod.rs\");")?;
    assert!(read_source(&root).is_err(), "cyclic includes must fail");
    let first = "macro_rules! compose_first { ($($collected:tt)*) => { compose_finish! { $($collected)* First, } }; }";
    for attribute in [
        "#[cfg(any())]",
        "#[cfg_attr(all(), cfg(any()))]",
        "#[allow(dead_code)]",
    ] {
        std::fs::write(&stage, first)?;
        std::fs::write(
            &root,
            format!("{attribute}\ninclude!(\"stage.rs\");\n{terminal}\ncompose_first! {{}}"),
        )?;
        assert!(
            read_source(&root).is_err(),
            "include attributes must not be discarded: {attribute}"
        );
        std::fs::write(
            &root,
            format!("include!(\"stage.rs\");\n{terminal}\n{attribute}\ncompose_first! {{}}"),
        )?;
        assert!(
            read_source(&root).is_err(),
            "entry attributes must not be discarded: {attribute}"
        );
        std::fs::write(&stage, format!("{attribute}\n{first}"))?;
        std::fs::write(
            &root,
            format!("include!(\"stage.rs\");\n{terminal}\ncompose_first! {{}}"),
        )?;
        assert!(
            read_source(&root).is_err(),
            "definition attributes must not be discarded: {attribute}"
        );
    }
    std::fs::write(&root, "#[cfg(any())]\npub struct Dormant;\n")?;
    let retained = read_source(&root)?;
    let item = syn::parse_file(&retained)?
        .items
        .into_iter()
        .next()
        .ok_or("ordinary item missing")?;
    assert!(
        matches!(item, syn::Item::Struct(item) if item.attrs.len() == 1 && item.attrs[0].path().is_ident("cfg")),
        "ordinary cfg attributes must survive unchanged"
    );
    Ok(())
}
