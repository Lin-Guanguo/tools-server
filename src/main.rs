use std::collections::HashMap;

use async_process::{Command, Stdio};
use futures::AsyncWriteExt;
use regex::{Captures, Regex};
use tools_server::{command::command, echo::echo};
use warp::{hyper::body::Bytes, Filter};

#[tokio::main]
async fn main() {
    let hello = warp::path!("command" / String)
        .and(warp::query::<HashMap<String, String>>())
        .and(warp::header::headers_cloned())
        .and(warp::body::bytes())
        .then(command);

    let echo = warp::path("echo")
        .and(warp::path::full())
        .and(warp::query::<HashMap<String, String>>())
        .and(warp::header::headers_cloned())
        .and(warp::body::bytes())
        .then(echo);

    warp::serve(hello.or(echo)).run(([0, 0, 0, 0], 3030)).await;
}

#[test]
fn test() {
    // TODO: more test
}
