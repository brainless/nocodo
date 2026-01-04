use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub mod config;
pub mod handlers;
pub mod helpers;
pub mod models;

pub type DbConnection = Arc<Mutex<Connection>>;
