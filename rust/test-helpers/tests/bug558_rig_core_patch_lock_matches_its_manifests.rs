//! BUG-558 — the patched rig-core's own Cargo.lock matches the manifests of
//! its path packages, so the documented standalone test run
//! (`cargo test --manifest-path rust/patches/rig-core/Cargo.toml --locked
//! --offline`, docs/TESTING.md) can start.
//!
//! Feature: spec/features/the-patched-rig-core-lock-matches-its-manifests.feature
//!
//! An exact file check: it needs neither cargo, the network nor the local
//! registry cache. The path packages of the patch are rig-core (the patch's
//! root package, whose dev-dependencies are locked) and codelet-common (a
//! path dependency outside the patch's workspace, whose dev-dependencies are
//! not).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use toml::Value;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const REFRESH: &str =
    "cp rust/Cargo.lock rust/patches/rig-core/Cargo.lock && cargo metadata --offline \
     --format-version 1 --manifest-path rust/patches/rig-core/Cargo.toml > /dev/null";

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// One path package of the patch: its name, manifest and whether its
/// dev-dependencies are locked.
struct PathPackage {
    name: &'static str,
    manifest: Value,
    locks_dev_dependencies: bool,
}

/// The patch lock, the workspace manifest and the patch's path packages, as
/// they are in the repository.
fn repository() -> Result<(Value, Value, Vec<PathPackage>), Box<dyn std::error::Error>> {
    let root = workspace();
    let read = |path: PathBuf| -> Result<Value, Box<dyn std::error::Error>> {
        Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
    };
    let lock = read(root.join("patches/rig-core/Cargo.lock"))?;
    let workspace_manifest = read(root.join("Cargo.toml"))?;
    let packages = vec![
        PathPackage {
            name: "rig-core",
            manifest: read(root.join("patches/rig-core/Cargo.toml"))?,
            locks_dev_dependencies: true,
        },
        PathPackage {
            name: "codelet-common",
            manifest: read(root.join("common/Cargo.toml"))?,
            locks_dev_dependencies: false,
        },
    ];
    Ok((lock, workspace_manifest, packages))
}

/// The lock's `[[package]]` entry for a path package (no `source`).
fn locked<'a>(lock: &'a Value, name: &str) -> Option<&'a Value> {
    lock.get("package")?.as_array()?.iter().find(|entry| {
        entry.get("name").and_then(Value::as_str) == Some(name) && entry.get("source").is_none()
    })
}

/// The version a manifest declares, with `version.workspace = true` resolved
/// from the workspace manifest.
fn declared_version(manifest: &Value, workspace_manifest: &Value) -> Option<String> {
    let version = manifest.get("package")?.get("version")?;
    match version {
        Value::String(version) => Some(version.clone()),
        Value::Table(table) if table.get("workspace").and_then(Value::as_bool) == Some(true) => {
            workspace_manifest
                .get("workspace")?
                .get("package")?
                .get("version")?
                .as_str()
                .map(str::to_string)
        }
        _ => None,
    }
}

/// Version problems of the path packages: each names the package, the
/// locked and the declared version.
fn version_problems(
    lock: &Value,
    workspace_manifest: &Value,
    packages: &[PathPackage],
) -> Vec<String> {
    packages
        .iter()
        .filter_map(|package| {
            let locked_version = locked(lock, package.name)
                .and_then(|entry| entry.get("version"))
                .and_then(Value::as_str);
            let declared = declared_version(&package.manifest, workspace_manifest);
            (locked_version != declared.as_deref()).then(|| {
                format!(
                    "{} is locked at {} but its manifest declares {}",
                    package.name,
                    locked_version.unwrap_or("no version"),
                    declared.as_deref().unwrap_or("no version")
                )
            })
        })
        .collect()
}

/// The package names a dependency table declares: a `package` rename wins,
/// and a workspace-inherited entry takes the workspace entry's rename.
fn declared_names(table: Option<&Value>, workspace_manifest: &Value, names: &mut BTreeSet<String>) {
    let Some(table) = table.and_then(Value::as_table) else {
        return;
    };
    let workspace_dependencies = workspace_manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"));
    for (key, spec) in table {
        let inherited = spec.get("workspace").and_then(Value::as_bool) == Some(true);
        let rename = spec.get("package").or_else(|| {
            inherited
                .then(|| workspace_dependencies.and_then(|deps| deps.get(key)))
                .flatten()
                .and_then(|entry| entry.get("package"))
        });
        names.insert(rename.and_then(Value::as_str).unwrap_or(key).to_string());
    }
}

/// Every dependency name a package's manifest declares in the sections the
/// lock records for it.
fn manifest_dependencies(package: &PathPackage, workspace_manifest: &Value) -> BTreeSet<String> {
    let mut sections = vec!["dependencies", "build-dependencies"];
    if package.locks_dev_dependencies {
        sections.push("dev-dependencies");
    }
    let mut names = BTreeSet::new();
    for section in &sections {
        declared_names(
            package.manifest.get(*section),
            workspace_manifest,
            &mut names,
        );
    }
    if let Some(targets) = package.manifest.get("target").and_then(Value::as_table) {
        for target in targets.values() {
            for section in &sections {
                declared_names(target.get(*section), workspace_manifest, &mut names);
            }
        }
    }
    names
}

/// The dependency names a lock entry lists (`name`, `name version` or
/// `name version (source)`).
fn locked_dependencies(entry: Option<&Value>) -> BTreeSet<String> {
    entry
        .and_then(|entry| entry.get("dependencies"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter_map(|dependency| dependency.split(' ').next())
        .map(str::to_string)
        .collect()
}

/// Dependency problems of the path packages: each names the package and the
/// missing or extra dependency names.
fn dependency_problems(
    lock: &Value,
    workspace_manifest: &Value,
    packages: &[PathPackage],
) -> Vec<String> {
    let mut problems = Vec::new();
    for package in packages {
        let declared = manifest_dependencies(package, workspace_manifest);
        let recorded = locked_dependencies(locked(lock, package.name));
        let missing: Vec<&String> = declared.difference(&recorded).collect();
        let extra: Vec<&String> = recorded.difference(&declared).collect();
        if !missing.is_empty() {
            problems.push(format!(
                "{} lacks the locked dependencies {missing:?}",
                package.name
            ));
        }
        if !extra.is_empty() {
            problems.push(format!(
                "{} has locked dependencies its manifest no longer declares {extra:?}",
                package.name
            ));
        }
    }
    problems
}

/// The failure message: every problem, then the refresh command.
fn report(problems: &[String]) -> String {
    format!(
        "rust/patches/rig-core/Cargo.lock is stale:\n  {}\nrefresh it offline from the workspace lock (docs/TESTING.md):\n  {REFRESH}",
        problems.join("\n  ")
    )
}

#[test]
fn the_patch_lock_records_each_path_package_at_its_manifests_version() -> TestResult {
    // @step Given rust/patches/rig-core/Cargo.lock and the manifests of rig-core and codelet-common
    let (lock, workspace_manifest, packages) = repository()?;
    // @step When the locked versions of the path packages are read
    let problems = version_problems(&lock, &workspace_manifest, &packages);
    // @step Then rig-core is locked at the version its manifest declares
    // @step And codelet-common is locked at the workspace version it inherits
    assert!(problems.is_empty(), "{}", report(&problems));
    Ok(())
}

#[test]
fn the_patch_lock_lists_exactly_the_dependencies_each_path_package_declares() -> TestResult {
    // @step Given rust/patches/rig-core/Cargo.lock and the manifests of rig-core and codelet-common
    let (lock, workspace_manifest, packages) = repository()?;
    // @step When the locked dependency names of the path packages are read
    let problems = dependency_problems(&lock, &workspace_manifest, &packages);
    // @step Then rig-core's locked dependencies are its dependencies, build, target and dev dependencies
    // @step And codelet-common's locked dependencies are its dependencies, build and target dependencies
    assert!(problems.is_empty(), "{}", report(&problems));
    for package in &packages {
        assert!(
            !manifest_dependencies(package, &workspace_manifest).is_empty(),
            "{} declares dependencies",
            package.name
        );
    }
    Ok(())
}

#[test]
fn a_stale_patch_lock_is_reported_with_the_refresh_command() -> TestResult {
    // @step Given a patch lock that records codelet-common 0.10.9 without the dependency fs2 while the workspace version is 0.11.0
    let lock: Value = toml::from_str(
        r#"
        version = 4

        [[package]]
        name = "codelet-common"
        version = "0.10.9"
        dependencies = ["anyhow", "base64 0.22.1"]

        [[package]]
        name = "anyhow"
        version = "1.0.0"
        source = "registry+https://github.com/rust-lang/crates.io-index"
        "#,
    )?;
    let workspace_manifest: Value = toml::from_str(
        r#"
        [workspace.package]
        version = "0.11.0"

        [workspace.dependencies]
        anyhow = "1"
        "#,
    )?;
    let packages = vec![PathPackage {
        name: "codelet-common",
        manifest: toml::from_str(
            r#"
            [package]
            name = "codelet-common"
            version.workspace = true

            [dependencies]
            anyhow.workspace = true
            base64 = "0.22"
            fs2 = "0.4"

            [dev-dependencies]
            tempfile = "3"
            "#,
        )?,
        locks_dev_dependencies: false,
    }];
    // @step When the lock is checked against the manifests
    let mut problems = version_problems(&lock, &workspace_manifest, &packages);
    problems.extend(dependency_problems(&lock, &workspace_manifest, &packages));
    let message = report(&problems);
    // @step Then the check fails naming codelet-common, 0.10.9, 0.11.0 and the missing dependency fs2
    assert_eq!(problems.len(), 2, "{message}");
    assert!(
        message.contains("codelet-common is locked at 0.10.9 but its manifest declares 0.11.0"),
        "{message}"
    );
    assert!(
        message.contains("codelet-common lacks the locked dependencies [\"fs2\"]"),
        "{message}"
    );
    assert!(!message.contains("tempfile"), "{message}");
    // @step And the failure gives the documented refresh command
    assert!(message.contains(REFRESH), "{message}");
    let testing = std::fs::read_to_string(workspace().join("../docs/TESTING.md"))?;
    assert!(
        testing.contains(REFRESH),
        "docs/TESTING.md documents {REFRESH}"
    );
    Ok(())
}
