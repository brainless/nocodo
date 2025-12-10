pub mod auth;
pub mod bash_executor;
pub mod bash_permissions;
pub mod command_discovery;
pub mod config;
pub mod database;
pub mod error;
pub mod git;
pub mod handlers;
pub mod llm_agent;
pub mod llm_client;
pub mod middleware;
pub mod models;
pub mod permissions;
pub mod schema_provider;
pub mod socket;
pub mod templates;
pub mod websocket;

#[cfg(test)]
mod tests;
