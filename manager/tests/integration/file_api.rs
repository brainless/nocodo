use actix_web::{test, web};
use serde_json::json;
use std::fs;
use std::path::Path;

use nocodo_manager::models::{FileListRequest, FileCreateRequest, FileUpdateRequest};

use crate::common::{TestApp, TestDataGenerator};

#[actix_rt::test]
async fn test_list_files_empty_project() {
    let test_app = TestApp::new().await;

    // Create a project with empty directory
    let project = TestDataGenerator::create_project(Some("empty-project"), Some("/tmp/empty-project"));
    test_app.db().create_project(&project).unwrap();

    // Create the project directory
    fs::create_dir_all(&project.path).unwrap();

    let list_request = FileListRequest {
        project_id: Some(project.id.clone()),
        path: None, // Root directory
    };

    let req = test::TestRequest::post()
        .uri("/api/files/list")
        .set_json(&list_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let files = body["files"].as_array().unwrap();
    assert_eq!(files.len(), 0); // Empty directory
    assert_eq!(body["current_path"], ".");
}

#[actix_rt::test]
async fn test_list_files_with_files() {
    let test_app = TestApp::new().await;

    // Create a project
    let project = TestDataGenerator::create_project(Some("files-project"), Some("/tmp/files-project"));
    test_app.db().create_project(&project).unwrap();

    // Create project directory and some files
    fs::create_dir_all(&project.path).unwrap();
    fs::write(Path::new(&project.path).join("README.md"), "# Test Project").unwrap();
    fs::write(Path::new(&project.path).join("main.rs"), "fn main() {}").unwrap();
    fs::create_dir_all(Path::new(&project.path).join("src")).unwrap();
    fs::write(Path::new(&project.path).join("src").join("lib.rs"), "pub fn test() {}").unwrap();

    let list_request = FileListRequest {
        project_id: Some(project.id.clone()),
        path: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/files/list")
        .set_json(&list_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let files = body["files"].as_array().unwrap();

    // Should have README.md, main.rs, and src directory
    assert_eq!(files.len(), 3);

    let file_names: Vec<&str> = files.iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();

    assert!(file_names.contains(&"README.md"));
    assert!(file_names.contains(&"main.rs"));
    assert!(file_names.contains(&"src"));

    // Check that src is marked as directory
    let src_file = files.iter().find(|f| f["name"] == "src").unwrap();
    assert_eq!(src_file["is_directory"], true);
}

#[actix_rt::test]
async fn test_list_files_subdirectory() {
    let test_app = TestApp::new().await;

    // Create a project
    let project = TestDataGenerator::create_project(Some("subdir-project"), Some("/tmp/subdir-project"));
    test_app.db().create_project(&project).unwrap();

    // Create directory structure
    fs::create_dir_all(&project.path).unwrap();
    fs::create_dir_all(Path::new(&project.path).join("src")).unwrap();
    fs::write(Path::new(&project.path).join("src").join("main.rs"), "fn main() {}").unwrap();
    fs::write(Path::new(&project.path).join("src").join("lib.rs"), "pub fn lib() {}").unwrap();

    let list_request = FileListRequest {
        project_id: Some(project.id.clone()),
        path: Some("src".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/files/list")
        .set_json(&list_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let files = body["files"].as_array().unwrap();

    assert_eq!(files.len(), 2);
    assert_eq!(body["current_path"], "src");

    let file_names: Vec<&str> = files.iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();

    assert!(file_names.contains(&"main.rs"));
    assert!(file_names.contains(&"lib.rs"));
}

#[actix_rt::test]
async fn test_list_files_invalid_project() {
    let test_app = TestApp::new().await;

    let list_request = FileListRequest {
        project_id: Some("non-existent-project".to_string()),
        path: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/files/list")
        .set_json(&list_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 404);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "project_not_found");
}

#[actix_rt::test]
async fn test_list_files_invalid_path() {
    let test_app = TestApp::new().await;

    // Create a project
    let project = TestDataGenerator::create_project(Some("invalid-path-project"), Some("/tmp/invalid-path-project"));
    test_app.db().create_project(&project).unwrap();
    fs::create_dir_all(&project.path).unwrap();

    let list_request = FileListRequest {
        project_id: Some(project.id.clone()),
        path: Some("non/existent/path".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/files/list")
        .set_json(&list_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 404);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].as_str().unwrap().contains("path_not_found"));
}

#[actix_rt::test]
async fn test_read_file() {
    let test_app = TestApp::new().await;

    // Create a project
    let project = TestDataGenerator::create_project(Some("read-file-project"), Some("/tmp/read-file-project"));
    test_app.db().create_project(&project).unwrap();

    // Create project directory and a file
    fs::create_dir_all(&project.path).unwrap();
    let file_path = Path::new(&project.path).join("test.txt");
    let file_content = "This is a test file content.";
    fs::write(&file_path, file_content).unwrap();

    let read_request = json!({
        "project_id": project.id,
        "path": "test.txt"
    });

    let req = test::TestRequest::post()
        .uri("/api/files/read")
        .set_json(&read_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["path"], "test.txt");
    assert_eq!(body["content"], file_content);
    assert!(body["modified_at"].is_number());
}

#[actix_rt::test]
async fn test_read_file_not_found() {
    let test_app = TestApp::new().await;

    // Create a project
    let project = TestDataGenerator::create_project(Some("read-not-found-project"), Some("/tmp/read-not-found-project"));
    test_app.db().create_project(&project).unwrap();
    fs::create_dir_all(&project.path).unwrap();

    let read_request = json!({
        "project_id": project.id,
        "path": "non-existent.txt"
    });

    let req = test::TestRequest::post()
        .uri("/api/files/read")
        .set_json(&read_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 404);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].as_str().unwrap().contains("file_not_found"));
}

#[actix_rt::test]
async fn test_create_file() {
    let test_app = TestApp::new().await;

    // Create a project
    let project = TestDataGenerator::create_project(Some("create-file-project"), Some("/tmp/create-file-project"));
    test_app.db().create_project(&project).unwrap();
    fs::create_dir_all(&project.path).unwrap();

    let create_request = FileCreateRequest {
        project_id: project.id.clone(),
        path: "new-file.txt".to_string(),
        content: Some("New file content".to_string()),
        is_directory: false,
    };

    let req = test::TestRequest::post()
        .uri("/api/files/create")
        .set_json(&create_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let file_info = &body["file"];

    assert_eq!(file_info["name"], "new-file.txt");
    assert_eq!(file_info["path"], "new-file.txt");
    assert_eq!(file_info["is_directory"], false);

    // Verify file was actually created
    let file_path = Path::new(&project.path).join("new-file.txt");
    assert!(file_path.exists());
    assert_eq!(fs::read_to_string(&file_path).unwrap(), "New file content");
}

#[actix_rt::test]
async fn test_create_directory() {
    let test_app = TestApp::new().await;

    // Create a project
    let project = TestDataGenerator::create_project(Some("create-dir-project"), Some("/tmp/create-dir-project"));
    test_app.db().create_project(&project).unwrap();
    fs::create_dir_all(&project.path).unwrap();

    let create_request = FileCreateRequest {
        project_id: project.id.clone(),
        path: "new-directory".to_string(),
        content: None,
        is_directory: true,
    };

    let req = test::TestRequest::post()
        .uri("/api/files/create")
        .set_json(&create_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let file_info = &body["file"];

    assert_eq!(file_info["name"], "new-directory");
    assert_eq!(file_info["is_directory"], true);

    // Verify directory was actually created
    let dir_path = Path::new(&project.path).join("new-directory");
    assert!(dir_path.exists());
    assert!(dir_path.is_dir());
}

#[actix_rt::test]
async fn test_create_file_already_exists() {
    let test_app = TestApp::new().await;

    // Create a project
    let project = TestDataGenerator::create_project(Some("create-exists-project"), Some("/tmp/create-exists-project"));
    test_app.db().create_project(&project).unwrap();
    fs::create_dir_all(&project.path).unwrap();

    // Create a file first
    let file_path = Path::new(&project.path).join("existing.txt");
    fs::write(&file_path, "existing content").unwrap();

    let create_request = FileCreateRequest {
        project_id: project.id.clone(),
        path: "existing.txt".to_string(),
        content: Some("new content".to_string()),
        is_directory: false,
    };

    let req = test::TestRequest::post()
        .uri("/api/files/create")
        .set_json(&create_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 409); // Conflict

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].as_str().unwrap().contains("already_exists"));
}

#[actix_rt::test]
async fn test_update_file() {
    let test_app = TestApp::new().await;

    // Create a project
    let project = TestDataGenerator::create_project(Some("update-file-project"), Some("/tmp/update-file-project"));
    test_app.db().create_project(&project).unwrap();
    fs::create_dir_all(&project.path).unwrap();

    // Create a file first
    let file_path = Path::new(&project.path).join("update-me.txt");
    fs::write(&file_path, "original content").unwrap();

    let update_request = FileUpdateRequest {
        project_id: project.id.clone(),
        content: "updated content".to_string(),
    };

    let req = test::TestRequest::put()
        .uri("/api/files/update-me.txt")
        .set_json(&update_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    // Verify file was updated
    let updated_content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(updated_content, "updated content");
}

#[actix_rt::test]
async fn test_update_file_not_found() {
    let test_app = TestApp::new().await;

    // Create a project
    let project = TestDataGenerator::create_project(Some("update-not-found-project"), Some("/tmp/update-not-found-project"));
    test_app.db().create_project(&project).unwrap();
    fs::create_dir_all(&project.path).unwrap();

    let update_request = FileUpdateRequest {
        project_id: project.id.clone(),
        content: "content".to_string(),
    };

    let req = test::TestRequest::put()
        .uri("/api/files/non-existent.txt")
        .set_json(&update_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 404);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].as_str().unwrap().contains("file_not_found"));
}

#[actix_rt::test]
async fn test_file_operations_workflow() {
    let test_app = TestApp::new().await;

    // Create a project
    let project = TestDataGenerator::create_project(Some("workflow-project"), Some("/tmp/workflow-project"));
    test_app.db().create_project(&project).unwrap();
    fs::create_dir_all(&project.path).unwrap();

    // 1. Create a file
    let create_request = FileCreateRequest {
        project_id: project.id.clone(),
        path: "workflow.txt".to_string(),
        content: Some("Initial content".to_string()),
        is_directory: false,
    };

    let create_req = test::TestRequest::post()
        .uri("/api/files/create")
        .set_json(&create_request)
        .to_request();

    let create_resp = test::call_service(&test_app.service(), create_req).await;
    assert!(create_resp.status().is_success());

    // 2. Read the file
    let read_request = json!({
        "project_id": project.id,
        "path": "workflow.txt"
    });

    let read_req = test::TestRequest::post()
        .uri("/api/files/read")
        .set_json(&read_request)
        .to_request();

    let read_resp = test::call_service(&test_app.service(), read_req).await;
    assert!(read_resp.status().is_success());

    let read_body: serde_json::Value = test::read_body_json(read_resp).await;
    assert_eq!(read_body["content"], "Initial content");

    // 3. Update the file
    let update_request = FileUpdateRequest {
        project_id: project.id.clone(),
        content: "Updated content".to_string(),
    };

    let update_req = test::TestRequest::put()
        .uri("/api/files/workflow.txt")
        .set_json(&update_request)
        .to_request();

    let update_resp = test::call_service(&test_app.service(), update_req).await;
    assert!(update_resp.status().is_success());

    // 4. Read again to verify update
    let read_req2 = test::TestRequest::post()
        .uri("/api/files/read")
        .set_json(&read_request)
        .to_request();

    let read_resp2 = test::call_service(&test_app.service(), read_req2).await;
    assert!(read_resp2.status().is_success());

    let read_body2: serde_json::Value = test::read_body_json(read_resp2).await;
    assert_eq!(read_body2["content"], "Updated content");

    // 5. List files to see our file
    let list_request = FileListRequest {
        project_id: Some(project.id.clone()),
        path: None,
    };

    let list_req = test::TestRequest::post()
        .uri("/api/files/list")
        .set_json(&list_request)
        .to_request();

    let list_resp = test::call_service(&test_app.service(), list_req).await;
    assert!(list_resp.status().is_success());

    let list_body: serde_json::Value = test::read_body_json(list_resp).await;
    let files = list_body["files"].as_array().unwrap();

    assert_eq!(files.len(), 1);
    assert_eq!(files[0]["name"], "workflow.txt");
    assert_eq!(files[0]["is_directory"], false);
}

#[actix_rt::test]
async fn test_file_nested_operations() {
    let test_app = TestApp::new().await;

    // Create a project
    let project = TestDataGenerator::create_project(Some("nested-project"), Some("/tmp/nested-project"));
    test_app.db().create_project(&project).unwrap();
    fs::create_dir_all(&project.path).unwrap();

    // Create nested directory structure
    fs::create_dir_all(Path::new(&project.path).join("src")).unwrap();
    fs::create_dir_all(Path::new(&project.path).join("src").join("components")).unwrap();

    // Create files in nested directories
    fs::write(Path::new(&project.path).join("src").join("main.rs"), "fn main() {}").unwrap();
    fs::write(Path::new(&project.path).join("src").join("components").join("button.rs"), "pub struct Button;").unwrap();

    // List root directory
    let root_list = FileListRequest {
        project_id: Some(project.id.clone()),
        path: None,
    };

    let root_req = test::TestRequest::post()
        .uri("/api/files/list")
        .set_json(&root_list)
        .to_request();

    let root_resp = test::call_service(&test_app.service(), root_req).await;
    assert!(root_resp.status().is_success());

    let root_body: serde_json::Value = test::read_body_json(root_resp).await;
    let root_files = root_body["files"].as_array().unwrap();

    assert_eq!(root_files.len(), 1);
    assert_eq!(root_files[0]["name"], "src");
    assert_eq!(root_files[0]["is_directory"], true);

    // List src directory
    let src_list = FileListRequest {
        project_id: Some(project.id.clone()),
        path: Some("src".to_string()),
    };

    let src_req = test::TestRequest::post()
        .uri("/api/files/list")
        .set_json(&src_list)
        .to_request();

    let src_resp = test::call_service(&test_app.service(), src_req).await;
    assert!(src_resp.status().is_success());

    let src_body: serde_json::Value = test::read_body_json(src_resp).await;
    let src_files = src_body["files"].as_array().unwrap();

    assert_eq!(src_files.len(), 2);

    let src_names: Vec<&str> = src_files.iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();

    assert!(src_names.contains(&"main.rs"));
    assert!(src_names.contains(&"components"));

    // Read nested file
    let read_request = json!({
        "project_id": project.id,
        "path": "src/components/button.rs"
    });

    let read_req = test::TestRequest::post()
        .uri("/api/files/read")
        .set_json(&read_request)
        .to_request();

    let read_resp = test::call_service(&test_app.service(), read_req).await;
    assert!(read_resp.status().is_success());

    let read_body: serde_json::Value = test::read_body_json(read_resp).await;
    assert_eq!(read_body["content"], "pub struct Button;");
    assert_eq!(read_body["path"], "src/components/button.rs");
}