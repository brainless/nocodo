#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::pin::Pin;
    use std::future::Future;

    // Mock BashExecutor for testing
    struct MockBashExecutor;

    impl BashExecutorTrait for MockBashExecutor {
        fn execute_with_cwd(
            &self,
            _command: &str,
            _working_dir: &std::path::PathBuf,
            _timeout_secs: Option<u64>,
        ) -> Pin<Box<dyn Future<Output = Result<BashExecutionResult>> + Send>> {
            Box::pin(async {
                Ok(BashExecutionResult {
                    stdout: "mock output".to_string(),
                    stderr: "".to_string(),
                    exit_code: 0,
                    timed_out: false,
                })
            })
        }
    }

    #[test]
    fn test_glob_to_regex() {
        // Test simple wildcard patterns (no start anchor for patterns starting with *)
        assert_eq!(ToolExecutor::glob_to_regex("*.rs"), r"[^/]*\.rs$");
        assert_eq!(ToolExecutor::glob_to_regex("*.py"), r"[^/]*\.py$");
        assert_eq!(ToolExecutor::glob_to_regex("*.txt"), r"[^/]*\.txt$");

        // Test patterns with prefix (start anchor for patterns not starting with *)
        assert_eq!(ToolExecutor::glob_to_regex("test*.rs"), r"^test[^/]*\.rs$");
        assert_eq!(ToolExecutor::glob_to_regex("mod*.py"), r"^mod[^/]*\.py$");

        // Test double wildcard for directory depth
        assert_eq!(ToolExecutor::glob_to_regex("**/*.rs"), r".*/[^/]*\.rs$");
        assert_eq!(
            ToolExecutor::glob_to_regex("src/**/*.rs"),
            r"^src/.*/[^/]*\.rs$"
        );

        // Test question mark (single character)
        assert_eq!(ToolExecutor::glob_to_regex("test?.rs"), r"^test[^/]\.rs$");

        // Test escaping of regex special characters
        assert_eq!(
            ToolExecutor::glob_to_regex("test+file.rs"),
            r"^test\+file\.rs$"
        );
        assert_eq!(
            ToolExecutor::glob_to_regex("test[1].rs"),
            r"^test\[1\]\.rs$"
        );

        // Test patterns without wildcards
        assert_eq!(ToolExecutor::glob_to_regex("test.rs"), r"^test\.rs$");
    }

    #[test]
    fn test_glob_to_regex_matching() {
        use regex::Regex;

        // Test *.rs matches - should match filenames ending in .rs
        // Note: The pattern is applied to relative file paths, so src/main.rs will match
        let pattern = ToolExecutor::glob_to_regex("*.rs");
        let regex = Regex::new(&pattern).unwrap();
        assert!(regex.is_match("main.rs"));
        assert!(regex.is_match("lib.rs"));
        assert!(!regex.is_match("main.py"));

        // In grep_search, this is matched against relative_path which includes directories
        // For file-only matching, the grep tool filters by filename, not full path
        // So *.rs pattern will be matched against just "main.rs" not "src/main.rs"

        // Test *.py matches
        let pattern = ToolExecutor::glob_to_regex("*.py");
        let regex = Regex::new(&pattern).unwrap();
        assert!(regex.is_match("test.py"));
        assert!(regex.is_match("module.py"));
        assert!(!regex.is_match("test.rs"));

        // Test **/*.rs matches (nested paths) - this should match full paths
        let pattern = ToolExecutor::glob_to_regex("**/*.rs");
        let regex = Regex::new(&pattern).unwrap();
        assert!(regex.is_match("src/main.rs"));
        assert!(regex.is_match("src/lib/mod.rs"));
        assert!(regex.is_match("tests/integration.rs"));
        assert!(!regex.is_match("main.py"));

        // Test that patterns are properly anchored
        let pattern = ToolExecutor::glob_to_regex("test.rs");
        let regex = Regex::new(&pattern).unwrap();
        assert!(regex.is_match("test.rs"));
        assert!(!regex.is_match("test.rs.bak"));
        assert!(!regex.is_match("my_test.rs"));
    }

    #[tokio::test]
    async fn test_tool_executor_list_files() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::new(temp_dir.path().to_path_buf());

        // Create test files
        fs::write(temp_dir.path().join("test.txt"), "Hello World").unwrap();
        fs::create_dir_all(temp_dir.path().join("subdir")).unwrap();
        fs::write(temp_dir.path().join("subdir/nested.txt"), "Nested").unwrap();

        let request = ListFilesRequest {
            path: ".".to_string(),
            recursive: Some(true),
            include_hidden: Some(false),
            max_files: None,
        };

        let response = executor
            .execute(manager_models::ToolRequest::ListFiles(request))
            .await
            .unwrap();

        match response {
            ToolResponse::ListFiles(list_response) => {
                assert_eq!(list_response.total_files, 3);
                assert!(!list_response.truncated);
                assert_eq!(list_response.limit, 100);
                // Check that tree string contains expected files
                assert!(list_response.files.contains("test.txt"));
                assert!(list_response.files.contains("subdir"));
                assert!(list_response.files.contains("nested.txt"));
            }
            _ => panic!("Expected ListFiles response"),
        }
    }

    #[tokio::test]
    async fn test_tool_executor_list_files_hidden_files_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::new(temp_dir.path().to_path_buf());

        // Create test files including hidden ones
        fs::write(temp_dir.path().join("normal.txt"), "Normal file").unwrap();
        fs::write(temp_dir.path().join(".hidden.txt"), "Hidden file").unwrap();
        fs::create_dir_all(temp_dir.path().join(".hidden_dir")).unwrap();
        fs::write(
            temp_dir.path().join(".hidden_dir/file.txt"),
            "File in hidden dir",
        )
        .unwrap();
        fs::create_dir_all(temp_dir.path().join("normal_dir")).unwrap();
        fs::write(
            temp_dir.path().join("normal_dir/.hidden_in_normal.txt"),
            "Hidden in normal dir",
        )
        .unwrap();

        // Test without including hidden files
        let request = ListFilesRequest {
            path: ".".to_string(),
            recursive: Some(true),
            include_hidden: Some(false),
            max_files: None,
        };

        let response = executor
            .execute(manager_models::ToolRequest::ListFiles(request))
            .await
            .unwrap();

        match response {
            ToolResponse::ListFiles(list_response) => {
                // Should only include normal.txt and normal_dir
                assert_eq!(list_response.total_files, 2);
                assert!(list_response.files.contains("normal.txt"));
                assert!(list_response.files.contains("normal_dir"));
                // Should not include any hidden files
                assert!(!list_response.files.contains(".hidden.txt"));
            }
            _ => panic!("Expected ListFiles response"),
        }

        // Test with including hidden files
        let request_hidden = ListFilesRequest {
            path: ".".to_string(),
            recursive: Some(true),
            include_hidden: Some(true),
            max_files: None,
        };

        let response_hidden = executor
            .execute(manager_models::ToolRequest::ListFiles(request_hidden))
            .await
            .unwrap();

        match response_hidden {
            ToolResponse::ListFiles(list_response) => {
                // Should include all files
                assert_eq!(list_response.total_files, 6);
                assert!(list_response.files.contains("normal.txt"));
                assert!(list_response.files.contains(".hidden.txt"));
                assert!(list_response.files.contains(".hidden_dir"));
                assert!(list_response.files.contains("file.txt"));
                assert!(list_response.files.contains("normal_dir"));
                assert!(list_response.files.contains(".hidden_in_normal.txt"));
            }
            _ => panic!("Expected ListFiles response"),
        }
    }

    #[tokio::test]
    async fn test_tool_executor_list_files_sorting() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::new(temp_dir.path().to_path_buf());

        // Create files with mixed case and types
        fs::create_dir_all(temp_dir.path().join("Z_dir")).unwrap();
        fs::create_dir_all(temp_dir.path().join("a_dir")).unwrap();
        fs::write(temp_dir.path().join("Z_file.txt"), "Z content").unwrap();
        fs::write(temp_dir.path().join("a_file.txt"), "a content").unwrap();
        fs::write(temp_dir.path().join("M_file.txt"), "M content").unwrap();

        let request = ListFilesRequest {
            path: ".".to_string(),
            recursive: Some(false),
            include_hidden: Some(false),
            max_files: None,
        };

        let response = executor
            .execute(manager_models::ToolRequest::ListFiles(request))
            .await
            .unwrap();

        match response {
            ToolResponse::ListFiles(list_response) => {
                assert_eq!(list_response.total_files, 5);

                // Check that tree string contains expected files
                assert!(list_response.files.contains("a_dir"));
                assert!(list_response.files.contains("Z_dir"));
                assert!(list_response.files.contains("a_file.txt"));
                assert!(list_response.files.contains("M_file.txt"));
                assert!(list_response.files.contains("Z_file.txt"));
            }
            _ => panic!("Expected ListFiles response"),
        }
    }

    #[tokio::test]
    async fn test_tool_executor_list_files_max_files_limit() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::new(temp_dir.path().to_path_buf());

        // Create more files than the limit
        for i in 0..15 {
            fs::write(
                temp_dir.path().join(format!("file_{:02}.txt", i)),
                format!("Content {}", i),
            )
            .unwrap();
        }

        // Test with max_files limit of 10
        let request = ListFilesRequest {
            path: ".".to_string(),
            recursive: Some(false),
            include_hidden: Some(false),
            max_files: Some(10),
        };

        let response = executor
            .execute(manager_models::ToolRequest::ListFiles(request))
            .await
            .unwrap();

        match response {
            ToolResponse::ListFiles(list_response) => {
                assert_eq!(list_response.total_files, 10);
                assert!(list_response.truncated);
                assert_eq!(list_response.limit, 10);
            }
            _ => panic!("Expected ListFiles response"),
        }

        // Test with max_files higher than available files
        let request_high_limit = ListFilesRequest {
            path: ".".to_string(),
            recursive: Some(false),
            include_hidden: Some(false),
            max_files: Some(20),
        };

        let response_high = executor
            .execute(manager_models::ToolRequest::ListFiles(request_high_limit))
            .await
            .unwrap();

        match response_high {
            ToolResponse::ListFiles(list_response) => {
                assert_eq!(list_response.total_files, 15);
                assert!(!list_response.truncated);
                assert_eq!(list_response.limit, 20);
            }
            _ => panic!("Expected ListFiles response"),
        }
    }

    #[tokio::test]
    async fn test_path_normalization() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::new(temp_dir.path().to_path_buf());

        // Create test structure
        fs::create_dir_all(temp_dir.path().join("subdir")).unwrap();
        fs::write(temp_dir.path().join("subdir/test.txt"), "test content").unwrap();

        // Test path normalization with . and .. components
        let test_cases = vec![
            ("./subdir/../subdir/test.txt", "subdir/test.txt"),
            ("subdir/./test.txt", "subdir/test.txt"),
            ("subdir//test.txt", "subdir/test.txt"), // Multiple slashes
        ];

        for (input_path, expected_relative) in test_cases {
            let resolved = executor.validate_and_resolve_path(input_path).unwrap();
            let expected = temp_dir.path().join(expected_relative);
            assert_eq!(
                resolved, expected,
                "Failed to normalize path: {}",
                input_path
            );
        }

        // Test directory traversal prevention
        let traversal_result = executor.validate_and_resolve_path("../outside");
        assert!(
            traversal_result.is_err(),
            "Should prevent directory traversal"
        );

        let traversal_result2 = executor.validate_and_resolve_path("../../../etc/passwd");
        assert!(
            traversal_result2.is_err(),
            "Should prevent directory traversal with multiple .."
        );
    }

    #[tokio::test]
    async fn test_tool_executor_read_file() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::new(temp_dir.path().to_path_buf());

        fs::write(temp_dir.path().join("test.txt"), "Hello World").unwrap();

        let request = ReadFileRequest {
            path: "test.txt".to_string(),
            max_size: None,
        };

        let response = executor
            .execute(manager_models::ToolRequest::ReadFile(request))
            .await
            .unwrap();

        match response {
            ToolResponse::ReadFile(read_response) => {
                assert_eq!(read_response.content, "Hello World");
                assert_eq!(read_response.size, 11);
            }
            _ => panic!("Expected ReadFile response"),
        }
    }

    #[tokio::test]
    async fn test_tool_executor_write_file() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::new(temp_dir.path().to_path_buf());

        let request = WriteFileRequest {
            path: "test.txt".to_string(),
            content: "Hello World".to_string(),
            create_dirs: None,
            append: None,
            search: None,
            replace: None,
            create_if_not_exists: Some(true),
        };

        let response = executor
            .execute(manager_models::ToolRequest::WriteFile(request))
            .await
            .unwrap();

        match response {
            ToolResponse::WriteFile(write_response) => {
                assert_eq!(write_response.path, "test.txt");
                assert!(write_response.success);
                assert_eq!(write_response.bytes_written, 11);
                assert!(write_response.created);
            }
            _ => panic!("Expected WriteFile response"),
        }

        // Verify file was created
        let content = fs::read_to_string(temp_dir.path().join("test.txt")).unwrap();
        assert_eq!(content, "Hello World");
    }

    #[tokio::test]
    async fn test_tool_executor_write_file_search_replace() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::new(temp_dir.path().to_path_buf());

        // Create initial file
        fs::write(temp_dir.path().join("test.txt"), "Hello old world").unwrap();

        let request = WriteFileRequest {
            path: "test.txt".to_string(),
            content: None, // Not used in search/replace
            create_dirs: None,
            append: None,
            search: Some("old".to_string()),
            replace: Some("new".to_string()),
            create_if_not_exists: None,
        };

        let response = executor
            .execute(manager_models::ToolRequest::WriteFile(request))
            .await
            .unwrap();

        match response {
            ToolResponse::WriteFile(write_response) => {
                assert_eq!(write_response.path, "test.txt");
                assert!(write_response.success);
                assert_eq!(write_response.bytes_written, 15); // "Hello new world".len()
                assert!(!write_response.created); // File was modified, not created
            }
            _ => panic!("Expected WriteFile response"),
        }

        // Verify content was replaced
        let content = fs::read_to_string(temp_dir.path().join("test.txt")).unwrap();
        assert_eq!(content, "Hello new world");
    }

    #[tokio::test]
    async fn test_tool_executor_grep_search_excludes_binary_files() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::new(temp_dir.path().to_path_buf());

        // Create test files including binary-like files
        fs::write(
            temp_dir.path().join("test.txt"),
            "This is a text file with some content",
        )
        .unwrap();
        fs::write(
            temp_dir.path().join("binary.exe"),
            b"\x00\x01\x02\x03binary content",
        )
        .unwrap();
        fs::write(
            temp_dir.path().join("image.jpg"),
            b"\xff\xd8\xff\xe0\x00\x10JFIFbinary image data",
        )
        .unwrap();

        let request = GrepRequest {
            pattern: "content".to_string(),
            path: None,
            include_pattern: None,
            exclude_pattern: None,
            recursive: Some(false),
            case_sensitive: Some(false),
            include_line_numbers: Some(true),
            max_results: Some(10),
            max_files_searched: Some(100),
        };

        let response = executor.execute(manager_models::ToolRequest::Grep(request)).await.unwrap();

        match response {
            ToolResponse::Grep(grep_response) => {
                // Should find content in text file but not in binary files
                assert_eq!(grep_response.files_searched, 1); // Only the .txt file should be searched
                assert!(grep_response
                    .matches
                    .iter()
                    .any(|m| m.file_path == "test.txt"));
                // Should not find matches in binary files
                assert!(!grep_response
                    .matches
                    .iter()
                    .any(|m| m.file_path.contains("binary.exe")));
                assert!(!grep_response
                    .matches
                    .iter()
                    .any(|m| m.file_path.contains("image.jpg")));
            }
            _ => panic!("Expected Grep response"),
        }
    }

    #[tokio::test]
    async fn test_tool_executor_grep_search() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::new(temp_dir.path().to_path_buf());

        // Create test files
        fs::write(
            temp_dir.path().join("test1.txt"),
            "fn main() {\n    println!(\"Hello\");\n}",
        )
        .unwrap();
        fs::write(
            temp_dir.path().join("test2.txt"),
            "fn helper() {\n    println!(\"World\");\n}",
        )
        .unwrap();

        let request = GrepRequest {
            pattern: "fn \\w+\\(\\)".to_string(),
            path: None,
            include_pattern: None,
            exclude_pattern: None,
            recursive: Some(false),
            case_sensitive: Some(false),
            include_line_numbers: Some(true),
            max_results: Some(10),
            max_files_searched: Some(100),
        };

        let response = executor.execute(manager_models::ToolRequest::Grep(request)).await.unwrap();

        match response {
            ToolResponse::Grep(grep_response) => {
                assert_eq!(grep_response.pattern, "fn \\w+\\(\\)");
                assert!(grep_response.total_matches >= 2); // Should find both functions
                assert!(grep_response.files_searched >= 2);
                assert!(!grep_response.truncated);

                // Check that we found matches
                let main_match = grep_response
                    .matches
                    .iter()
                    .find(|m| m.matched_text.contains("main"));
                let helper_match = grep_response
                    .matches
                    .iter()
                    .find(|m| m.matched_text.contains("helper"));

                assert!(main_match.is_some());
                assert!(helper_match.is_some());
            }
            _ => panic!("Expected Grep response"),
        }
    }
}