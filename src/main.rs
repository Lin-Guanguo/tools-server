use async_process::{Command, Stdio};
use futures::AsyncWriteExt;
use warp::Filter;

#[tokio::main]
async fn main() {
    let hello = warp::path!("command" / String)
        .and(warp::header::headers_cloned())
        .and(warp::body::bytes())
        .and_then(execute);

    warp::serve(hello).run(([0, 0, 0, 0], 3030)).await;
}

async fn execute(
    app: String,
    headers: warp::hyper::HeaderMap,
    body: warp::hyper::body::Bytes,
) -> Result<String, warp::Rejection> {
    let args = headers
        .get_all("args")
        .into_iter()
        .flat_map(|h| h.to_str().unwrap().split(','))
        .flat_map(|s| shell_words::split(s).unwrap())
        .map(|s| {
            if s == "$body" {
                String::from_utf8_lossy(&body).into()
            } else {
                s
            }
        })
        .collect::<Vec<_>>();

    println!(
        " === accpet command === \napp: {} \nheaders {:?} \nbody: {:?}\nargs: {:?}",
        app, headers, body, args
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
        .write_all(body.as_ref())
        .await
        .unwrap();
    let out = child.output().await.unwrap();
    let end = std::time::Instant::now();

    Ok(format!(
        "status: {}\n\ntime: {}ms\n\nstdout: {}\n\nstderr: {}\n",
        &out.status.to_string(),
        (end - start).as_millis(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    ))
}
