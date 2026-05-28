use std::path::Path;

use actix_web::{post, web, HttpResponse, Responder};

use crate::agents_api::state::AgentState;

use super::types::{
    CodeIndexBuildRequest, CodeIndexGetFreeFnRequest, CodeIndexGetImplFnRequest,
    CodeIndexGetStructRequest, CodeIndexListImplFnsRequest, CodeIndexReindexRequest,
    ExtractFreeFnRequest, ExtractImplFnRequest, ExtractStructRequest, FindFreeFnRequest,
    FindImplFnRequest, FindStructRequest,
};

use nocodo_agents::code_extractor::{
    extract_free_fn, extract_impl_fn, extract_struct, find_free_fn_file, find_impl_fn_file,
    find_struct_file, CodeBlock, CodeIndex,
};

fn to_shared_block(block: CodeBlock) -> shared_types::CodeBlock {
    shared_types::CodeBlock {
        file: block.file.to_string_lossy().to_string(),
        start_line: block.start_line,
        end_line: block.end_line,
        source: block.source,
    }
}

fn to_shared_stats(
    stats: nocodo_agents::code_extractor::BuildStats,
) -> shared_types::CodeIndexBuildStats {
    shared_types::CodeIndexBuildStats {
        structs: stats.structs,
        free_fns: stats.free_fns,
        impl_fns: stats.impl_fns,
    }
}

fn get_project_path(db_path: &str, project_id: i64) -> Result<Option<String>, String> {
    use rusqlite::OptionalExtension;
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    let result = conn
        .query_row(
            "SELECT path FROM project WHERE id = ?1",
            [project_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(result)
}

fn code_index_path(_db_path: &str, project_id: i64) -> String {
    format!("code_index_{}.db", project_id)
}

// ---------------------------------------------------------------------------
// Single-file extraction
// ---------------------------------------------------------------------------

#[post("/api/code-extractor/extract-struct")]
pub async fn extract_struct_handler(
    state: web::Data<AgentState>,
    request: web::Json<ExtractStructRequest>,
) -> impl Responder {
    let req = request.into_inner();
    let project_path = match resolve_project_path(&state.db_path, req.project_id) {
        Some(p) => p,
        None => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({ "error": "Project not found" }))
        }
    };

    let file_path = Path::new(&project_path).join(&req.file);
    if !file_path.exists() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": format!("File not found: {}", req.file) }));
    }

    match extract_struct(&file_path, &req.name) {
        Ok(Some(block)) => HttpResponse::Ok().json(shared_types::CodeBlockResponse {
            block: Some(to_shared_block(block)),
        }),
        Ok(None) => HttpResponse::Ok().json(shared_types::CodeBlockResponse { block: None }),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

#[post("/api/code-extractor/extract-free-fn")]
pub async fn extract_free_fn_handler(
    state: web::Data<AgentState>,
    request: web::Json<ExtractFreeFnRequest>,
) -> impl Responder {
    let req = request.into_inner();
    let project_path = match resolve_project_path(&state.db_path, req.project_id) {
        Some(p) => p,
        None => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({ "error": "Project not found" }))
        }
    };

    let file_path = Path::new(&project_path).join(&req.file);
    if !file_path.exists() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": format!("File not found: {}", req.file) }));
    }

    match extract_free_fn(&file_path, &req.name) {
        Ok(Some(block)) => HttpResponse::Ok().json(shared_types::CodeBlockResponse {
            block: Some(to_shared_block(block)),
        }),
        Ok(None) => HttpResponse::Ok().json(shared_types::CodeBlockResponse { block: None }),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

#[post("/api/code-extractor/extract-impl-fn")]
pub async fn extract_impl_fn_handler(
    state: web::Data<AgentState>,
    request: web::Json<ExtractImplFnRequest>,
) -> impl Responder {
    let req = request.into_inner();
    let project_path = match resolve_project_path(&state.db_path, req.project_id) {
        Some(p) => p,
        None => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({ "error": "Project not found" }))
        }
    };

    let file_path = Path::new(&project_path).join(&req.file);
    if !file_path.exists() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": format!("File not found: {}", req.file) }));
    }

    match extract_impl_fn(&file_path, &req.struct_name, &req.fn_name) {
        Ok(Some(block)) => HttpResponse::Ok().json(shared_types::CodeBlockResponse {
            block: Some(to_shared_block(block)),
        }),
        Ok(None) => HttpResponse::Ok().json(shared_types::CodeBlockResponse { block: None }),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

// ---------------------------------------------------------------------------
// Find across all files
// ---------------------------------------------------------------------------

#[post("/api/code-extractor/find-struct")]
pub async fn find_struct_handler(
    state: web::Data<AgentState>,
    request: web::Json<FindStructRequest>,
) -> impl Responder {
    let req = request.into_inner();
    let project_path = match resolve_project_path(&state.db_path, req.project_id) {
        Some(p) => p,
        None => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({ "error": "Project not found" }))
        }
    };

    match find_struct_file(Path::new(&project_path), &req.name) {
        Ok(Some(file)) => HttpResponse::Ok().json(serde_json::json!({
            "file": file.to_string_lossy().to_string()
        })),
        Ok(None) => HttpResponse::Ok().json(serde_json::json!({ "file": null })),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

#[post("/api/code-extractor/find-free-fn")]
pub async fn find_free_fn_handler(
    state: web::Data<AgentState>,
    request: web::Json<FindFreeFnRequest>,
) -> impl Responder {
    let req = request.into_inner();
    let project_path = match resolve_project_path(&state.db_path, req.project_id) {
        Some(p) => p,
        None => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({ "error": "Project not found" }))
        }
    };

    match find_free_fn_file(Path::new(&project_path), &req.name) {
        Ok(Some(file)) => HttpResponse::Ok().json(serde_json::json!({
            "file": file.to_string_lossy().to_string()
        })),
        Ok(None) => HttpResponse::Ok().json(serde_json::json!({ "file": null })),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

#[post("/api/code-extractor/find-impl-fn")]
pub async fn find_impl_fn_handler(
    state: web::Data<AgentState>,
    request: web::Json<FindImplFnRequest>,
) -> impl Responder {
    let req = request.into_inner();
    let project_path = match resolve_project_path(&state.db_path, req.project_id) {
        Some(p) => p,
        None => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({ "error": "Project not found" }))
        }
    };

    match find_impl_fn_file(Path::new(&project_path), &req.struct_name, &req.fn_name) {
        Ok(Some(file)) => HttpResponse::Ok().json(serde_json::json!({
            "file": file.to_string_lossy().to_string()
        })),
        Ok(None) => HttpResponse::Ok().json(serde_json::json!({ "file": null })),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

// ---------------------------------------------------------------------------
// Code Index (SQLite-backed)
// ---------------------------------------------------------------------------

#[post("/api/code-extractor/index/build")]
pub async fn index_build_handler(
    state: web::Data<AgentState>,
    request: web::Json<CodeIndexBuildRequest>,
) -> impl Responder {
    let req = request.into_inner();
    let project_path = match resolve_project_path(&state.db_path, req.project_id) {
        Some(p) => p,
        None => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({ "error": "Project not found" }))
        }
    };

    let index_path = code_index_path(&state.db_path, req.project_id);
    let mut idx = match CodeIndex::open(&index_path) {
        Ok(i) => i,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    };

    match idx.build(Path::new(&project_path)) {
        Ok(stats) => HttpResponse::Ok().json(shared_types::CodeIndexBuildResponse {
            stats: to_shared_stats(stats),
        }),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

#[post("/api/code-extractor/index/reindex")]
pub async fn index_reindex_handler(
    state: web::Data<AgentState>,
    request: web::Json<CodeIndexReindexRequest>,
) -> impl Responder {
    let req = request.into_inner();
    let project_path = match resolve_project_path(&state.db_path, req.project_id) {
        Some(p) => p,
        None => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({ "error": "Project not found" }))
        }
    };

    let index_path = code_index_path(&state.db_path, req.project_id);
    let mut idx = match CodeIndex::open(&index_path) {
        Ok(i) => i,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    };

    let file_path = Path::new(&project_path).join(&req.file);
    match idx.reindex_file(Path::new(&project_path), &file_path) {
        Ok(stats) => HttpResponse::Ok().json(shared_types::CodeIndexBuildResponse {
            stats: to_shared_stats(stats),
        }),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

#[post("/api/code-extractor/index/list-structs")]
pub async fn index_list_structs_handler(
    state: web::Data<AgentState>,
    request: web::Json<CodeIndexBuildRequest>,
) -> impl Responder {
    let req = request.into_inner();
    let index_path = code_index_path(&state.db_path, req.project_id);
    let idx = match CodeIndex::open(&index_path) {
        Ok(i) => i,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    };

    match idx.list_structs() {
        Ok(names) => HttpResponse::Ok().json(shared_types::CodeIndexListStructsResponse { names }),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

#[post("/api/code-extractor/index/list-free-fns")]
pub async fn index_list_free_fns_handler(
    state: web::Data<AgentState>,
    request: web::Json<CodeIndexBuildRequest>,
) -> impl Responder {
    let req = request.into_inner();
    let index_path = code_index_path(&state.db_path, req.project_id);
    let idx = match CodeIndex::open(&index_path) {
        Ok(i) => i,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    };

    match idx.list_free_fns() {
        Ok(names) => HttpResponse::Ok().json(shared_types::CodeIndexListFreeFnsResponse { names }),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

#[post("/api/code-extractor/index/list-impl-fns")]
pub async fn index_list_impl_fns_handler(
    state: web::Data<AgentState>,
    request: web::Json<CodeIndexListImplFnsRequest>,
) -> impl Responder {
    let req = request.into_inner();
    let index_path = code_index_path(&state.db_path, req.project_id);
    let idx = match CodeIndex::open(&index_path) {
        Ok(i) => i,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    };

    match idx.list_impl_fns(&req.struct_name) {
        Ok(names) => HttpResponse::Ok().json(shared_types::CodeIndexListImplFnsResponse { names }),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

#[post("/api/code-extractor/index/get-struct")]
pub async fn index_get_struct_handler(
    state: web::Data<AgentState>,
    request: web::Json<CodeIndexGetStructRequest>,
) -> impl Responder {
    let req = request.into_inner();
    let index_path = code_index_path(&state.db_path, req.project_id);
    let idx = match CodeIndex::open(&index_path) {
        Ok(i) => i,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    };

    match idx.get_struct(&req.name) {
        Ok(Some(block)) => HttpResponse::Ok().json(shared_types::CodeBlockResponse {
            block: Some(to_shared_block(block)),
        }),
        Ok(None) => HttpResponse::Ok().json(shared_types::CodeBlockResponse { block: None }),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

#[post("/api/code-extractor/index/get-free-fn")]
pub async fn index_get_free_fn_handler(
    state: web::Data<AgentState>,
    request: web::Json<CodeIndexGetFreeFnRequest>,
) -> impl Responder {
    let req = request.into_inner();
    let index_path = code_index_path(&state.db_path, req.project_id);
    let idx = match CodeIndex::open(&index_path) {
        Ok(i) => i,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    };

    match idx.get_free_fn(&req.name) {
        Ok(Some(block)) => HttpResponse::Ok().json(shared_types::CodeBlockResponse {
            block: Some(to_shared_block(block)),
        }),
        Ok(None) => HttpResponse::Ok().json(shared_types::CodeBlockResponse { block: None }),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

#[post("/api/code-extractor/index/get-impl-fn")]
pub async fn index_get_impl_fn_handler(
    state: web::Data<AgentState>,
    request: web::Json<CodeIndexGetImplFnRequest>,
) -> impl Responder {
    let req = request.into_inner();
    let index_path = code_index_path(&state.db_path, req.project_id);
    let idx = match CodeIndex::open(&index_path) {
        Ok(i) => i,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    };

    match idx.get_impl_fn(&req.struct_name, &req.fn_name) {
        Ok(Some(block)) => HttpResponse::Ok().json(shared_types::CodeBlockResponse {
            block: Some(to_shared_block(block)),
        }),
        Ok(None) => HttpResponse::Ok().json(shared_types::CodeBlockResponse { block: None }),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn resolve_project_path(db_path: &str, project_id: i64) -> Option<String> {
    get_project_path(db_path, project_id).ok().flatten()
}
