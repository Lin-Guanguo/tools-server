use std::collections::HashMap;

use bytes::Bytes;
use hyper::HeaderMap;

use crate::Reply;

pub async fn echo(
    path: warp::path::FullPath,
    query: HashMap<String, String>,
    headers: HeaderMap,
    body: Bytes,
) -> Reply {
    let reply = format!(
        "path: {}\n\nquery: {:#?}\n\nheaders: {:#?}\n\nbody:\n{}\n\n",
        path.as_str(),
        query,
        headers,
        String::from_utf8_lossy(body.as_ref())
    );
    println!(" === accpet echo === \n{}", reply);
    Reply::UTF8(reply)
}
