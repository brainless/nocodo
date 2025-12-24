// Main handlers (system/health handlers)
pub mod main_handlers;
pub use main_handlers::AppState;

// Project handlers module
pub mod project_handlers;

// Work handlers module
pub mod work_handlers;

// User handlers module
pub mod user_handlers;

// Team handlers module
pub mod team_handlers;

// File handlers module
pub mod file_handlers;

// AI session handlers module
pub mod ai_session_handlers;

// Project commands handlers (separate module to keep main handlers from growing too large)
pub mod project_commands;
