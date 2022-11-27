use std::collections::HashMap;

use async_process::{Command, Stdio};
use futures::AsyncWriteExt;
use warp::{hyper::body::Bytes, Filter};

#[tokio::main]
async fn main() {
    let hello = warp::path!("command" / String)
        .and(warp::query::<HashMap<String, String>>())
        .and(warp::header::headers_cloned())
        .and(warp::body::bytes())
        .and_then(execute);

    warp::serve(hello).run(([0, 0, 0, 0], 3030)).await;
}

async fn execute(
    app: String,
    query: HashMap<String, String>,
    headers: warp::hyper::HeaderMap,
    mut body: Bytes,
) -> Result<ExecuteReply, warp::Rejection> {
    let mut parse_header_args = |h: &str| {
        headers
            .get_all(h)
            .into_iter()
            .flat_map(|h| h.to_str().unwrap().split(','))
            .flat_map(|s| shell_words::split(s).unwrap())
            .map(|s| match s.as_str() {
                "$body" => {
                    let (head, tail) = split_blank_line(body.clone());
                    body = tail;
                    String::from_utf8_lossy(&head).to_string()
                }
                key if s.starts_with('$') => query.get(&key[1..]).cloned().unwrap_or(s),
                _ => s,
            })
            .collect::<Vec<_>>()
    };
    let args = parse_header_args("args");
    let opts = parse_header_args("opts");

    println!(
        " === accpet command === \napp: {}\nquery: {:?}\nheaders: {:?} \nbody: {:?}\nargs: {:?}\nopts: {:?}",
        app,
        query,
        headers,
        String::from_utf8_lossy(&body),
        args,
        opts
    );

    let start = std::time::Instant::now();
    let mut child = Command::new(format!("./tools/{}", app))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&body)
        .await
        .unwrap();
    let out = child.output().await.unwrap();
    let end = std::time::Instant::now();

    if let Some(_) = opts.iter().find(|s| *s == "stdout") {
        Ok(ExecuteReply::Binary(out.stdout))
    } else if let Some(_) = opts.iter().find(|s| *s == "stderr") {
        Ok(ExecuteReply::Binary(out.stderr))
    } else {
        Ok(ExecuteReply::UTF8(format!(
            "status: {}\ntime: {}ms\n\nstdout:\n{}\n\nstderr:\n{}\n",
            &out.status.to_string(),
            (end - start).as_millis(),
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        )))
    }
}

enum ExecuteReply {
    UTF8(String),
    Binary(Vec<u8>),
}

impl warp::Reply for ExecuteReply {
    fn into_response(self) -> warp::reply::Response {
        match self {
            ExecuteReply::UTF8(x) => x.into_response(),
            ExecuteReply::Binary(x) => x.into_response(),
        }
    }
}

fn split_blank_line(mut input: Bytes) -> (Bytes, Bytes) {
    let pos = input.windows(2).position(|x| x == b"\n\n");
    if let Some(pos) = pos {
        let mut tail = input.split_off(pos);
        (input, tail.split_off(2))
    } else {
        (input, Bytes::new())
    }
}
