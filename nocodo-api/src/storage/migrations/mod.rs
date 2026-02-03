mod v1__create_agent_sessions;
mod v2__create_agent_messages;
mod v3__create_agent_tool_calls;
mod v4__create_project_requirements_qna;
mod v5__create_project_settings;

use refinery::embed_migrations;

embed_migrations!("src/storage/migrations");

pub fn run_migrations(conn: &mut rusqlite::Connection) -> Result<(), refinery::Error> {
    migrations::runner().run(conn).map(|_| ())
}
