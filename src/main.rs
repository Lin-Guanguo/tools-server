use std::collections::HashMap;

use tools_server::{command::command, echo::echo, mock::mock};
use warp::Filter;

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

    let mock = warp::path("mock")
        .and(warp::path::full())
        .and(warp::query::<HashMap<String, String>>())
        .and(warp::header::headers_cloned())
        .and(warp::body::bytes())
        .then(mock);

    warp::serve(hello.or(echo).or(mock))
        .run(([0, 0, 0, 0], 3030))
        .await;
}

#[test]
fn test() {
    // TODO: more test
}
