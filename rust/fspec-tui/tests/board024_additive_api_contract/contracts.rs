use std::path::{Path, PathBuf};

use codelet_test_helpers::source_composition::read_source;
use quote::ToTokens;
use sha2::{Digest, Sha256};

use super::TestResult;

pub struct Member {
    pub name: String,
    pub hash: String,
    pub signature: String,
    pub body: Option<String>,
    pub non_doc_attributes: bool,
}

pub struct Surface {
    pub header: String,
    pub members: Vec<Member>,
}

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn hash(tokens: impl ToTokens) -> String {
    format!(
        "{:x}",
        Sha256::digest(tokens.to_token_stream().to_string().as_bytes())
    )
}

pub fn surface(path: &str, implementation: bool) -> Result<Surface, Box<dyn std::error::Error>> {
    let name = if path.starts_with("rpc/") {
        "FspecService"
    } else {
        "FspecBackend"
    };
    for item in syn::parse_file(&read_source(&workspace().join(path))?)?.items {
        match item {
            syn::Item::Trait(mut item) if !implementation && item.ident == name => {
                let members = item
                    .items
                    .iter()
                    .map(|member| {
                        let syn::TraitItem::Fn(method) = member else {
                            return Err("unexpected non-method trait member");
                        };
                        Ok(Member {
                            name: method.sig.ident.to_string(),
                            hash: hash(member),
                            signature: method.sig.to_token_stream().to_string(),
                            non_doc_attributes: method
                                .attrs
                                .iter()
                                .any(|attribute| !attribute.path().is_ident("doc")),
                            body: method
                                .default
                                .as_ref()
                                .map(|body| body.to_token_stream().to_string()),
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                item.items.clear();
                return Ok(Surface {
                    header: hash(&item),
                    members,
                });
            }
            syn::Item::Impl(mut item)
                if implementation
                    && item
                        .trait_
                        .as_ref()
                        .is_some_and(|(_, path, _)| path.is_ident(name)) =>
            {
                let members = item
                    .items
                    .iter()
                    .map(|member| {
                        let syn::ImplItem::Fn(method) = member else {
                            return Err("unexpected non-method implementation member");
                        };
                        Ok(Member {
                            name: method.sig.ident.to_string(),
                            hash: hash(member),
                            signature: method.sig.to_token_stream().to_string(),
                            non_doc_attributes: method
                                .attrs
                                .iter()
                                .any(|attribute| !attribute.path().is_ident("doc")),
                            body: Some(method.block.to_token_stream().to_string()),
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                item.items.clear();
                return Ok(Surface {
                    header: hash(&item),
                    members,
                });
            }
            _ => {}
        }
    }
    Err(format!("contract missing in {path}").into())
}

pub fn frozen(label: &str) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(format!("refac019_contracts__refac019_{label}.snap"));
    let snapshot = std::fs::read_to_string(path)?;
    let mut entries = Vec::new();
    for line in snapshot.lines() {
        if let Some((name, digest)) = line.split_once(' ') {
            if digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                entries.push((name.to_owned(), digest.to_owned()));
            }
        }
    }
    if entries.first().is_none_or(|entry| entry.0 != "header") {
        return Err(format!("invalid frozen {label} snapshot").into());
    }
    Ok(entries)
}

pub fn assert_append_only(
    actual: &Surface,
    expected: &[(String, String)],
    signature: &str,
) -> TestResult {
    let (header, members) = expected.split_first().ok_or("frozen contract is empty")?;
    if actual.header != header.1 {
        return Err("original header or attributes changed".into());
    }
    if actual.members.len() != members.len() + 1 {
        return Err(format!(
            "expected {} frozen members plus one details tail, found {} members",
            members.len(),
            actual.members.len()
        )
        .into());
    }
    for (actual, expected) in actual.members.iter().zip(members) {
        if actual.name != expected.0 || actual.hash != expected.1 {
            return Err(format!(
                "frozen ordered member changed: expected {}, found {}",
                expected.0, actual.name
            )
            .into());
        }
    }
    let tail = actual.members.last().ok_or("details tail missing")?;
    if tail.non_doc_attributes {
        return Err("details tail has unexpected behavioral attributes".into());
    }
    let expected: syn::TraitItemFn = syn::parse_str(signature)?;
    if tail.name != "get_work_unit_details"
        || tail.signature != expected.sig.to_token_stream().to_string()
    {
        return Err(format!("unexpected appended tail signature: {}", tail.signature).into());
    }
    Ok(())
}

pub fn assert_backend_error_variant() -> TestResult {
    let source = read_source(&workspace().join("fspec-tui/src/transport/mod.rs"))?;
    for item in syn::parse_file(&source)?.items {
        if let syn::Item::Enum(item) = item {
            if item.ident == "BackendError" {
                let variant = item
                    .variants
                    .iter()
                    .find(|variant| variant.ident == "WorkUnitDetails")
                    .ok_or("BackendError::WorkUnitDetails is missing")?;
                let syn::Fields::Unnamed(fields) = &variant.fields else {
                    return Err("details error must carry its message".into());
                };
                assert_eq!(fields.unnamed.len(), 1);
                assert_eq!(fields.unnamed[0].ty.to_token_stream().to_string(), "String");
                assert!(variant
                    .attrs
                    .iter()
                    .any(|attribute| attribute.path().is_ident("error")));
                return Ok(());
            }
        }
    }
    Err("BackendError enum is missing".into())
}
