pub mod api_client;
pub mod app;
pub mod config;
pub mod connection_manager;
pub mod ssh;
pub mod ui;
pub mod ui_text;

// New modular architecture
pub mod components;
pub mod pages;
pub mod services;
pub mod state;

pub use app::DesktopApp;
