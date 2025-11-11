pub mod auth;
pub mod bash_executor;
pub mod bash_permissions;
pub mod config;
pub mod database;
pub mod error;
pub mod handlers;
pub mod llm_agent;
pub mod llm_client;
pub mod llm_providers;
pub mod middleware;
pub mod models;
pub mod permissions;
pub mod socket;
pub mod templates;
pub mod tools;
pub mod websocket;

#[cfg(test)]
mod tests;
