use std::path::{Path, PathBuf};
use std::process::Command;

use quote::ToTokens;
use serde_json::Value;

use crate::{source, TestResult};

const HARNESS: &str = include_str!("tree_harness.txt");

fn dependency(directory: &Path, name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut candidates = std::fs::read_dir(directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.file_name().and_then(|name| name.to_str())
            .is_some_and(|file| file.starts_with(&format!("lib{name}-")) && file.ends_with(".rlib")))
        .map(|path| Ok((std::fs::metadata(&path)?.modified()?, path)))
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    candidates.sort_by(|first, second| second.0.cmp(&first.0));
    Ok(candidates.into_iter().next().ok_or("compiled harness dependency unavailable")?.1)
}

fn compiled_tree(text: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let temporary = codelet_test_helpers::setup_temp_directory()?;
    let parser = temporary.path().join("parser.rs");
    let executable = temporary.path().join("parser");
    let declarations = source::declarations(text)?;
    assert_eq!(declarations.len(), 2, "both actual CLI items must compile");
    let mut generated = String::from("use clap::{Parser, Subcommand, CommandFactory};\nuse std::path::PathBuf;\n");
    for declaration in declarations {
        generated.push_str(&declaration.to_token_stream().to_string());
        generated.push('\n');
    }
    generated.push_str(HARNESS);
    std::fs::write(&parser, generated)?;
    let test_executable = std::env::current_exe()?;
    let dependencies = test_executable.parent().ok_or("test executable has no dependency directory")?;
    let output = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .arg("--edition=2021")
        .arg("--crate-name=refac020_parser_contract")
        .arg("-Cdebuginfo=0")
        .arg("-L").arg(format!("dependency={}", dependencies.display()))
        .arg("--extern").arg(format!("clap={}", dependency(dependencies, "clap")?.display()))
        .arg("--extern").arg(format!("serde_json={}", dependency(dependencies, "serde_json")?.display()))
        .arg(&parser).arg("-o").arg(&executable)
        .env("CARGO_PKG_VERSION", env!("CARGO_PKG_VERSION"))
        .output()?;
    assert!(output.status.success(), "parser harness must compile before behavior assertions: {}", String::from_utf8_lossy(&output.stderr));
    let output = Command::new(executable).output()?;
    assert!(output.status.success(), "parser harness failed: {}", String::from_utf8_lossy(&output.stderr));
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn strip_additions(command: &mut Value) {
    let name = command["name"].as_str().unwrap_or_default().to_owned();
    if let Some(arguments) = command["arguments"].as_array_mut() {
        arguments.retain(|argument| !source::additions(&name)
            .contains(&argument["id"].as_str().unwrap_or_default()));
    }
    if let Some(children) = command["subcommands"].as_array_mut() {
        for child in children {
            strip_additions(child);
        }
    }
}

#[test]
fn compiled_clap_registration_retains_its_complete_ordered_tree() -> TestResult {
    // @step Given a compiled parser harness using frozen original declarations
    let expected = compiled_tree(&source::original()?)?;
    if let Some(path) = std::env::var_os("REFAC020_FREEZE_TREE") {
        std::fs::write(path, serde_json::to_vec_pretty(&expected)?)?;
    }
    // @step When the actual CLI declarations are compiled and their command argument tree is traversed
    let mut actual = compiled_tree(&source::actual()?)?;
    strip_additions(&mut actual);
    // @step Then ordered commands aliases defaults actions arity requiredness and scope match the frozen tree except approved additions
    assert_eq!(actual, expected);
    Ok(())
}
