use crate::error::Result;
use crate::models::{Workflow, WorkflowCommand, WorkflowInfo};
use std::path::Path;

/// Parser for GitHub Actions workflow files
pub struct WorkflowParser;

impl WorkflowParser {
    /// Parse a workflow file and extract commands
    pub fn parse_workflow_file(
        file_path: &Path,
        base_path: &Path,
    ) -> Result<(WorkflowInfo, Vec<WorkflowCommand>)> {
        let content = std::fs::read_to_string(file_path)?;
        let workflow: Workflow = serde_yaml::from_str(&content)?;

        let file_path_str = file_path.to_string_lossy().to_string();
        let workflow_name = workflow.name.clone().unwrap_or_else(|| {
            file_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });

        let mut commands = Vec::new();
        let mut job_count = 0;

        for (job_name, job) in &workflow.jobs {
            job_count += 1;
            let job_commands = Self::extract_commands_from_job(
                &workflow_name,
                job_name,
                job,
                &file_path_str,
                base_path,
            );
            commands.extend(job_commands);
        }

        let workflow_info = WorkflowInfo {
            name: workflow_name,
            file_path: file_path_str,
            jobs_count: job_count,
            commands_count: commands.len(),
        };

        Ok((workflow_info, commands))
    }

    /// Extract commands from a job
    fn extract_commands_from_job(
        workflow_name: &str,
        job_name: &str,
        job: &crate::models::Job,
        file_path: &str,
        base_path: &Path,
    ) -> Vec<WorkflowCommand> {
        let mut commands = Vec::new();

        for (step_index, step) in job.steps.iter().enumerate() {
            if let Some(run_command) = &step.run {
                let command_id = format!("{}_{}_{}", workflow_name, job_name, step_index);

                // Determine working directory
                let working_directory = step
                    .working_directory
                    .as_ref()
                    .or(job.working_directory.as_ref())
                    .map(|wd| {
                        if wd.starts_with('/') {
                            wd.clone()
                        } else {
                            base_path.join(wd).to_string_lossy().to_string()
                        }
                    });

                let command = WorkflowCommand {
                    id: command_id,
                    workflow_name: workflow_name.to_string(),
                    job_name: job_name.to_string(),
                    step_name: step.name.clone(),
                    command: run_command.clone(),
                    shell: step.shell.clone(),
                    working_directory,
                    environment: step.env.clone(),
                    file_path: file_path.to_string(),
                };

                commands.push(command);
            }
        }

        commands
    }

    /// Scan a directory for workflow files
    pub fn scan_workflows_directory(
        workflows_dir: &Path,
        base_path: &Path,
    ) -> Result<Vec<(WorkflowInfo, Vec<WorkflowCommand>)>> {
        let mut workflows = Vec::new();

        let entries = std::fs::read_dir(workflows_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|ext| ext == "yml" || ext == "yaml")
            {
                match Self::parse_workflow_file(&path, base_path) {
                    Ok(workflow_data) => workflows.push(workflow_data),
                    Err(e) => {
                        // Log error but continue with other files
                        eprintln!("Failed to parse workflow {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(workflows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_simple_workflow() {
        let yaml_content = r#"
name: CI
on: push
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - name: Run tests
        run: cargo test
"#;

        let temp_dir = tempfile::tempdir().unwrap();
        let workflow_path = temp_dir.path().join("ci.yml");
        tokio::fs::write(&workflow_path, yaml_content)
            .await
            .unwrap();

        let (info, commands) =
            WorkflowParser::parse_workflow_file(&workflow_path, temp_dir.path()).unwrap();

        assert_eq!(info.name, "CI");
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].command, "cargo test");
        assert_eq!(commands[0].job_name, "test");
    }
}
