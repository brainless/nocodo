use crate::agents_api::state::AgentState;
use actix_web::{get, post, web, HttpResponse, Responder};
use nocodo_agents::storage::{EpicCommentRow, TaskCommentRow};
use nocodo_agents::{AgentType, CommentStorage, SqliteCommentStorage};
use serde::Deserialize;
use serde::Serialize;

#[derive(Deserialize)]
struct AddCommentRequest {
    author_type: String,
    author_user_id: Option<i64>,
    agent_type: Option<String>,
    content: String,
}

#[derive(Serialize)]
struct EpicCommentsResponse {
    comments: Vec<EpicCommentRow>,
}

#[derive(Serialize)]
struct TaskCommentsResponse {
    comments: Vec<TaskCommentRow>,
}

#[get("/api/epics/{epic_id}/comments")]
pub async fn list_epic_comments(
    state: web::Data<AgentState>,
    path: web::Path<i64>,
) -> impl Responder {
    let epic_id = path.into_inner();

    let storage = match SqliteCommentStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Storage error: {}", e)
            }))
        }
    };

    let comments = match storage.get_epic_comments(epic_id).await {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to load comments: {}", e)
            }))
        }
    };

    HttpResponse::Ok().json(EpicCommentsResponse { comments })
}

#[post("/api/epics/{epic_id}/comments")]
pub async fn add_epic_comment(
    state: web::Data<AgentState>,
    path: web::Path<i64>,
    request: web::Json<AddCommentRequest>,
) -> impl Responder {
    let epic_id = path.into_inner();
    let body = request.into_inner();

    let storage = match SqliteCommentStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Storage error: {}", e)
            }))
        }
    };

    let agent_type = body.agent_type.as_deref().map(AgentType::from_str);

    match storage
        .add_epic_comment(
            epic_id,
            &body.author_type,
            body.author_user_id,
            agent_type,
            body.content,
        )
        .await
    {
        Ok(comment_id) => HttpResponse::Ok().json(serde_json::json!({ "id": comment_id })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to add comment: {}", e)
        })),
    }
}

#[get("/api/tasks/{task_id}/comments")]
pub async fn list_task_comments(
    state: web::Data<AgentState>,
    path: web::Path<i64>,
) -> impl Responder {
    let task_id = path.into_inner();

    let storage = match SqliteCommentStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Storage error: {}", e)
            }))
        }
    };

    let comments = match storage.get_task_comments(task_id).await {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to load comments: {}", e)
            }))
        }
    };

    HttpResponse::Ok().json(TaskCommentsResponse { comments })
}

#[post("/api/tasks/{task_id}/comments")]
pub async fn add_task_comment(
    state: web::Data<AgentState>,
    path: web::Path<i64>,
    request: web::Json<AddCommentRequest>,
) -> impl Responder {
    let task_id = path.into_inner();
    let body = request.into_inner();

    let storage = match SqliteCommentStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Storage error: {}", e)
            }))
        }
    };

    let agent_type = body.agent_type.as_deref().map(AgentType::from_str);

    match storage
        .add_task_comment(
            task_id,
            &body.author_type,
            body.author_user_id,
            agent_type,
            body.content,
        )
        .await
    {
        Ok(comment_id) => HttpResponse::Ok().json(serde_json::json!({ "id": comment_id })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to add comment: {}", e)
        })),
    }
}
