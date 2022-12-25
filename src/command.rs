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
    println!(
        " === accpet command === \napp: {}\nquery: {:?}\nheaders: {:?} \nbody: {:?}",
        app,
        query,
        headers,
        String::from_utf8_lossy(&body),
    );

    let parse_header_args = |h: &str, vars: &HashMap<String, String>| {
        headers
            .get_all(h)
            .iter()
            .flat_map(|s| s.to_str().unwrap().split(','))
            .map(|s| replace_variable(vars, s))
            .flat_map(|s| shell_words::split(&s).unwrap())
            .collect::<Vec<_>>()
    };

    // parse variables
    let mut vars = query;
    vars.extend(
        parse_header_args("vars", &vars)
            .iter()
            .map(|s| s.split_once('='))
            .filter_map(|kv| kv.map(|(k, v)| (k.to_string(), v.to_string()))),
    );
    let mut body = body;
    let vars_copy = vars.clone();
    vars.iter_mut().for_each(|(_, v)| {
        if v.as_str() == "body" {
            let (head, tail) = split_blank_line(body.clone());
            body = tail;
            *v = replace_variable(&vars_copy, String::from_utf8_lossy(&head).as_ref()).to_string();
        }
    });

    let args = parse_header_args("args", &vars);
    let opts = parse_header_args("opts", &vars);
    let opts = CommandOpt::from_opts(opts);

    if opts.parse_body {
        let body2 = String::from_utf8_lossy(&body);
        let body2 = replace_variable(&vars, &body2);
        let body2 = Bytes::from(body2.to_string());
        body = body2
    }

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
    let async_process::Output {
        status,
        stdout,
        stderr,
    } = child.output().await.unwrap();
    let end = std::time::Instant::now();

    // reply
    match opts.output_format {
        RespFormat::Stdout => Reply::Binary(stdout),
        RespFormat::Stderr => Reply::Binary(stderr),
        RespFormat::Stdall => Reply::Binary({
            let (mut stdout, mut stderr) = (stdout, stderr);
            stdout.push(b'\n');
            stdout.push(b'\n');
            stdout.append(&mut stderr);
            stdout
        }),
        RespFormat::Format => Reply::UTF8(format!(
            "status: {}\ntime: {}ms\n\nstdout:\n{}\n\nstderr:\n{}\n",
            status,
            (end - start).as_millis(),
            String::from_utf8_lossy(&stdout),
            String::from_utf8_lossy(&stderr),
        )),
    }
}

enum RespFormat {
    Stdout,
    Stderr,
    Stdall,
    Format,
}

struct CommandOpt {
    parse_body: bool,
    output_format: RespFormat,
    unknown_opts: Vec<String>,
}

impl Default for CommandOpt {
    fn default() -> Self {
        Self {
            parse_body: false,
            output_format: RespFormat::Format,
            unknown_opts: Vec::new(),
        }
    }
}

impl CommandOpt {
    fn from_opts(opts: Vec<String>) -> CommandOpt {
        let mut opt = Self::default();
        opts.into_iter().for_each(|o| match o.as_str() {
            "parse-body" => opt.parse_body = true,
            "stdout" => opt.output_format = RespFormat::Stdout,
            "stderr" => opt.output_format = RespFormat::Stderr,
            "stdall" => opt.output_format = RespFormat::Stdall,
            "format" => opt.output_format = RespFormat::Format,
            _ => opt.unknown_opts.push(o),
        });
        opt
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
                ( \$\{\s* (?P<fn>[0-9A-Za-z_]+) \( \s*(?P<finput>[^\)]*)\s* \) \s*\} )
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
            let finput = cap.name("finput").unwrap();
            let finput = replace_variable(vars, finput.as_str());
            let finput = finput.as_ref();
            match fname.as_str() {
                "base64" => base64(finput),
                fname => format!("${{{}({})}}", fname, finput),
            }
        } else {
            panic!("inner error")
        }
    })
}

fn base64(input: &str) -> String {
    base64::encode(input)
}
