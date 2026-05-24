use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CodeIndexBuildRequest {
    pub project_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct CodeIndexReindexRequest {
    pub project_id: i64,
    pub file: String,
}

#[derive(Debug, Deserialize)]
pub struct ExtractStructRequest {
    pub project_id: i64,
    pub file: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct ExtractFreeFnRequest {
    pub project_id: i64,
    pub file: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct ExtractImplFnRequest {
    pub project_id: i64,
    pub file: String,
    pub struct_name: String,
    pub fn_name: String,
}

#[derive(Debug, Deserialize)]
pub struct FindStructRequest {
    pub project_id: i64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct FindFreeFnRequest {
    pub project_id: i64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct FindImplFnRequest {
    pub project_id: i64,
    pub struct_name: String,
    pub fn_name: String,
}

#[derive(Debug, Deserialize)]
pub struct CodeIndexListImplFnsRequest {
    pub project_id: i64,
    pub struct_name: String,
}

#[derive(Debug, Deserialize)]
pub struct CodeIndexGetStructRequest {
    pub project_id: i64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CodeIndexGetFreeFnRequest {
    pub project_id: i64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CodeIndexGetImplFnRequest {
    pub project_id: i64,
    pub struct_name: String,
    pub fn_name: String,
}
