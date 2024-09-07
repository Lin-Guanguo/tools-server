use axum::{
    body::Bytes,
    extract::Query,
    http::{HeaderMap, Method, Uri},
    response::Json,
};
use serde::Serialize;
use std::collections::HashMap;
use tracing::info;

#[derive(Serialize, Debug)]
pub struct RequestInfo {
    method: String,
    uri: String,
    headers: HashMap<String, String>,
    query_params: HashMap<String, String>,
    body: Option<String>,
}

pub async fn echo(
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    Query(query_params): Query<HashMap<String, String>>,
    body: Bytes,
) -> Json<RequestInfo> {
    let headers_map = headers
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("").to_string()))
        .collect::<HashMap<_, _>>();

    let body_str = if body.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(&body).to_string())
    };

    let request_info = RequestInfo {
        method: method.to_string(),
        uri: uri.to_string(),
        headers: headers_map,
        query_params,
        body: body_str,
    };

    info!(request = ?request_info);

    Json(request_info)
}
