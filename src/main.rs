use std::collections::HashMap;

use async_process::{Command, Stdio};
use futures::AsyncWriteExt;
use warp::Filter;

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
    body: warp::hyper::body::Bytes,
) -> Result<String, warp::Rejection> {
    let mut body = body.as_ref();
    let args = headers
        .get_all("args")
        .into_iter()
        .flat_map(|h| h.to_str().unwrap().split(','))
        .flat_map(|s| shell_words::split(s).unwrap())
        .map(|s| match s.as_str() {
            "$body" => {
                let (head, tail) = split_blank_line(body);
                body = tail;
                String::from_utf8_lossy(&head).to_string()
            }
            key if s.starts_with('$') => query.get(&key[1..]).cloned().unwrap_or(s),
            _ => s,
        })
        .collect::<Vec<_>>();

    println!(
        " === accpet command === \napp: {} \nquery: {:?}\nheaders: {:?} \nbody: {:?}\nargs: {:?}",
        app,
        query,
        headers,
        String::from_utf8_lossy(body),
        args
    );

    let start = std::time::Instant::now();
    let mut child = Command::new(format!("./tools/{}", app))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(body).await.unwrap();
    let out = child.output().await.unwrap();
    let end = std::time::Instant::now();

    Ok(format!(
        "status: {}\ntime: {}ms\n\nstdout:\n{}\n\nstderr:\n{}\n",
        &out.status.to_string(),
        (end - start).as_millis(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    ))
}

fn split_blank_line<'a, T>(input: &'a T) -> (&'a [u8], &'a [u8])
where
    T: AsRef<[u8]> + ?Sized,
{
    let input = input.as_ref();
    let pos = input.windows(2).position(|x| x == b"\n\n");
    if let Some(pos) = pos {
        let (head, tail) = input.split_at(pos);
        (head, &tail[2..])
    } else {
        (input, &[] as &[u8])
    }
}
