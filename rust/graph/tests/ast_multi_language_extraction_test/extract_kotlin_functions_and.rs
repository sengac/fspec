// Feature: spec/features/multi-language-ast-extraction.feature
use super::*;

// ============================================================================
// Kotlin Extractor Tests
// ============================================================================
#[tokio::test]
async fn test_extract_kotlin_functions_and_types() {
    // @step Given a Kotlin file with fun, suspend fun, and class/interface/object/enum
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let kt_content = r#"
package com.example

data class User(val id: Int, val name: String)

interface UserRepository {
    fun findById(id: Int): User?
}

object UserFactory {
    fun create(name: String): User {
        return User(0, name)
    }
}

enum class Role {
    ADMIN,
    USER
}

suspend fun fetchUser(id: Int): User {
    return User(id, "test")
}

fun validateName(name: String): Boolean {
    return name.isNotEmpty()
}
"#;
    let file_path = write_test_file(temp_dir.path(), "src/main/kotlin/User.kt", kt_content);

    // @step When the Kotlin extractor parses the file
    let entities = index_file(&file_path, temp_dir.path());

    assert_eq!(count_nodes(&entities, "File"), 1);
    let file_node = find_node(&entities, "File", "src/main/kotlin/User.kt");
    let Some(GraphEntity::Node { properties, .. }) = file_node else {
        panic!("{} not found", stringify!(file_node));
    };
    assert_eq!(
        properties.get("language").and_then(|v| v.as_str()),
        Some("kotlin")
    );

    // @step Then Function nodes should be created with isAsync for suspend functions
    let fn_count = count_nodes(&entities, "Function");
    assert!(
        fn_count >= 1,
        "Should extract at least 1 function, got {fn_count}"
    );

    // Check suspend fun is marked async
    let fetch_fn = find_node(&entities, "Function", "src/main/kotlin/User.kt::fetchUser");
    let Some(GraphEntity::Node { properties, .. }) = fetch_fn else {
        panic!("{} not found", stringify!(fetch_fn));
    };
    assert_eq!(
        properties.get("isAsync").and_then(|v| v.as_bool()),
        Some(true)
    );

    // @step And Type nodes should be created for all Kotlin type declarations
    let type_count = count_nodes(&entities, "Type");
    assert!(
        type_count >= 1,
        "Should find at least 1 Type node, got {type_count}"
    );
}

// ============================================================================
// Swift Extractor Tests
// ============================================================================
#[tokio::test]
async fn test_extract_swift_functions_and_types() {
    // @step Given a Swift file with func, async func, and class/struct/protocol/enum
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let swift_content = r#"
import Foundation

class NetworkManager {
    func fetchData(url: String) -> Data {
        return Data()
    }

    func sendAsync(data: Data) async -> Bool {
        return true
    }
}

struct Config {
    var apiKey: String
    var timeout: Int
}

protocol Fetchable {
    func fetch(id: Int) -> String
}

enum AppError {
    case networkError
    case parseError
}

func processRequest(url: String, timeout: Int) -> Bool {
    return true
}
"#;
    let file_path = write_test_file(temp_dir.path(), "Sources/App/Network.swift", swift_content);

    // @step When the Swift extractor parses the file
    let entities = index_file(&file_path, temp_dir.path());

    assert_eq!(count_nodes(&entities, "File"), 1);
    let file_node = find_node(&entities, "File", "Sources/App/Network.swift");
    let Some(GraphEntity::Node { properties, .. }) = file_node else {
        panic!("{} not found", stringify!(file_node));
    };
    assert_eq!(
        properties.get("language").and_then(|v| v.as_str()),
        Some("swift")
    );

    // @step Then Function nodes and Type nodes should be created
    let fn_count = count_nodes(&entities, "Function");
    assert!(
        fn_count >= 1,
        "Should extract at least 1 function, got {fn_count}"
    );

    // @step And protocols should have typeKind "trait_kind"
    let type_count = count_nodes(&entities, "Type");
    assert!(
        type_count >= 1,
        "Should find at least 1 Type node, got {type_count}"
    );
    // Verify at least one type has trait_kind (the protocol)
    let has_trait_kind = entities.iter().any(|e| {
        if let GraphEntity::Node {
            node_type,
            properties,
            ..
        } = e
        {
            node_type == "Type"
                && properties.get("typeKind").and_then(|v| v.as_str()) == Some("trait_kind")
        } else {
            false
        }
    });
    assert!(has_trait_kind, "Protocol should have typeKind 'trait_kind'");
}

// ============================================================================
// Scala Extractor Tests
// ============================================================================
#[tokio::test]
async fn test_extract_scala_functions_and_types() {
    // @step Given a Scala file with def, class, trait, object, and case class
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let scala_content = r#"
package com.example

trait Repository {
  def findById(id: Int): Option[String]
}

class UserRepo extends Repository {
  def findById(id: Int): Option[String] = {
    Some("user")
  }

  def save(name: String): Unit = {
    println(name)
  }
}

object UserRepo {
  def apply(): UserRepo = {
    new UserRepo()
  }
}

case class User(id: Int, name: String)
"#;
    let file_path = write_test_file(
        temp_dir.path(),
        "src/main/scala/UserRepo.scala",
        scala_content,
    );

    // @step When the Scala extractor parses the file
    let entities = index_file(&file_path, temp_dir.path());

    assert_eq!(count_nodes(&entities, "File"), 1);
    let file_node = find_node(&entities, "File", "src/main/scala/UserRepo.scala");
    let Some(GraphEntity::Node { properties, .. }) = file_node else {
        panic!("{} not found", stringify!(file_node));
    };
    assert_eq!(
        properties.get("language").and_then(|v| v.as_str()),
        Some("scala")
    );

    // @step Then Function nodes and Type nodes should be created
    let fn_count = count_nodes(&entities, "Function");
    assert!(
        fn_count >= 1,
        "Should extract at least 1 function, got {fn_count}"
    );

    // @step And traits should have typeKind "trait_kind"
    let type_count = count_nodes(&entities, "Type");
    assert!(
        type_count >= 1,
        "Should find at least 1 Type node, got {type_count}"
    );
    // Verify at least one type has trait_kind (the Scala trait)
    let has_trait_kind = entities.iter().any(|e| {
        if let GraphEntity::Node {
            node_type,
            properties,
            ..
        } = e
        {
            node_type == "Type"
                && properties.get("typeKind").and_then(|v| v.as_str()) == Some("trait_kind")
        } else {
            false
        }
    });
    assert!(
        has_trait_kind,
        "Scala trait should have typeKind 'trait_kind'"
    );
}
