use actix_web::{test, web};
use serde_json::json;
use std::time::{Duration, Instant};

use nocodo_manager::models::{
    CreateProjectRequest, CreateWorkRequest, FileCreateRequest,
    FileListRequest, FileUpdateRequest,
};

use crate::common::{TestApp, TestDataGenerator};

#[actix_rt::test]
async fn test_project_creation_performance() {
    let test_app = TestApp::new().await;

    let mut creation_times = Vec::new();
    let project_count = 100;

    for i in 0..project_count {
        let start_time = Instant::now();

        let project_temp_dir = test_app.test_config().projects_dir().join(format!("perf-project-{}", i));
        let create_request = CreateProjectRequest {
            name: format!("perf-project-{}", i),
            path: Some(project_temp_dir.to_string_lossy().to_string()),
            language: Some("rust".to_string()),
            framework: Some("actix-web".to_string()),
            template: None,
        };

        let req = test::TestRequest::post()
            .uri("/api/projects")
            .set_json(&create_request)
            .to_request();

        let resp = test::call_service(&test_app.service(), req).await;
        assert!(resp.status().is_success());

        let duration = start_time.elapsed();
        creation_times.push(duration.as_millis());
    }

    // Calculate performance metrics
    let total_time: u128 = creation_times.iter().sum();
    let avg_time = total_time / project_count as u128;
    let min_time = creation_times.iter().min().unwrap();
    let max_time = creation_times.iter().max().unwrap();

    println!("Project Creation Performance ({} projects):", project_count);
    println!("  Total time: {}ms", total_time);
    println!("  Average time: {}ms", avg_time);
    println!("  Min time: {}ms", min_time);
    println!("  Max time: {}ms", max_time);

    // Performance assertions
    assert!(avg_time < 500, "Average project creation time should be less than 500ms, got {}ms", avg_time);
    assert!(max_time < &2000, "Max project creation time should be less than 2000ms, got {}ms", max_time);

    // Verify all projects were created
    let projects = test_app.db().get_all_projects().unwrap();
    assert_eq!(projects.len(), project_count);
}

#[actix_rt::test]
async fn test_work_session_creation_performance() {
    let test_app = TestApp::new().await;

    // Create a project first
    let project = TestDataGenerator::create_project(Some("perf-work-project"), Some("/tmp/perf-work-project"));
    test_app.db().create_project(&project).unwrap();

    let mut creation_times = Vec::new();
    let work_count = 200;

    for i in 0..work_count {
        let start_time = Instant::now();

        let create_request = CreateWorkRequest {
            title: format!("Performance Work Session {}", i),
            project_id: Some(project.id.clone()),
            tool_name: Some("test-tool".to_string()),
        };

        let req = test::TestRequest::post()
            .uri("/api/works")
            .set_json(&create_request)
            .to_request();

        let resp = test::call_service(&test_app.service(), req).await;
        assert!(resp.status().is_success());

        let duration = start_time.elapsed();
        creation_times.push(duration.as_millis());
    }

    // Calculate performance metrics
    let total_time: u128 = creation_times.iter().sum();
    let avg_time = total_time / work_count as u128;
    let min_time = creation_times.iter().min().unwrap();
    let max_time = creation_times.iter().max().unwrap();

    println!("Work Session Creation Performance ({} works):", work_count);
    println!("  Total time: {}ms", total_time);
    println!("  Average time: {}ms", avg_time);
    println!("  Min time: {}ms", min_time);
    println!("  Max time: {}ms", max_time);

    // Performance assertions
    assert!(avg_time < 300, "Average work creation time should be less than 300ms, got {}ms", avg_time);
    assert!(max_time < &1000, "Max work creation time should be less than 1000ms, got {}ms", max_time);

    // Verify all works were created
    let works = test_app.db().get_all_works().unwrap();
    assert_eq!(works.len(), work_count);
}

#[actix_rt::test]
async fn test_file_operations_performance() {
    let test_app = TestApp::new().await;

    // Create a project
    let project = TestDataGenerator::create_project(Some("perf-file-project"), Some("/tmp/perf-file-project"));
    test_app.db().create_project(&project).unwrap();
    std::fs::create_dir_all(&project.path).unwrap();

    let mut creation_times = Vec::new();
    let file_count = 50;

    // Create multiple files
    for i in 0..file_count {
        let start_time = Instant::now();

        let create_request = FileCreateRequest {
            project_id: project.id.clone(),
            path: format!("perf-file-{}.txt", i),
            content: Some(format!("Performance test content for file {}", i).repeat(10)), // Larger content
            is_directory: false,
        };

        let req = test::TestRequest::post()
            .uri("/api/files/create")
            .set_json(&create_request)
            .to_request();

        let resp = test::call_service(&test_app.service(), req).await;
        assert!(resp.status().is_success());

        let duration = start_time.elapsed();
        creation_times.push(duration.as_millis());
    }

    // Calculate performance metrics
    let total_time: u128 = creation_times.iter().sum();
    let avg_time = total_time / file_count as u128;
    let min_time = creation_times.iter().min().unwrap();
    let max_time = creation_times.iter().max().unwrap();

    println!("File Creation Performance ({} files):", file_count);
    println!("  Total time: {}ms", total_time);
    println!("  Average time: {}ms", avg_time);
    println!("  Min time: {}ms", min_time);
    println!("  Max time: {}ms", max_time);

    // Performance assertions
    assert!(avg_time < 200, "Average file creation time should be less than 200ms, got {}ms", avg_time);
    assert!(max_time < &500, "Max file creation time should be less than 500ms, got {}ms", max_time);

    // Test file reading performance
    let mut read_times = Vec::new();

    for i in 0..file_count {
        let start_time = Instant::now();

        let read_request = json!({
            "project_id": project.id,
            "path": format!("perf-file-{}.txt", i)
        });

        let req = test::TestRequest::post()
            .uri("/api/files/read")
            .set_json(&read_request)
            .to_request();

        let resp = test::call_service(&test_app.service(), req).await;
        assert!(resp.status().is_success());

        let duration = start_time.elapsed();
        read_times.push(duration.as_millis());
    }

    let read_total: u128 = read_times.iter().sum();
    let read_avg = read_total / file_count as u128;

    println!("File Reading Performance ({} files):", file_count);
    println!("  Total time: {}ms", read_total);
    println!("  Average time: {}ms", read_avg);

    assert!(read_avg < 100, "Average file read time should be less than 100ms, got {}ms", read_avg);
}

#[actix_rt::test]
async fn test_database_query_performance() {
    let test_app = TestApp::new().await;

    // Create test data
    let project_count = 50;
    let work_count_per_project = 10;

    let mut projects = Vec::new();

    // Create projects
    for i in 0..project_count {
        let project = TestDataGenerator::create_project(
            Some(&format!("query-perf-project-{}", i)),
            Some(&format!("/tmp/query-perf-project-{}", i)),
        );
        test_app.db().create_project(&project).unwrap();
        projects.push(project);
    }

    // Create works for each project
    for project in &projects {
        for j in 0..work_count_per_project {
            let work = TestDataGenerator::create_work(
                Some(&format!("query-perf-work-{}-{}", project.name, j)),
                Some(&project.id),
            );
            test_app.db().create_work(&work).unwrap();
        }
    }

    // Test project listing performance
    let start_time = Instant::now();
    let req = test::TestRequest::get().uri("/api/projects").to_request();
    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());
    let project_list_time = start_time.elapsed();

    // Test work listing performance
    let start_time = Instant::now();
    let req = test::TestRequest::get().uri("/api/works").to_request();
    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());
    let work_list_time = start_time.elapsed();

    // Test individual project queries
    let mut individual_project_times = Vec::new();
    for project in &projects {
        let start_time = Instant::now();
        let uri = format!("/api/projects/{}", project.id);
        let req = test::TestRequest::get().uri(&uri).to_request();
        let resp = test::call_service(&test_app.service(), req).await;
        assert!(resp.status().is_success());
        individual_project_times.push(start_time.elapsed().as_millis());
    }

    let avg_individual_time = individual_project_times.iter().sum::<u128>() / individual_project_times.len() as u128;

    println!("Database Query Performance:");
    println!("  Project list ({} projects): {}ms", project_count, project_list_time.as_millis());
    println!("  Work list ({} works): {}ms", project_count * work_count_per_project, work_list_time.as_millis());
    println!("  Average individual project query: {}ms", avg_individual_time);

    // Performance assertions
    assert!(project_list_time < Duration::from_millis(500), "Project listing should be faster than 500ms");
    assert!(work_list_time < Duration::from_millis(1000), "Work listing should be faster than 1000ms");
    assert!(avg_individual_time < 50, "Individual project queries should be faster than 50ms");
}

#[actix_rt::test]
async fn test_concurrent_request_performance() {
    let test_app = TestApp::new().await;

    // Create test data
    let project = TestDataGenerator::create_project(Some("concurrent-perf-project"), Some("/tmp/concurrent-perf-project"));
    test_app.db().create_project(&project).unwrap();
    std::fs::create_dir_all(&project.path).unwrap();

    let concurrent_requests = 20;
    let mut handles = Vec::new();

    let start_time = Instant::now();

    // Launch concurrent requests
    for i in 0..concurrent_requests {
        let project_id = project.id.clone();
        let service = test_app.service().clone();

        let handle = tokio::spawn(async move {
            // Create a file
            let create_request = FileCreateRequest {
                project_id: project_id.clone(),
                path: format!("concurrent-file-{}.txt", i),
                content: Some(format!("Content for concurrent file {}", i)),
                is_directory: false,
            };

            let req = test::TestRequest::post()
                .uri("/api/files/create")
                .set_json(&create_request)
                .to_request();

            let resp = test::call_service(&service, req).await;
            assert!(resp.status().is_success());

            // Read the file back
            let read_request = json!({
                "project_id": project_id,
                "path": format!("concurrent-file-{}.txt", i)
            });

            let req = test::TestRequest::post()
                .uri("/api/files/read")
                .set_json(&read_request)
                .to_request();

            let resp = test::call_service(&service, req).await;
            assert!(resp.status().is_success());

            i
        });

        handles.push(handle);
    }

    // Wait for all requests to complete
    let mut completed_requests = Vec::new();
    for handle in handles {
        completed_requests.push(handle.await.unwrap());
    }

    let total_time = start_time.elapsed();

    println!("Concurrent Request Performance ({} requests):", concurrent_requests);
    println!("  Total time: {}ms", total_time.as_millis());
    println!("  Average time per request: {}ms", total_time.as_millis() / concurrent_requests as u128);

    // Verify all requests completed
    assert_eq!(completed_requests.len(), concurrent_requests);
    let unique_requests: std::collections::HashSet<_> = completed_requests.iter().collect();
    assert_eq!(unique_requests.len(), concurrent_requests);

    // Performance assertions
    assert!(total_time < Duration::from_secs(5), "Concurrent requests should complete within 5 seconds");
    let avg_time_per_request = total_time.as_millis() / concurrent_requests as u128;
    assert!(avg_time_per_request < 500, "Average time per concurrent request should be less than 500ms");
}

#[actix_rt::test]
async fn test_memory_usage_performance() {
    let test_app = TestApp::new().await;

    // Create a large number of entities to test memory usage
    let project_count = 100;
    let work_count_per_project = 5;
    let message_count_per_work = 3;

    let mut projects = Vec::new();

    // Create projects
    for i in 0..project_count {
        let project = TestDataGenerator::create_project(
            Some(&format!("memory-perf-project-{}", i)),
            Some(&format!("/tmp/memory-perf-project-{}", i)),
        );
        test_app.db().create_project(&project).unwrap();
        projects.push(project);
    }

    // Create works and messages
    for project in &projects {
        for j in 0..work_count_per_project {
            let work = TestDataGenerator::create_work(
                Some(&format!("memory-perf-work-{}-{}", project.name, j)),
                Some(&project.id),
            );
            test_app.db().create_work(&work).unwrap();

            for k in 0..message_count_per_work {
                let message = TestDataGenerator::create_work_message(
                    &work.id,
                    &format!("Memory performance test message {} for work {}", k, work.title),
                    if k % 2 == 0 { nocodo_manager::models::MessageAuthorType::User }
                           else { nocodo_manager::models::MessageAuthorType::Ai },
                    k as i32,
                );
                test_app.db().create_work_message(&message).unwrap();
            }
        }
    }

    // Test memory usage during bulk operations
    let start_time = Instant::now();

    // Perform bulk read operations
    let req = test::TestRequest::get().uri("/api/projects").to_request();
    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let req = test::TestRequest::get().uri("/api/works").to_request();
    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let bulk_read_time = start_time.elapsed();

    println!("Memory Usage Performance:");
    println!("  Created {} projects, {} works, {} messages", project_count, project_count * work_count_per_project, project_count * work_count_per_project * message_count_per_work);
    println!("  Bulk read operations completed in {}ms", bulk_read_time.as_millis());

    // Performance assertions
    assert!(bulk_read_time < Duration::from_millis(2000), "Bulk read operations should complete within 2 seconds");

    // Verify data integrity
    let db_projects = test_app.db().get_all_projects().unwrap();
    let db_works = test_app.db().get_all_works().unwrap();

    assert_eq!(db_projects.len(), project_count);
    assert_eq!(db_works.len(), project_count * work_count_per_project);
}

#[actix_rt::test]
async fn test_api_response_time_distribution() {
    let test_app = TestApp::new().await;

    // Create test data
    let project = TestDataGenerator::create_project(Some("response-time-project"), Some("/tmp/response-time-project"));
    test_app.db().create_project(&project).unwrap();

    let work = TestDataGenerator::create_work(Some("Response Time Test"), Some(&project.id));
    test_app.db().create_work(&work).unwrap();

    // Test different API endpoints
    let endpoints = vec![
        ("/api/health", "GET", None),
        ("/api/projects", "GET", None),
        (&format!("/api/projects/{}", project.id), "GET", None),
        ("/api/works", "GET", None),
        (&format!("/api/works/{}", work.id), "GET", None),
    ];

    let mut response_times = Vec::new();

    for (endpoint, method, body) in endpoints {
        let mut times = Vec::new();

        // Make multiple requests to each endpoint
        for _ in 0..10 {
            let start_time = Instant::now();

            let req = match method {
                "GET" => test::TestRequest::get().uri(endpoint),
                "POST" => {
                    let mut req = test::TestRequest::post().uri(endpoint);
                    if let Some(b) = body {
                        req = req.set_json(b);
                    }
                    req
                },
                _ => panic!("Unsupported method"),
            }.to_request();

            let resp = test::call_service(&test_app.service(), req).await;
            assert!(resp.status().is_success());

            let duration = start_time.elapsed();
            times.push(duration.as_micros());
        }

        let avg_time = times.iter().sum::<u128>() / times.len() as u128;
        let min_time = times.iter().min().unwrap();
        let max_time = times.iter().max().unwrap();

        response_times.push((endpoint.to_string(), avg_time, *min_time, *max_time));

        println!("Endpoint: {} - Avg: {}μs, Min: {}μs, Max: {}μs", endpoint, avg_time, min_time, max_time);
    }

    // Performance assertions for different endpoint types
    for (endpoint, avg_time, _, max_time) in &response_times {
        if endpoint.contains("/health") {
            assert!(avg_time < &10000, "Health check should be faster than 10ms, got {}μs", avg_time);
        } else if endpoint.contains("/projects") || endpoint.contains("/works") {
            assert!(avg_time < &50000, "CRUD operations should be faster than 50ms, got {}μs", avg_time);
            assert!(max_time < &100000, "Max response time should be less than 100ms, got {}μs", max_time);
        }
    }
}

#[actix_rt::test]
async fn test_scalability_under_load() {
    let test_app = TestApp::new().await;

    let load_test_iterations = 50;
    let concurrent_operations = 5;

    let mut operation_times = Vec::new();

    for iteration in 0..load_test_iterations {
        let mut handles = Vec::new();
        let start_time = Instant::now();

        // Launch concurrent operations
        for operation in 0..concurrent_operations {
            let service = test_app.service().clone();
            let iteration = iteration;
            let operation = operation;

            let handle = tokio::spawn(async move {
                match operation % 5 {
                    0 => {
                        // Health check
                        let req = test::TestRequest::get().uri("/api/health").to_request();
                        let resp = test::call_service(&service, req).await;
                        assert!(resp.status().is_success());
                    },
                    1 => {
                        // List projects
                        let req = test::TestRequest::get().uri("/api/projects").to_request();
                        let resp = test::call_service(&service, req).await;
                        assert!(resp.status().is_success());
                    },
                    2 => {
                        // List works
                        let req = test::TestRequest::get().uri("/api/works").to_request();
                        let resp = test::call_service(&service, req).await;
                        assert!(resp.status().is_success());
                    },
                    3 => {
                        // Create a temporary work session
                        let work_req = CreateWorkRequest {
                            title: format!("Load Test Work {}-{}", iteration, operation),
                            project_id: None,
                            tool_name: Some("load-test".to_string()),
                        };

                        let req = test::TestRequest::post()
                            .uri("/api/works")
                            .set_json(&work_req)
                            .to_request();

                        let resp = test::call_service(&service, req).await;
                        assert!(resp.status().is_success());
                    },
                    4 => {
                        // Get templates
                        let req = test::TestRequest::get().uri("/api/templates").to_request();
                        let resp = test::call_service(&service, req).await;
                        assert!(resp.status().is_success());
                    },
                    _ => unreachable!(),
                }
            });

            handles.push(handle);
        }

        // Wait for all operations in this iteration to complete
        for handle in handles {
            handle.await.unwrap();
        }

        let iteration_time = start_time.elapsed();
        operation_times.push(iteration_time.as_millis());
    }

    // Calculate performance metrics
    let total_time: u128 = operation_times.iter().sum();
    let avg_time = total_time / load_test_iterations as u128;
    let min_time = operation_times.iter().min().unwrap();
    let max_time = operation_times.iter().max().unwrap();

    println!("Scalability Load Test Performance:");
    println!("  Iterations: {}", load_test_iterations);
    println!("  Concurrent operations per iteration: {}", concurrent_operations);
    println!("  Total operations: {}", load_test_iterations * concurrent_operations);
    println!("  Total time: {}ms", total_time);
    println!("  Average time per iteration: {}ms", avg_time);
    println!("  Min iteration time: {}ms", min_time);
    println!("  Max iteration time: {}ms", max_time);

    // Performance assertions
    assert!(avg_time < 1000, "Average iteration time should be less than 1000ms, got {}ms", avg_time);
    assert!(max_time < &3000, "Max iteration time should be less than 3000ms, got {}ms", max_time);

    // Verify system stability - no crashes or data corruption
    let projects = test_app.db().get_all_projects().unwrap();
    let works = test_app.db().get_all_works().unwrap();

    // Should have at least some works created during load test
    assert!(works.len() >= load_test_iterations / 2, "Should have created works during load test");

    println!("  Final state - Projects: {}, Works: {}", projects.len(), works.len());
}