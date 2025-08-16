use crate::error::{AppError, AppResult};
use crate::models::Project;
use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub type DbConnection = Arc<Mutex<Connection>>;

pub struct Database {
    connection: DbConnection,
}

impl Database {
    pub fn new(db_path: &PathBuf) -> AppResult<Self> {
        // Ensure the database directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let conn = Connection::open(db_path)?;
        
        let database = Database {
            connection: Arc::new(Mutex::new(conn)),
        };
        
        database.run_migrations()?;
        
        Ok(database)
    }
    
    #[allow(dead_code)]
    pub fn connection(&self) -> DbConnection {
        Arc::clone(&self.connection)
    }
    
    fn run_migrations(&self) -> AppResult<()> {
        let conn = self.connection.lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {}", e)))?;
        
        // Create projects table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                path TEXT NOT NULL,
                language TEXT,
                framework TEXT,
                status TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;
        
        // Create an index on the name for faster lookups
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_projects_name ON projects(name)",
            [],
        )?;
        
        tracing::info!("Database migrations completed");
        Ok(())
    }
    
    pub fn get_all_projects(&self) -> AppResult<Vec<Project>> {
        let conn = self.connection.lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {}", e)))?;
        
        let mut stmt = conn.prepare(
            "SELECT id, name, path, language, framework, status, created_at, updated_at 
             FROM projects ORDER BY created_at DESC"
        )?;
        
        let project_iter = stmt.query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                language: row.get(3)?,
                framework: row.get(4)?,
                status: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        
        let mut projects = Vec::new();
        for project in project_iter {
            projects.push(project?);
        }
        
        Ok(projects)
    }
    
    pub fn get_project_by_id(&self, id: &str) -> AppResult<Project> {
        let conn = self.connection.lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {}", e)))?;
        
        let mut stmt = conn.prepare(
            "SELECT id, name, path, language, framework, status, created_at, updated_at 
             FROM projects WHERE id = ?"
        )?;
        
        let project = stmt.query_row([id], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                language: row.get(3)?,
                framework: row.get(4)?,
                status: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        }).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::ProjectNotFound(id.to_string()),
            _ => AppError::Database(e),
        })?;
        
        Ok(project)
    }
    
    pub fn create_project(&self, project: &Project) -> AppResult<()> {
        let conn = self.connection.lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {}", e)))?;
        
        conn.execute(
            "INSERT INTO projects (id, name, path, language, framework, status, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                project.id,
                project.name,
                project.path,
                project.language,
                project.framework,
                project.status,
                project.created_at,
                project.updated_at
            ],
        )?;
        
        tracing::info!("Created project: {} ({})", project.name, project.id);
        Ok(())
    }
    
    #[allow(dead_code)]
    pub fn update_project(&self, project: &Project) -> AppResult<()> {
        let conn = self.connection.lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {}", e)))?;
        
        let rows_affected = conn.execute(
            "UPDATE projects SET name = ?, path = ?, language = ?, framework = ?, 
             status = ?, updated_at = ? WHERE id = ?",
            params![
                project.name,
                project.path,
                project.language,
                project.framework,
                project.status,
                project.updated_at,
                project.id
            ],
        )?;
        
        if rows_affected == 0 {
            return Err(AppError::ProjectNotFound(project.id.clone()));
        }
        
        tracing::info!("Updated project: {} ({})", project.name, project.id);
        Ok(())
    }
    
    pub fn delete_project(&self, id: &str) -> AppResult<()> {
        let conn = self.connection.lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {}", e)))?;
        
        let rows_affected = conn.execute("DELETE FROM projects WHERE id = ?", [id])?;
        
        if rows_affected == 0 {
            return Err(AppError::ProjectNotFound(id.to_string()));
        }
        
        tracing::info!("Deleted project: {}", id);
        Ok(())
    }
}
