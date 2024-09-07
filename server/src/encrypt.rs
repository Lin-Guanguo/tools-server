use std::collections::HashMap;

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, HeaderName, HeaderValue},
};
use reqwest::Method;

use crate::util::*;
use base64::engine::general_purpose::STANDARD as Base64;
use base64::Engine;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::state::ServerState;

#[derive(Deserialize, Debug)]
pub struct EncryptRequest {
    url: String,
    method: String,
    headers: HashMap<String, String>,
    body: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct EncryptResponse {
    status: u16,
    headers: HashMap<String, String>,
    body: Option<String>,
}

pub async fn encrypt(State(state): State<ServerState>, body: Bytes) -> Bytes {
    let request = decrypt_request(body);
    let body = request.body.map(|b| Base64.decode(b.as_bytes()).unwrap());
    let new_request = state
        .copy
        .http_client
        .request(
            Method::from_bytes(request.method.as_bytes()).unwrap(),
            &request.url,
        )
        .headers(convert_headers(request.headers))
        .with_opt(body, |builder, body| builder.body(body));

    let response = new_request.send().await.unwrap();
    let status = response.status().as_u16();
    let headers = convert_headers2(response.headers().clone());
    let response_body = response.bytes().await.unwrap();
    let response_body = if response_body.is_empty() {
        None
    } else {
        Some(Base64.encode(response_body.as_ref()))
    };

    let new_response = EncryptResponse {
        status,
        headers,
        body: response_body,
    };
    encrypt_response(new_response)
}

fn decrypt_request(body: Bytes) -> EncryptRequest {
    let mut body = body.to_vec();
    body.reverse();
    let body = Base64.decode(body).unwrap();
    serde_json::from_slice(&body).unwrap()
}

fn encrypt_response(resp: EncryptResponse) -> Bytes {
    let resp_json = serde_json::to_string(&resp).unwrap();
    let mut resp_base64 = Base64.encode(resp_json.as_bytes()).as_bytes().to_vec();
    resp_base64.reverse();
    Bytes::from(resp_base64)
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

fn convert_headers2(headers: HeaderMap) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (key, value) in headers.iter() {
        if let Ok(value_str) = value.to_str() {
            map.insert(key.as_str().to_string(), value_str.to_string());
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decrypt_request() {
        // test data is base64 encoded json reverse
        // echo '{ "url": "http://localhost:8001/echo", "method": "POST", "headers": { "testh": "1" }, "body": "aGVsbG8" }' | base64 -w 0 | rev
        let body = Bytes::from(
            "==gC9BiI4ckYzZ1RhJCI6ISek9mYiACL9BiIxICI6ICa0NXZ0JCI7BiOiMnclRWYlhmIgwiIUN1TQJCI6ICZvhGdl1mIgwiIvh2Yl9SMwADO6Q3cvhGbhN2bs9yL6AHd0hmIgojIsJXdiAye",
        );
        let decrypt = decrypt_request(body);
        assert_eq!(decrypt.url, "http://localhost:8001/echo");
        assert_eq!(decrypt.method, "POST");
        assert_eq!(decrypt.headers.len(), 1);
        assert_eq!(decrypt.headers.get("testh").unwrap().to_string(), "1");
        assert_eq!(decrypt.body.unwrap(), "aGVsbG8");
    }
}
