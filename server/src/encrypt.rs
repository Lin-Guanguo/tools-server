use std::collections::HashMap;

use axum::{
    body::{Body, Bytes},
    extract::State,
    http::{HeaderMap, HeaderName, HeaderValue, Response},
};
use reqwest::Method;

use crate::util::*;
use base64::Engine;
use serde::Deserialize;
use tracing::info;

use crate::state::ServerState;

#[derive(Deserialize, Debug)]
pub struct EncryptRequest {
    url: String,
    method: String,
    headers: HashMap<String, String>,
    body: Option<String>,
}

pub async fn encrypt(State(state): State<ServerState>, body: Bytes) -> Response<Body> {
    let request = decrypt_request(body);

    info!("Received encrypt request");
    let new_request = state
        .copy
        .http_client
        .request(
            Method::from_bytes(request.method.as_bytes()).unwrap(),
            &request.url,
        )
        .headers(convert_headers(request.headers))
        .with_opt(request.body, |builder, body| builder.body(body));

    let response = new_request.send().await.unwrap();
    convert_reqwest_to_hyper_response(response).await
}

fn decrypt_request(body: Bytes) -> EncryptRequest {
    let mut body = body.to_vec();
    body.reverse();
    let body = base64::engine::general_purpose::STANDARD
        .decode(body)
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

fn convert_headers(headers: HashMap<String, String>) -> HeaderMap {
    let mut header_map = HeaderMap::new();
    for (key, value) in headers {
        let header_name = HeaderName::from_bytes(key.as_bytes()).unwrap();
        let header_value = HeaderValue::from_str(&value).unwrap();
        header_map.insert(header_name, header_value);
    }
    header_map
}

async fn convert_reqwest_to_hyper_response(reqwest_response: reqwest::Response) -> Response<Body> {
    let status = reqwest_response.status();
    let headers = reqwest_response.headers().clone();
    let body_bytes = reqwest_response.bytes().await.unwrap();

    let mut hyper_response = Response::builder()
        .status(status)
        .body(Body::from(body_bytes))
        .unwrap();

    let hyper_headers = hyper_response.headers_mut();
    for (key, value) in headers.iter() {
        hyper_headers.insert(key, value.clone());
    }

    hyper_response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decrypt_request() {
        // test data is base64 encoded json reverse
        // echo '{ "url": "http://localhost:8001/echo", "method": "POST", "headers": { "testh": "1" }, "body": "124121211211" }' | base64 -w 0 | rev
        let body = Bytes::from(
            "K0HIiETMyETMyEjMxQjMxICI6ISek9mYiACL9BiIxICI6ICa0NXZ0JCI7BiOiMnclRWYlhmIgwiIUN1TQJCI6ICZvhGdl1mIgwiIvh2Yl9SMwADO6Q3cvhGbhN2bs9yL6AHd0hmIgojIsJXdiAye",
        );
        let decrypt = decrypt_request(body);
        println!("{:?}", decrypt)
    }
}
