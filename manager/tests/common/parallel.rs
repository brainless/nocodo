use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use super::{TestApp, TestDataGenerator};

/// Counter for generating unique test identifiers in parallel execution
static PARALLEL_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a unique identifier for parallel test execution
pub fn get_parallel_test_id() -> String {
    let counter = PARALLEL_COUNTER.fetch_add(1, Ordering::SeqCst);
    let thread_id = thread::current().id();
    format!("parallel-{}-{:?}", counter, thread_id)
}

/// Run a test function in parallel with other tests
pub async fn run_parallel_test<F, Fut>(test_name: &str, test_fn: F)
where
    F: FnOnce(TestApp) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let test_app = TestApp::new().await;
    let test_id = get_parallel_test_id();

    tracing::info!("Starting parallel test: {} ({})", test_name, test_id);

    // Run the test function
    test_fn(test_app).await;

    tracing::info!("Completed parallel test: {} ({})", test_name, test_id);
}

/// Run multiple test functions concurrently
pub async fn run_concurrent_tests<F, Fut>(tests: Vec<(String, F)>)
where
    F: FnOnce(TestApp) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let handles: Vec<_> = tests
        .into_iter()
        .map(|(test_name, test_fn)| {
            tokio::spawn(async move {
                run_parallel_test(&test_name, test_fn).await;
            })
        })
        .collect();

    // Wait for all tests to complete
    for handle in handles {
        handle.await.expect("Test task panicked");
    }
}

/// Test utilities for parallel execution scenarios
pub struct ParallelTestUtils;

impl ParallelTestUtils {
    /// Create multiple isolated projects in parallel
    pub async fn create_parallel_projects(count: usize) -> Vec<nocodo_manager::models::Project> {
        let mut handles = Vec::new();

        for i in 0..count {
            let handle = tokio::spawn(async move {
                let test_app = TestApp::new().await;
                let project_name = format!("parallel-project-{}", i);
                let project_path = format!("/tmp/parallel-project-{}", i);

                let project = TestDataGenerator::create_project(
                    Some(&project_name),
                    Some(&project_path),
                );

                test_app.db().create_project(&project).unwrap();

                // Verify isolation by checking project count
                let projects = test_app.db().get_all_projects().unwrap();
                assert_eq!(projects.len(), 1, "Project isolation violated in parallel execution");

                project
            });

            handles.push(handle);
        }

        let mut projects = Vec::new();
        for handle in handles {
            projects.push(handle.await.unwrap());
        }

        projects
    }

    /// Test concurrent work session creation
    pub async fn test_concurrent_work_creation(project_id: &str, work_count: usize) {
        let mut handles = Vec::new();

        for i in 0..work_count {
            let project_id = project_id.to_string();
            let handle = tokio::spawn(async move {
                let test_app = TestApp::new().await;

                let work_title = format!("Concurrent Work {}", i);
                let work = TestDataGenerator::create_work(
                    Some(&work_title),
                    Some(&project_id),
                );

                test_app.db().create_work(&work).unwrap();

                // Add a message to each work
                let message = TestDataGenerator::create_work_message(
                    &work.id,
                    &format!("Message for work {}", i),
                    nocodo_manager::models::MessageAuthorType::User,
                    0,
                );

                test_app.db().create_work_message(&message).unwrap();

                work
            });

            handles.push(handle);
        }

        let works: Vec<_> = handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect();

        // Verify all works were created
        assert_eq!(works.len(), work_count);

        // Verify all works have unique IDs
        let work_ids: std::collections::HashSet<_> = works.iter().map(|w| &w.id).collect();
        assert_eq!(work_ids.len(), work_count);
    }

    /// Test file operations in parallel
    pub async fn test_parallel_file_operations(project_id: &str, file_count: usize) {
        let mut handles = Vec::new();

        for i in 0..file_count {
            let project_id = project_id.to_string();
            let handle = tokio::spawn(async move {
                let test_app = TestApp::new().await;

                // Create a file
                let file_path = format!("parallel-file-{}.txt", i);
                let content = format!("Content for file {}", i);

                let create_request = nocodo_manager::models::FileCreateRequest {
                    project_id: project_id.clone(),
                    path: file_path.clone(),
                    content: Some(content.clone()),
                    is_directory: false,
                };

                // Note: This would normally call the API, but for this test we simulate
                // the file creation by directly writing to the project directory
                let project = test_app.db().get_project_by_id(&project_id).unwrap();
                let full_path = std::path::Path::new(&project.path).join(&file_path);

                std::fs::create_dir_all(full_path.parent().unwrap()).unwrap();
                std::fs::write(&full_path, &content).unwrap();

                (file_path, content)
            });

            handles.push(handle);
        }

        let files: Vec<_> = handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect();

        // Verify all files were created
        assert_eq!(files.len(), file_count);

        // Verify all files have unique names
        let file_names: std::collections::HashSet<_> = files.iter().map(|(name, _)| name).collect();
        assert_eq!(file_names.len(), file_count);
    }

    /// Stress test with many concurrent operations
    pub async fn stress_test_concurrent_operations(operation_count: usize) {
        let start_time = std::time::Instant::now();

        let handles: Vec<_> = (0..operation_count)
            .map(|i| {
                tokio::spawn(async move {
                    let test_app = TestApp::new().await;

                    // Create project
                    let project = TestDataGenerator::create_project(
                        Some(&format!("stress-project-{}", i)),
                        Some(&format!("/tmp/stress-project-{}", i)),
                    );
                    test_app.db().create_project(&project).unwrap();

                    // Create work
                    let work = TestDataGenerator::create_work(
                        Some(&format!("stress-work-{}", i)),
                        Some(&project.id),
                    );
                    test_app.db().create_work(&work).unwrap();

                    // Create message
                    let message = TestDataGenerator::create_work_message(
                        &work.id,
                        &format!("Stress message {}", i),
                        nocodo_manager::models::MessageAuthorType::User,
                        0,
                    );
                    test_app.db().create_work_message(&message).unwrap();

                    // Verify isolation
                    let projects = test_app.db().get_all_projects().unwrap();
                    let works = test_app.db().get_all_works().unwrap();

                    assert_eq!(projects.len(), 1);
                    assert_eq!(works.len(), 1);

                    i
                })
            })
            .collect();

        // Wait for all operations to complete
        let results: Vec<_> = handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect();

        let duration = start_time.elapsed();

        // Verify all operations completed
        assert_eq!(results.len(), operation_count);

        // Verify all indices are unique (each operation ran in isolation)
        let indices: std::collections::HashSet<_> = results.iter().collect();
        assert_eq!(indices.len(), operation_count);

        tracing::info!(
            "Stress test completed: {} operations in {:?} ({:?} per operation)",
            operation_count,
            duration,
            duration / operation_count as u32
        );

        // Performance assertion: should complete within reasonable time
        // Adjust threshold based on system performance
        let max_duration = Duration::from_secs(30); // 30 seconds for 100 operations
        assert!(
            duration < max_duration,
            "Stress test took too long: {:?} (max: {:?})",
            duration,
            max_duration
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_rt::test]
    async fn test_parallel_project_creation() {
        let projects = ParallelTestUtils::create_parallel_projects(5).await;

        assert_eq!(projects.len(), 5);

        // Verify all projects have unique IDs
        let project_ids: std::collections::HashSet<_> = projects.iter().map(|p| &p.id).collect();
        assert_eq!(project_ids.len(), 5);

        // Verify all projects have unique names
        let project_names: std::collections::HashSet<_> = projects.iter().map(|p| &p.name).collect();
        assert_eq!(project_names.len(), 5);
    }

    #[actix_rt::test]
    async fn test_concurrent_work_creation() {
        let test_app = TestApp::new().await;
        let project = TestDataGenerator::create_project(Some("concurrent-test"), Some("/tmp/concurrent-test"));
        test_app.db().create_project(&project).unwrap();

        ParallelTestUtils::test_concurrent_work_creation(&project.id, 10).await;

        // Verify all works were created in the main database
        let works = test_app.db().get_all_works().unwrap();
        assert_eq!(works.len(), 10);
    }

    #[actix_rt::test]
    async fn test_parallel_file_operations() {
        let test_app = TestApp::new().await;
        let project = TestDataGenerator::create_project(Some("parallel-files-test"), Some("/tmp/parallel-files-test"));
        test_app.db().create_project(&project).unwrap();

        // Create project directory
        std::fs::create_dir_all(&project.path).unwrap();

        ParallelTestUtils::test_parallel_file_operations(&project.id, 5).await;

        // Verify files were created
        let project_path = std::path::Path::new(&project.path);
        for i in 0..5 {
            let file_path = project_path.join(format!("parallel-file-{}.txt", i));
            assert!(file_path.exists());
            let content = std::fs::read_to_string(&file_path).unwrap();
            assert_eq!(content, format!("Content for file {}", i));
        }
    }

    #[actix_rt::test]
    async fn test_stress_concurrent_operations() {
        let operation_count = 20; // Smaller number for unit test
        ParallelTestUtils::stress_test_concurrent_operations(operation_count).await;
    }

    #[actix_rt::test]
    async fn test_run_parallel_test() {
        let test_name = "example-parallel-test";

        run_parallel_test(test_name, |test_app| async move {
            // Simple test: create a project
            let project = TestDataGenerator::create_project(Some("parallel-example"), Some("/tmp/parallel-example"));
            test_app.db().create_project(&project).unwrap();

            // Verify it was created
            let projects = test_app.db().get_all_projects().unwrap();
            assert_eq!(projects.len(), 1);
            assert_eq!(projects[0].name, "parallel-example");
        }).await;
    }

    #[actix_rt::test]
    async fn test_run_concurrent_tests() {
        let tests = vec![
            ("test-1".to_string(), |test_app: TestApp| async move {
                let project = TestDataGenerator::create_project(Some("concurrent-1"), Some("/tmp/concurrent-1"));
                test_app.db().create_project(&project).unwrap();
            }),
            ("test-2".to_string(), |test_app: TestApp| async move {
                let project = TestDataGenerator::create_project(Some("concurrent-2"), Some("/tmp/concurrent-2"));
                test_app.db().create_project(&project).unwrap();
            }),
            ("test-3".to_string(), |test_app: TestApp| async move {
                let project = TestDataGenerator::create_project(Some("concurrent-3"), Some("/tmp/concurrent-3"));
                test_app.db().create_project(&project).unwrap();
            }),
        ];

        run_concurrent_tests(tests).await;
    }
}