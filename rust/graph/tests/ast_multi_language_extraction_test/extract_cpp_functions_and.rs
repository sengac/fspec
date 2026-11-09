// Feature: spec/features/multi-language-ast-extraction.feature
use super::*;

// ============================================================================
// C++ Extractor Tests
// ============================================================================
#[tokio::test]
async fn test_extract_cpp_functions_and_types() {
    // @step Given a C++ file with classes, methods, namespaces, and templates
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let cpp_content = r#"
#include <string>
#include <vector>

namespace engine {

class GameObject {
public:
    void update() {
        tick();
    }
private:
    void tick() {
    }
};

struct Vec3 {
    float x;
    float y;
    float z;
};

enum class Direction {
    North,
    South,
    East,
    West
};

}

int main(int argc, char* argv[]) {
    return 0;
}
"#;
    let file_path = write_test_file(temp_dir.path(), "src/engine.cpp", cpp_content);

    // @step When the C++ extractor parses the file
    let entities = index_file(&file_path, temp_dir.path());

    assert_eq!(count_nodes(&entities, "File"), 1);
    let file_node = find_node(&entities, "File", "src/engine.cpp");
    let Some(GraphEntity::Node { properties, .. }) = file_node else {
        panic!("{} not found", stringify!(file_node));
    };
    assert_eq!(
        properties.get("language").and_then(|v| v.as_str()),
        Some("cpp")
    );

    // @step Then Function nodes should be created for standalone and class methods
    let fn_count = count_nodes(&entities, "Function");
    assert!(
        fn_count >= 1,
        "Should extract at least 1 function, got {fn_count}"
    );

    // @step And Type nodes should be created for classes, structs, enums, and namespaces
    let type_count = count_nodes(&entities, "Type");
    assert!(
        type_count >= 1,
        "Should find at least 1 Type node, got {type_count}"
    );
}

#[tokio::test]
async fn test_cpp_header_heuristic() {
    // @step Given a .h file containing C++ keywords like class or namespace
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let cpp_h_content = r#"
#pragma once
#include <string>

namespace utils {

class Logger {
public:
    void log(std::string msg) {
    }
};

}
"#;
    let file_path = write_test_file(temp_dir.path(), "include/logger.h", cpp_h_content);

    // @step When the pipeline processes the .h file
    let entities = index_file(&file_path, temp_dir.path());

    // @step Then the C++ extractor should be used instead of C
    // Should be detected as C++ due to namespace/class/std:: heuristics
    let file_node = find_node(&entities, "File", "include/logger.h");
    let Some(GraphEntity::Node { properties, .. }) = file_node else {
        panic!("{} not found", stringify!(file_node));
    };
    assert_eq!(
        properties.get("language").and_then(|v| v.as_str()),
        Some("cpp"),
        "Header with C++ features should be detected as cpp"
    );
}

// ============================================================================
// C# Extractor Tests
// ============================================================================
#[tokio::test]
async fn test_extract_csharp_methods_and_types() {
    // @step Given a C# file with public/private methods and class/interface/struct/enum
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let cs_content = r#"
using System;
using System.Threading.Tasks;

namespace MyApp {

    public class UserController {
        public string GetUser(int id) {
            return "user";
        }

        private void LogAccess(string action) {
            Console.WriteLine(action);
        }

        public async Task<string> FetchAsync(int id) {
            return await Task.FromResult("done");
        }
    }

    public interface IUserService {
        string FindUser(int id);
    }

    public enum UserRole {
        Admin,
        User,
        Guest
    }

    public struct Point {
        public int X;
        public int Y;
    }
}
"#;
    let file_path = write_test_file(temp_dir.path(), "src/UserController.cs", cs_content);

    // @step When the C# extractor parses the file
    let entities = index_file(&file_path, temp_dir.path());

    assert_eq!(count_nodes(&entities, "File"), 1);
    let file_node = find_node(&entities, "File", "src/UserController.cs");
    let Some(GraphEntity::Node { properties, .. }) = file_node else {
        panic!("{} not found", stringify!(file_node));
    };
    assert_eq!(
        properties.get("language").and_then(|v| v.as_str()),
        Some("csharp")
    );

    // @step Then Function nodes should be created with access modifier visibility
    let fn_count = count_nodes(&entities, "Function");
    assert!(
        fn_count >= 1,
        "Should extract at least 1 method, got {fn_count}"
    );

    // @step And Type nodes should be created for all C# type declarations
    let type_count = count_nodes(&entities, "Type");
    assert!(
        type_count >= 1,
        "Should find at least 1 Type node, got {type_count}"
    );
}

// ============================================================================
// Ruby Extractor Tests
// ============================================================================
#[tokio::test]
async fn test_extract_ruby_methods_and_types() {
    // @step Given a Ruby file with def, def self., class, and module declarations
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let rb_content = r#"
module Authentication
  class UserAuth
    def initialize(db)
      @db = db
    end

    def authenticate(username, password)
      @db.find(username)
    end

    def self.create(config)
      new(config)
    end

    def _private_method
      true
    end
  end
end
"#;
    let file_path = write_test_file(temp_dir.path(), "lib/auth/user_auth.rb", rb_content);

    // @step When the Ruby extractor parses the file
    let entities = index_file(&file_path, temp_dir.path());

    assert_eq!(count_nodes(&entities, "File"), 1);
    let file_node = find_node(&entities, "File", "lib/auth/user_auth.rb");
    let Some(GraphEntity::Node { properties, .. }) = file_node else {
        panic!("{} not found", stringify!(file_node));
    };
    assert_eq!(
        properties.get("language").and_then(|v| v.as_str()),
        Some("ruby")
    );

    // @step Then Function nodes and Type nodes should be created
    let fn_count = count_nodes(&entities, "Function");
    assert!(
        fn_count >= 1,
        "Should extract at least 1 method, got {fn_count}"
    );

    let type_count = count_nodes(&entities, "Type");
    assert!(
        type_count >= 1,
        "Should find at least 1 Type node, got {type_count}"
    );
}

#[tokio::test]
async fn test_ruby_spec_file_detection() {
    // @step And spec files should have isTest set to true
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let spec_content = "class TestAuth\n  def test_login\n  end\nend\n";
    let file_path = write_test_file(temp_dir.path(), "spec/auth_spec.rb", spec_content);

    let entities = index_file(&file_path, temp_dir.path());

    let file_node = find_node(&entities, "File", "spec/auth_spec.rb");
    let Some(GraphEntity::Node { properties, .. }) = file_node else {
        panic!("{} not found", stringify!(file_node));
    };
    assert_eq!(
        properties.get("isTest").and_then(|v| v.as_bool()),
        Some(true)
    );
}
