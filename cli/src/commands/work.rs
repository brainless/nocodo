use crate::client::ManagerClient;
use crate::error::CliError;
use tracing::info;

/// Work management commands
#[derive(Debug, clap::Subcommand)]
pub enum WorkCommands {
    /// List all works
    List,
    /// Create new work
    Create {
        #[arg(short, long)]
        title: String,
        #[arg(short, long)]
        project_id: Option<String>,
    },
    /// Show work history
    History {
        work_id: String,
        #[arg(short, long)]
        format: Option<OutputFormat>,
    },
    /// Add message to work
    AddMessage {
        work_id: String,
        #[arg(short, long)]
        content: String,
        #[arg(short, long, default_value = "text")]
        content_type: String, // text, markdown, json, code
        #[arg(short, long)] // Only for code type
        language: Option<String>,
    },
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    Markdown,
}

impl WorkCommands {
    pub async fn execute(&self, client: &ManagerClient) -> Result<(), CliError> {
        match self {
            WorkCommands::List => self.list_works(client).await,
            WorkCommands::Create { title, project_id } => self.create_work(client, title, project_id.clone()).await,
            WorkCommands::History { work_id, format } => {
                let format = format.clone().unwrap_or(OutputFormat::Text);
                self.show_work_history(client, work_id, format).await
            }
            WorkCommands::AddMessage {
                work_id,
                content,
                content_type,
                language,
            } => {
                let content_type_enum = match content_type.as_str() {
                    "text" => crate::client::MessageContentType::Text,
                    "markdown" => crate::client::MessageContentType::Markdown,
                    "json" => crate::client::MessageContentType::Json,
                    "code" => crate::client::MessageContentType::Code {
                        language: language.clone().unwrap_or_default(),
                    },
                    _ => crate::client::MessageContentType::Text,
                };
                
                self.add_message(
                    client,
                    work_id,
                    content,
                    content_type_enum,
                ).await
            }
        }
    }

    async fn list_works(&self, client: &ManagerClient) -> Result<(), CliError> {
        info!("Listing all works");
        
        let works = client.list_works().await?;
        
        if works.is_empty() {
            println!("No works found.");
        } else {
            println!("Works:");
            for work in works {
                println!("  {} - {} (Status: {})", work.id, work.title, work.status);
            }
        }
        
        Ok(())
    }

    async fn create_work(
        &self,
        client: &ManagerClient,
        title: &str,
        project_id: Option<String>,
    ) -> Result<(), CliError> {
        info!("Creating work with title: {}", title);
        
        let work = client.create_work(title.to_string(), project_id).await?;
        
        println!("Created work '{}' with ID: {}", work.title, work.id);
        Ok(())
    }

    async fn show_work_history(
        &self,
        client: &ManagerClient,
        work_id: &str,
        format: OutputFormat,
    ) -> Result<(), CliError> {
        info!("Showing work history for work ID: {}", work_id);
        
        let work_with_history = client.get_work_with_history(work_id).await?;
        
        match format {
            OutputFormat::Text => self.display_history_text(&work_with_history),
            OutputFormat::Json => self.display_history_json(&work_with_history),
            OutputFormat::Markdown => self.display_history_markdown(&work_with_history),
        }
    }

    async fn add_message(
        &self,
        client: &ManagerClient,
        work_id: &str,
        content: &str,
        content_type: crate::client::MessageContentType,
    ) -> Result<(), CliError> {
        info!("Adding message to work ID: {}", work_id);
        
        let message = client.add_message_to_work(
            work_id.to_string(),
            content.to_string(),
            content_type,
            crate::client::MessageAuthorType::User,
            None, // For CLI, we don't have a specific user ID
        ).await?;
        
        println!("Added message to work. Message ID: {}", message.id);
        Ok(())
    }

    fn display_history_text(&self, work_with_history: &crate::client::WorkWithHistory) -> Result<(), CliError> {
        println!("Work: {} ({})", work_with_history.work.title, work_with_history.work.id);
        println!("Status: {}", work_with_history.work.status);
        println!("Created: {}", work_with_history.work.created_at);
        println!("\nMessages ({} total):", work_with_history.total_messages);
        
        for message in &work_with_history.messages {
            let author = match &message.author_type {
                crate::client::MessageAuthorType::User => "User",
                crate::client::MessageAuthorType::Ai => "AI",
            };
            
            let content_type = match &message.content_type {
                crate::client::MessageContentType::Text => "text",
                crate::client::MessageContentType::Markdown => "markdown",
                crate::client::MessageContentType::Json => "json",
                crate::client::MessageContentType::Code { language } => {
                    if language.is_empty() {
                        "code"
                    } else {
                        language
                    }
                }
            };
            
            println!("\n[{}] {} ({}):", message.sequence_order, author, content_type);
            println!("{}", message.content);
        }
        
        Ok(())
    }

    fn display_history_json(&self, work_with_history: &crate::client::WorkWithHistory) -> Result<(), CliError> {
        let json = serde_json::to_string_pretty(work_with_history)
            .map_err(|e| CliError::Command(format!("Failed to serialize to JSON: {e}")))?;
        println!("{json}");
        Ok(())
    }

    fn display_history_markdown(&self, work_with_history: &crate::client::WorkWithHistory) -> Result<(), CliError> {
        println!("# Work: {} ({})", work_with_history.work.title, work_with_history.work.id);
        println!("**Status**: {}", work_with_history.work.status);
        println!("**Created**: {}", work_with_history.work.created_at);
        println!("\n## Messages ({} total)", work_with_history.total_messages);
        
        for message in &work_with_history.messages {
            let author = match &message.author_type {
                crate::client::MessageAuthorType::User => "User",
                crate::client::MessageAuthorType::Ai => "AI",
            };
            
            let content_type = match &message.content_type {
                crate::client::MessageContentType::Text => "text",
                crate::client::MessageContentType::Markdown => "markdown",
                crate::client::MessageContentType::Json => "json",
                crate::client::MessageContentType::Code { language } => {
                    if language.is_empty() {
                        "code"
                    } else {
                        language
                    }
                }
            };
            
            println!("\n### Message {} - {} ({})", message.sequence_order, author, content_type);
            
            match &message.content_type {
                crate::client::MessageContentType::Markdown => {
                    println!("{}", message.content);
                }
                crate::client::MessageContentType::Code { .. } => {
                    println!("```{content_type}");
                    println!("{}", message.content);
                    println!("```");
                }
                _ => {
                    println!("{}", message.content);
                }
            }
        }
        
        Ok(())
    }
}