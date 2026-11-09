// Feature: spec/features/dart-dead-code-false-positive-reduction.feature
use super::*;

// ============================================================================
// Scenario: Exclude Flutter platform directories from dead code for Flutter projects
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_exclude_flutter_platform_dirs_from_dead_code() -> TestResult {
    let (_directory, root) = project(&[])?;
    let project_dir = root.as_path();
    // @step Given a Flutter project with platform directories "ios/", "android/", "macos/", "linux/", and "windows/"
    write_test_file(
        project_dir,
        "ios/Runner/AppDelegate.swift",
        r#"
import UIKit
import Flutter
@UIApplicationMain
@objc class AppDelegate: FlutterAppDelegate {
  override func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }
}
"#,
    );
    write_test_file(
        project_dir,
        "android/app/src/main/kotlin/MainActivity.kt",
        r#"
package com.example.app
import io.flutter.embedding.android.FlutterActivity
class MainActivity: FlutterActivity()
"#,
    );
    write_test_file(
        project_dir,
        "macos/Runner/AppDelegate.swift",
        r#"
import Cocoa
import FlutterMacOS
@main
class AppDelegate: FlutterAppDelegate {}
"#,
    );
    write_test_file(
        project_dir,
        "linux/main.cc",
        r#"
#include "my_application.h"
int main(int argc, char** argv) { return 0; }
"#,
    );

    // @step And the project has a pubspec.yaml with a "flutter" dependency
    write_test_file(
        project_dir,
        "pubspec.yaml",
        r#"
name: my_flutter_app
dependencies:
  flutter:
    sdk: flutter
"#,
    );

    let lib_source = r#"
void main() {
  runApp();
}

void runApp() {
  return;
}
"#;
    write_test_file(project_dir, "lib/main.dart", lib_source);

    let orphan_source = r#"
void orphanFunction() {
  return;
}
"#;
    write_test_file(project_dir, "lib/orphan.dart", orphan_source);
    write_test_file(project_dir, ".gitignore", "build/\n.dart_tool/\n");

    // (ast_index also extracts the pubspec.yaml dependencies)
    index(&root).await?;

    // @step When I run ast_dead_code detection on the indexed project
    let parsed = dead_code(&root, Some("File")).await?;
    let orphan_paths = items(&parsed, "File", "path");
    // The platform files are indexed (and orphans before the Flutter filter).
    let all_files: Vec<String> = nodes(&entities(&root).await?, "File")
        .into_iter()
        .map(|(slug, _)| slug)
        .collect();
    for platform_file in [
        "ios/Runner/AppDelegate.swift",
        "android/app/src/main/kotlin/MainActivity.kt",
        "macos/Runner/AppDelegate.swift",
        "linux/main.cc",
    ] {
        assert!(
            all_files.contains(&platform_file.to_string()),
            "{platform_file} is indexed: {all_files:?}"
        );
    }

    // @step Then no files from platform directories should appear in the orphan files results
    let platform_orphans: Vec<&String> = orphan_paths
        .iter()
        .filter(|p| {
            p.starts_with("ios/")
                || p.starts_with("android/")
                || p.starts_with("macos/")
                || p.starts_with("linux/")
                || p.starts_with("windows/")
        })
        .collect();
    assert!(
        platform_orphans.is_empty(),
        "Platform directory files should NOT appear in orphan results for Flutter projects. Got: {:?}",
        platform_orphans
    );

    // @step And non-platform orphan files should still be detected
    assert!(
        platform_orphans.is_empty(),
        "Platform directory files should NOT appear in orphan results for Flutter projects. Got: {:?}",
        platform_orphans
    );

    // @step And non-platform orphan files should still be detected
    let non_platform_orphans: Vec<&String> = orphan_paths
        .iter()
        .filter(|p| {
            !p.starts_with("ios/")
                && !p.starts_with("android/")
                && !p.starts_with("macos/")
                && !p.starts_with("linux/")
                && !p.starts_with("windows/")
        })
        .collect();

    // lib/orphan.dart should be detected as orphan (it's not imported by anyone)
    assert!(
        non_platform_orphans
            .iter()
            .any(|p| p.as_str() == "lib/orphan.dart"),
        "Non-platform orphan files should still be detected. Got: {:?}",
        orphan_paths
    );
    Ok(())
}

// ============================================================================
// Scenario: Do not flag main.dart entry point as orphan file
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_do_not_flag_main_dart_as_orphan() -> TestResult {
    let (_directory, root) = project(&[])?;
    let project_dir = root.as_path();
    // @step Given a Dart project with "lib/main.dart" containing a top-level main() function
    let main_source = r#"
void main() {
  runApp();
}

void runApp() {
  return;
}
"#;
    write_test_file(project_dir, "lib/main.dart", main_source);

    // @step And no other file imports main.dart
    let utils_source = r#"
void utilFunction() {
  return;
}
"#;
    write_test_file(project_dir, "lib/utils.dart", utils_source);
    write_test_file(project_dir, ".gitignore", "build/\n.dart_tool/\n");

    index(&root).await?;

    // @step When I run ast_dead_code detection on the indexed project
    let parsed = dead_code(&root, Some("File")).await?;
    let orphan_paths = items(&parsed, "File", "path");

    // @step Then "lib/main.dart" should not appear in the orphan files results
    assert!(
        !orphan_paths.iter().any(|p| p.ends_with("main.dart")),
        "main.dart entry point should NOT appear as orphan. Got: {:?}",
        orphan_paths
    );

    // @step And other files that are genuinely orphaned should still appear
    assert!(
        orphan_paths.contains(&"lib/utils.dart".to_string()),
        "Genuinely orphan files (utils.dart) should still appear. Got: {:?}",
        orphan_paths
    );
    Ok(())
}
