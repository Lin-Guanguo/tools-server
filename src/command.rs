use std::collections::HashMap;

use async_process::{Command, Stdio};
use bytes::Bytes;
use futures::AsyncWriteExt;
use hyper::HeaderMap;
use regex::Regex;

use crate::Reply;

pub async fn command(
    app: String,
    query: HashMap<String, String>,
    headers: HeaderMap,
    body: Bytes,
) -> Reply {
    let parse_header_args = |h: &str, vars: &HashMap<String, String>| {
        headers
            .get_all(h)
            .iter()
            .flat_map(|s| s.to_str().unwrap().split(","))
            .map(|s| replace_variable(&vars, s))
            .flat_map(|s| shell_words::split(&s).unwrap())
            .collect::<Vec<_>>()
    };

    // query vars
    let mut vars = query;

    // header vars
    let header_vars = parse_header_args("vars", &vars);
    let mut body = body;
    header_vars
        .iter()
        .map(|s| s.split_once("="))
        .for_each(|kv| match kv {
            // TODO: approve body vars
            Some((k, "body")) => {
                let (head, tail) = split_blank_line(body.clone());
                body = tail;
                let v = String::from_utf8_lossy(&head);
                let v = replace_variable(&vars, &v);
                vars.insert(k.to_string(), v.to_string());
            }
            Some((k, v)) => {
                vars.insert(k.to_string(), v.to_string());
            }
            _ => {}
        });

    let args = parse_header_args("args", &vars);
    let opts = parse_header_args("opts", &vars);

    // parse-body
    if let Some(_) = opts.iter().find(|s| *s == "parse-body") {
        let body2 = String::from_utf8_lossy(&body);
        let body2 = replace_variable(&vars, &body2);
        let body2 = Bytes::from(body2.to_string());
        body = body2
    }

    println!(
        " === accpet command === \napp: {}\nvars: {:?}\nheaders: {:?} \nbody: {:?}\nargs: {:?}\nopts: {:?}",
        app,
        vars,
        headers,
        String::from_utf8_lossy(&body),
        args,
        opts
    );

    // start execute
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

    // reply
    if let Some(_) = opts.iter().find(|s| *s == "stdout") {
        Reply::Binary(out.stdout)
    } else if let Some(_) = opts.iter().find(|s| *s == "stderr") {
        Reply::Binary(out.stderr)
    } else {
        Reply::UTF8(format!(
            "status: {}\ntime: {}ms\n\nstdout:\n{}\n\nstderr:\n{}\n",
            &out.status.to_string(),
            (end - start).as_millis(),
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        ))
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

fn replace_variable<'a>(
    vars: &HashMap<String, String>,
    input: &'a str,
) -> std::borrow::Cow<'a, str> {
    lazy_static::lazy_static!(
        static ref VARIABLE_PAT: Regex = Regex::new(
            r#"(?x)
                ( \$(?P<var1>[0-9A-Za-z_]+) ) |
                ( \$\{\s*(?P<var2>[0-9A-Za-z_]+)\s*\} ) |
                ( \$\{\s* (?P<fn>[0-9A-Za-z_]+) \( \s*(?P<input>[^\)]*)\s* \) \s*\} )
            "#,
        ).unwrap();
    );

    VARIABLE_PAT.replace_all(input, |cap: &regex::Captures| {
        if let Some(var) = cap.name("var1") {
            vars.get(var.as_str())
                .cloned()
                .unwrap_or_else(|| format!("${{{}}}", var.as_str()))
        } else if let Some(var) = cap.name("var2") {
            vars.get(var.as_str())
                .cloned()
                .unwrap_or_else(|| format!("${{{}}}", var.as_str()))
        } else if let Some(fname) = cap.name("fn") {
            let input = cap.name("input").unwrap();
            let input = replace_variable(&vars, input.as_str());
            let input = input.as_ref();
            match fname.as_str() {
                "base64" => base64(input),
                fname => format!("${{{}({})}}", fname, input),
            }
        } else {
            panic!("inner error")
        }
    })
}

fn base64(input: &str) -> String {
    base64::encode(input)
}
