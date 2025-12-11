use super::main_handlers::AppState;
use crate::auth;
use crate::error::AppError;
use crate::models::{
    CreateUserRequest, UpdateUserRequest, User, UserResponse,
};
use manager_models::{TeamItem, UserListItem, SearchQuery};
use actix_web::{web, HttpResponse, Result, HttpMessage};
use manager_models::TeamListResponse;

pub async fn list_users(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let users = data.database.get_all_users()?;
    let mut user_list_items = Vec::new();

    for user in users {
        let teams = data.database.get_user_teams(user.id)?;
        let team_items: Vec<TeamItem> = teams
            .into_iter()
            .map(|team| TeamItem {
                id: team.id,
                name: team.name,
            })
            .collect();

        let user_item = UserListItem {
            id: user.id,
            name: user.name,
            email: user.email,
            teams: team_items,
        };

        user_list_items.push(user_item);
    }

    let response = manager_models::UserListResponse {
        users: user_list_items,
    };
    Ok(HttpResponse::Ok().json(response))
}

pub async fn create_user(
    data: web::Data<AppState>,
    request: web::Json<CreateUserRequest>,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let create_req = request.into_inner();

    // Validate username
    if create_req.username.trim().is_empty() {
        return Err(AppError::InvalidRequest(
            "Username cannot be empty".to_string(),
        ));
    }

    // Check if user already exists
    if data.database.get_user_by_name(&create_req.username).is_ok() {
        return Err(AppError::InvalidRequest(
            "Username already exists".to_string(),
        ));
    }

    // Hash password
    let password_hash = auth::hash_password(&create_req.password)?;

    // Get current user ID for created_by field (currently not used, but reserved for future audit logging)
    let _created_by = req
        .extensions()
        .get::<crate::models::UserInfo>()
        .map(|u| u.id)
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))?;

    // Create user
    let user = User {
        id: 0, // Will be set by database
        name: create_req.username,
        email: create_req.email.unwrap_or_default(),
        role: None,
        password_hash,
        is_active: true,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        updated_at: std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        last_login_at: None,
    };

    let user_id = data.database.create_user(&user)?;
    let mut user = user;
    user.id = user_id;

    let response = UserResponse { user };
    Ok(HttpResponse::Created().json(response))
}

pub async fn get_user(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let user_id = path.into_inner();
    let user = data.database.get_user_by_id(user_id)?;
    let response = UserResponse { user };
    Ok(HttpResponse::Ok().json(response))
}

pub async fn update_user(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    request: web::Json<UpdateUserRequest>,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let user_id = path.into_inner();
    let update_req = request.into_inner();

    // Get current user for updated_by field (currently not used, but reserved for future audit logging)
    let _updated_by = req
        .extensions()
        .get::<crate::models::UserInfo>()
        .map(|u| u.id)
        .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))?;

    // Update user
    data.database.update_user(
        user_id,
        update_req.name.as_deref(),
        update_req.email.as_deref(),
    )?;

    // Update team memberships if provided
    if let Some(team_ids) = &update_req.team_ids {
        data.database.update_user_teams(user_id, team_ids)?;
    }

    let user = data.database.get_user_by_id(user_id)?;
    let response = UserResponse { user };
    Ok(HttpResponse::Ok().json(response))
}

pub async fn search_users(
    data: web::Data<AppState>,
    query: web::Query<SearchQuery>,
    _req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let search_query = query.into_inner();
    let users = data.database.search_users(&search_query.q)?;
    let mut user_list_items = Vec::new();

    for user in users {
        let teams = data.database.get_user_teams(user.id)?;
        let team_items: Vec<TeamItem> = teams
            .into_iter()
            .map(|team| TeamItem {
                id: team.id,
                name: team.name,
            })
            .collect();

        let user_item = UserListItem {
            id: user.id,
            name: user.name,
            email: user.email,
            teams: team_items,
        };

        user_list_items.push(user_item);
    }

    let response = manager_models::UserListResponse {
        users: user_list_items,
    };
    Ok(HttpResponse::Ok().json(response))
}

pub async fn get_user_teams(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    _req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let user_id = path.into_inner();
    let teams = data.database.get_user_teams(user_id)?;
    let teams: Vec<manager_models::TeamListItem> = teams
        .into_iter()
        .map(|t| manager_models::TeamListItem {
            id: t.id,
            name: t.name,
            description: t.description,
            permissions: Vec::new(), // Will be loaded separately
        })
        .collect();
    let response = TeamListResponse { teams };
    Ok(HttpResponse::Ok().json(response))
}

pub async fn delete_user(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let user_id = path.into_inner();
    data.database.delete_user(user_id)?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn login(
    data: web::Data<AppState>,
    request: web::Json<serde_json::Value>,
) -> Result<HttpResponse, AppError> {
    let req = request.into_inner();

    // Get username and password from request
    let username = req.get("username")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            AppError::InvalidRequest("Username is required".to_string())
        })?;

    let password = req.get("password")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            AppError::InvalidRequest("Password is required".to_string())
        })?;

    // Get user from database
    let user = data.database.get_user_by_name(username)
        .map_err(|_| AppError::Unauthorized("Invalid credentials".to_string()))?;

    // Verify password
    if !crate::auth::verify_password(password, &user.password_hash)? {
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    // Generate JWT token
    let config = data.config.read().map_err(|e| {
        AppError::Internal(format!("Failed to acquire config read lock: {}", e))
    })?;

    let jwt_secret = config.auth.as_ref()
        .and_then(|a| a.jwt_secret.as_ref())
        .ok_or_else(|| AppError::Internal("JWT secret not configured".to_string()))?;

    let claims = crate::auth::Claims::new(user.id, "user".to_string(), None);
    let token = crate::auth::generate_token(&claims, jwt_secret)?;

    let response = serde_json::json!({
        "token": token,
        "user": {
            "id": user.id,
            "name": user.name,
            "email": user.email
        }
    });

    Ok(HttpResponse::Ok().json(response))
}

pub async fn register(
    data: web::Data<AppState>,
    request: web::Json<serde_json::Value>,
    _req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let req = request.into_inner();

    // Get username and password from request
    let username = req.get("username")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            AppError::InvalidRequest("Username is required".to_string())
        })?;

    let password = req.get("password")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            AppError::InvalidRequest("Password is required".to_string())
        })?;

    // Hash password
    let password_hash = crate::auth::hash_password(password)?;

    // Create user
    let user = crate::models::User {
        id: 0, // Will be set by database
        name: username.to_string(),
        email: req.get("email")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default(),
        role: None,
        password_hash,
        is_active: true,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        updated_at: std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        last_login_at: None,
    };

    let user_id = data.database.create_user(&user)?;
    let mut user = user;
    user.id = user_id;

    let response = crate::models::UserResponse { user };
    Ok(HttpResponse::Created().json(response))
}