pub mod auth;
pub mod command_discovery;
pub mod config;
pub mod database;
pub mod error;
pub mod git;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod permissions;
pub mod routes;
pub mod socket;
pub mod templates;
pub mod websocket;

pub mod helpers;

#[cfg(test)]
mod tests;
