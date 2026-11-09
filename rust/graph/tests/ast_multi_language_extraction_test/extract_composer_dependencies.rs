// Feature: spec/features/multi-language-ast-extraction.feature
use super::*;

#[tokio::test]
async fn test_extract_composer_dependencies() {
    // @step Given a project with composer.json require and require-dev sections
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let composer_content = r#"{
    "require": {
        "laravel/framework": "^10.0",
        "guzzlehttp/guzzle": "^7.0"
    },
    "require-dev": {
        "phpunit/phpunit": "^10.0"
    }
}"#;
    write_test_file(temp_dir.path(), "composer.json", composer_content);

    // @step When the composer dependency extractor runs
    let entities =
        codelet_graph::ast_pipeline::composer_dep_extractor::extract_composer_dependencies(
            temp_dir.path(),
        )
        .into_entities()
        .expect("Composer dep extraction should succeed");

    // @step Then Dependency nodes should be created with source "composer"
    let dep_count = count_nodes(&entities, "Dependency");
    assert!(
        dep_count >= 3,
        "Should extract at least 3 dependencies, got {dep_count}"
    );
    assert!(
        has_dependency_with_source(&entities, "composer"),
        "Dependency source should be 'composer'"
    );
}

#[tokio::test]
async fn test_extract_gemfile_dependencies() {
    // @step Given a project with Gemfile listing gems with version constraints
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let gemfile_content = r#"source 'https://rubygems.org'

gem 'rails', '~> 7.0'
gem 'pg'

group :development, :test do
  gem 'rspec-rails', '~> 6.0'
end
"#;
    write_test_file(temp_dir.path(), "Gemfile", gemfile_content);

    // @step When the Gemfile dependency extractor runs
    let entities =
        codelet_graph::ast_pipeline::gemfile_dep_extractor::extract_gemfile_dependencies(
            temp_dir.path(),
        )
        .into_entities()
        .expect("Gemfile dep extraction should succeed");

    // @step Then Dependency nodes should be created with source "gem"
    let dep_count = count_nodes(&entities, "Dependency");
    assert!(
        dep_count >= 2,
        "Should extract at least 2 dependencies, got {dep_count}"
    );
    assert!(
        has_dependency_with_source(&entities, "gem"),
        "Dependency source should be 'gem'"
    );
}

#[tokio::test]
async fn test_extract_swift_dependencies() {
    // @step Given a project with Package.swift listing .package dependencies
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let pkg_content = r#"
// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "MyApp",
    dependencies: [
        .package(url: "https://github.com/vapor/vapor", from: "4.0.0"),
        .package(url: "https://github.com/apple/swift-log.git", exact: "1.5.0"),
    ]
)
"#;
    write_test_file(temp_dir.path(), "Package.swift", pkg_content);

    // @step When the Swift dependency extractor runs
    let entities = codelet_graph::ast_pipeline::swift_dep_extractor::extract_swift_dependencies(
        temp_dir.path(),
    )
    .into_entities()
    .expect("Swift dep extraction should succeed");

    // @step Then Dependency nodes should be created with source "spm"
    let dep_count = count_nodes(&entities, "Dependency");
    assert!(
        dep_count >= 2,
        "Should extract at least 2 dependencies, got {dep_count}"
    );
    assert!(
        has_dependency_with_source(&entities, "spm"),
        "Dependency source should be 'spm'"
    );
}

#[tokio::test]
async fn test_extract_csproj_dependencies() {
    // @step Given a project with .csproj PackageReference elements
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let csproj_content = r#"<Project Sdk="Microsoft.NET.Sdk">
  <ItemGroup>
    <PackageReference Include="Newtonsoft.Json" Version="13.0.1" />
    <PackageReference Include="Microsoft.Extensions.Logging" Version="7.0.0" />
  </ItemGroup>
</Project>
"#;
    write_test_file(temp_dir.path(), "MyApp.csproj", csproj_content);

    // @step When the .csproj dependency extractor runs
    let entities = codelet_graph::ast_pipeline::csproj_dep_extractor::extract_csproj_dependencies(
        temp_dir.path(),
    )
    .into_entities()
    .expect("Csproj dep extraction should succeed");

    // @step Then Dependency nodes should be created with source "nuget"
    let dep_count = count_nodes(&entities, "Dependency");
    assert!(
        dep_count >= 2,
        "Should extract at least 2 dependencies, got {dep_count}"
    );
    assert!(
        has_dependency_with_source(&entities, "nuget"),
        "Dependency source should be 'nuget'"
    );
}

#[tokio::test]
async fn test_extract_sbt_dependencies() {
    // @step Given a project with build.sbt listing libraryDependencies
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let sbt_content = r#"
name := "myproject"
version := "0.1.0"

libraryDependencies ++= Seq(
  "com.typesafe.akka" %% "akka-actor" % "2.8.0",
  "org.scalatest" %% "scalatest" % "3.2.0" % Test
)
"#;
    write_test_file(temp_dir.path(), "build.sbt", sbt_content);

    // @step When the sbt dependency extractor runs
    let entities =
        codelet_graph::ast_pipeline::sbt_dep_extractor::extract_sbt_dependencies(temp_dir.path())
            .into_entities()
            .expect("SBT dep extraction should succeed");

    // @step Then Dependency nodes should be created with source "sbt"
    let dep_count = count_nodes(&entities, "Dependency");
    assert!(
        dep_count >= 2,
        "Should extract at least 2 dependencies, got {dep_count}"
    );
    assert!(
        has_dependency_with_source(&entities, "sbt"),
        "Dependency source should be 'sbt'"
    );
}

// ============================================================================
// Missing dep file returns Ok(vec![]) — never errors
// ============================================================================
#[tokio::test]
async fn test_missing_dep_files_return_empty() {
    // @step Given a project without any dependency manifest files
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let root = temp_dir.path();

    // @step When all dependency extractors run
    // @step Then each should return an empty list without errors
    assert!(
        codelet_graph::ast_pipeline::pip_dep_extractor::extract_python_dependencies(root)
            .into_entities()
            .unwrap()
            .is_empty()
    );
    assert!(
        codelet_graph::ast_pipeline::gomod_dep_extractor::extract_go_dependencies(root)
            .into_entities()
            .unwrap()
            .is_empty()
    );
    assert!(
        codelet_graph::ast_pipeline::java_dep_extractor::extract_java_dependencies(root)
            .into_entities()
            .unwrap()
            .is_empty()
    );
    assert!(
        codelet_graph::ast_pipeline::composer_dep_extractor::extract_composer_dependencies(root)
            .into_entities()
            .unwrap()
            .is_empty()
    );
    assert!(
        codelet_graph::ast_pipeline::gemfile_dep_extractor::extract_gemfile_dependencies(root)
            .into_entities()
            .unwrap()
            .is_empty()
    );
    assert!(
        codelet_graph::ast_pipeline::swift_dep_extractor::extract_swift_dependencies(root)
            .into_entities()
            .unwrap()
            .is_empty()
    );
    assert!(
        codelet_graph::ast_pipeline::csproj_dep_extractor::extract_csproj_dependencies(root)
            .into_entities()
            .unwrap()
            .is_empty()
    );
    assert!(
        codelet_graph::ast_pipeline::sbt_dep_extractor::extract_sbt_dependencies(root)
            .into_entities()
            .unwrap()
            .is_empty()
    );
}
