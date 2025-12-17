#![allow(dead_code)]

use super::main_handlers::AppState;
use crate::error::AppError;
use crate::models::{CreateTeamRequest, Permission, Team, UpdateTeamRequest};
use actix_web::{web, HttpMessage, HttpResponse, Result};
use manager_models::TeamListResponse;

/// Extract user ID from request
/// With nested PermissionMiddleware, UserInfo may not transfer from ServiceRequest to HttpRequest
/// This function tries extensions first, then extracts from JWT token as fallback
fn get_user_id(
    req: &actix_web::HttpRequest,
    config: &std::sync::Arc<std::sync::RwLock<crate::config::AppConfig>>,
) -> Result<i64, AppError> {
    // Try extensions first
    if let Some(user_info) = req.extensions().get::<crate::models::UserInfo>() {
        return Ok(user_info.id);
    }

    // Check if JWT secret is configured
    let jwt_secret_opt = {
        let config_guard = config
            .read()
            .map_err(|_| AppError::Internal("Failed to read config".to_string()))?;
        config_guard
            .auth
            .as_ref()
            .and_then(|a| a.jwt_secret.clone())
    };

    // If no JWT secret (test mode), return default test user ID
    let jwt_secret = match jwt_secret_opt {
        Some(secret) => secret,
        None => return Ok(1), // Test mode - use test user ID
    };

    // Fallback: extract from JWT token
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing authorization".to_string()))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("Invalid authorization format".to_string()))?;

    let claims = crate::auth::validate_token(token, &jwt_secret)
        .map_err(|_| AppError::Unauthorized("Invalid or expired token".to_string()))?;

    claims
        .sub
        .parse::<i64>()
        .map_err(|_| AppError::Internal("Invalid user ID in token".to_string()))
}

pub async fn list_teams(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let teams = data.database.get_all_teams()?;
    let manager_teams: Vec<manager_models::TeamListItem> = teams
        .into_iter()
        .map(|team| manager_models::TeamListItem {
            id: team.id,
            name: team.name,
            description: team.description,
            permissions: Vec::new(), // Will be loaded separately
        })
        .collect();
    let response = TeamListResponse {
        teams: manager_teams,
    };
    Ok(HttpResponse::Ok().json(response))
}

pub async fn create_team(
    data: web::Data<AppState>,
    request: web::Json<CreateTeamRequest>,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let create_req = request.into_inner();

    // Validate team name
    if create_req.name.trim().is_empty() {
        return Err(AppError::InvalidRequest(
            "Team name cannot be empty".to_string(),
        ));
    }

    // Get current user ID for created_by field
    let created_by = get_user_id(&req, &data.config)?;

    // Create team
    let team = Team::new(create_req.name, create_req.description, created_by);
    let team_id = data.database.create_team(&team)?;

    // Add creator as team member (similar to Super Admins team creation)
    data.database
        .add_team_member(team_id, created_by, Some(created_by))?;

    let mut team = team;
    team.id = team_id;

    Ok(HttpResponse::Created().json(team))
}

pub async fn get_team(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let team_id = path.into_inner();
    let team = data.database.get_team_by_id(team_id)?;
    Ok(HttpResponse::Ok().json(team))
}

pub async fn update_team(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    request: web::Json<UpdateTeamRequest>,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let team_id = path.into_inner();
    let update_req = request.into_inner();

    // Get current user ID for updated_by field (currently not used, but reserved for future audit logging)
    let _updated_by = get_user_id(&req, &data.config)?;

    // Update team
    data.database.update_team(team_id, &update_req)?;

    let team = data.database.get_team_by_id(team_id)?;
    Ok(HttpResponse::Ok().json(team))
}

pub async fn delete_team(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let team_id = path.into_inner();
    data.database.delete_team(team_id)?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn get_team_members(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let team_id = path.into_inner();
    let members = data.database.get_team_members(team_id)?;
    Ok(HttpResponse::Ok().json(members))
}

pub async fn add_team_member(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    request: web::Json<crate::models::AddTeamMemberRequest>,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let team_id = path.into_inner();
    let add_req = request.into_inner();

    // Get current user ID for added_by field
    let added_by = get_user_id(&req, &data.config)?;

    // Add team member
    data.database
        .add_team_member(team_id, add_req.user_id, Some(added_by))?;

    Ok(HttpResponse::Created().finish())
}

pub async fn remove_team_member(
    data: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
) -> Result<HttpResponse, AppError> {
    let (team_id, user_id) = path.into_inner();
    data.database.remove_team_member(team_id, user_id)?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn get_team_permissions(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let team_id = path.into_inner();
    let permissions = data.database.get_team_permissions(team_id)?;
    Ok(HttpResponse::Ok().json(permissions))
}

pub async fn list_permissions(data: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let permissions = data.database.get_all_permissions()?;
    Ok(HttpResponse::Ok().json(permissions))
}

pub async fn create_permission(
    data: web::Data<AppState>,
    request: web::Json<crate::models::CreatePermissionRequest>,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let create_req = request.into_inner();

    // Get current user ID for granted_by field
    let granted_by = get_user_id(&req, &data.config)?;

    // Create permission
    let permission = Permission::new(
        create_req.team_id,
        create_req.resource_type,
        create_req.resource_id,
        create_req.action,
        Some(granted_by),
    );

    let permission_id = data.database.create_permission(&permission)?;
    let mut permission = permission;
    permission.id = permission_id;

    Ok(HttpResponse::Created().json(permission))
}

pub async fn delete_permission(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let permission_id = path.into_inner();
    data.database.delete_permission(permission_id)?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn get_current_user_teams(
    data: web::Data<AppState>,
    _req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    // Get user ID from request
    let user_id = get_user_id(&_req, &data.config)?;

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
    let response = manager_models::TeamListResponse { teams };
    Ok(HttpResponse::Ok().json(response))
}
