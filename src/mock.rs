use bstr::{BString, ByteSlice};
use rlua::Lua;
use std::os::unix::prelude::OsStrExt;
use std::{collections::HashMap, path::Path};
use tokio::fs;
use warp::path::FullPath;

use bytes::Bytes;
use hyper::http::Response;
use hyper::HeaderMap;

use crate::Reply;

const MOCK_DIR: &str = "./mock";

const LUA_ARGS_PATH: &str = "path";
const LUA_ARGS_PATH_PARAM: &str = "path_param";
const LUA_ARGS_QUERY: &str = "query";
const LUA_ARGS_HEADER: &str = "header";
const LUA_ARGS_BODY: &str = "body";
const LUA_RESP_STATUS: &str = "resp_status";
const LUA_RESP_HEADER: &str = "resp_header";
const LUA_RESP_BODY: &str = "resp_body";

pub async fn mock(
    path: FullPath,
    query: HashMap<String, String>,
    headers: HeaderMap,
    body: Bytes,
) -> Reply {
    println!(
        " === accpet command === \npath: {:?}\nquery: {:?}\nheaders: {:?} \nbody: {:?}",
        path,
        query,
        headers,
        String::from_utf8_lossy(&body),
    );
    let ret = mock_inner(path, query, headers, body).await;
    match ret {
        Err(e) => Reply::UTF8(e.to_string()),
        Ok(ret) => ret,
    }
}

async fn mock_inner(
    path: FullPath,
    query: HashMap<String, String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Reply, Error> {
    let path_split = path.as_str().split('/').collect::<Vec<_>>();
    let (find, path_param) = find_mock(
        MOCK_DIR.into(),
        &path_split[2..], /* skip "" and "mock" */
    )
    .await?;
    let reply = execute_mock(find, path, path_param, query, headers, body).await?;
    Ok(reply)
}

async fn find_mock<'a>(
    cur_dir: String,
    path: &[&str],
) -> Result<(String, HashMap<String, String>), Error> {
    let mut cur_dir = cur_dir;
    let mut path = path;
    let mut path_param = HashMap::new();
    loop {
        let is_file = path.len() == 1;
        let target = path[0];
        let mut dir = fs::read_dir(&cur_dir).await?;

        let mut match_dir = None;
        let mut wildcard = None;

        while let Some(entry) = dir.next_entry().await? {
            let file_type = entry.file_type().await?;
            if (is_file && file_type.is_file()) || (!is_file && file_type.is_dir()) {
                let file_name = entry.file_name();
                let base_name = if is_file {
                    Path::new(&file_name).file_stem().unwrap_or(&file_name)
                } else {
                    &file_name
                };
                if base_name == target {
                    match_dir.replace(entry);
                    break;
                } else if base_name.as_bytes()[0] == b'_' {
                    wildcard.replace(entry);
                }
            }
        }

        let find = match (match_dir, wildcard) {
            (Some(e), _) => Some(e),
            (None, Some(e)) => {
                let file_name = e.file_name();
                let base_name = if is_file {
                    Path::new(&file_name).file_stem().unwrap_or(&file_name)
                } else {
                    &file_name
                };
                if base_name.len() > 1 {
                    let param_name = base_name.to_string_lossy()[1..].to_string();
                    path_param.insert(param_name, target.into());
                }
                Some(e)
            }
            _ => None,
        };

        match (find, is_file) {
            (None, _) => {
                cur_dir.extend("/".chars().chain(target.chars()));
                break Err(Error::NotFound(cur_dir));
            }
            (Some(entry), true) => {
                cur_dir.extend(
                    "/".chars()
                        .chain(entry.file_name().to_string_lossy().chars()),
                );
                break Ok((cur_dir, path_param));
            }
            (Some(entry), false) => {
                cur_dir.extend(
                    "/".chars()
                        .chain(entry.file_name().to_string_lossy().chars()),
                );
                path = &path[1..]
            }
        }
    }
}

async fn execute_mock(
    script: String,
    path: FullPath,
    path_param: HashMap<String, String>,
    query: HashMap<String, String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Reply, Error> {
    let script = fs::read(script).await?;
    let task = tokio::task::spawn_blocking(move || {
        let vm = Lua::new();
        vm.context(|ctx| -> Result<Reply, Error> {
            let globals = ctx.globals();
            globals.set(LUA_ARGS_PATH, path.as_str())?;
            globals.set(LUA_ARGS_QUERY, query)?;
            globals.set(LUA_ARGS_HEADER, headers2map(&headers))?;
            globals.set(LUA_ARGS_BODY, body.as_bstr())?;
            globals.set(LUA_ARGS_PATH_PARAM, path_param)?;

            ctx.load(&script).set_name(path.as_str())?.exec()?;

            let status = globals
                .get::<_, Option<u16>>(LUA_RESP_STATUS)?
                .unwrap_or(200);
            let header = globals.get::<_, Option<HashMap<String, BString>>>(LUA_RESP_HEADER)?;
            let body = globals
                .get::<_, Option<BString>>(LUA_RESP_BODY)?
                .map(Vec::from)
                .unwrap_or_else(Vec::new);

            let mut resp = Response::builder().status(status);
            for (k, v) in header.into_iter().flat_map(|i| i.into_iter()) {
                resp = resp.header(k, Vec::from(v));
            }
            let resp = resp.body(body)?;

            Ok(Reply::HttpBinary(resp))
        })
    });
    task.await.map_err(|_| Error::TokioBlockJoin)?
}

fn headers2map(headers: &HeaderMap) -> HashMap<String, BString> {
    let mut map: HashMap<String, BString> = HashMap::new();
    for (name, value) in headers.iter() {
        let entry = map.entry(name.to_string());
        entry
            .and_modify(|h| {
                h.push(b',');
                h.extend_from_slice(value.as_bytes());
            })
            .or_insert_with(|| value.as_bytes().into());
    }
    map
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("mock path:{0} not found")]
    NotFound(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Lua(#[from] rlua::Error),
    #[error("{0}")]
    Http(#[from] hyper::http::Error),
    #[error("tokio join lua script error")]
    TokioBlockJoin,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn route() {
        let ret = find_mock(MOCK_DIR.into(), &["test", "echo"]).await;
        assert!(matches!(ret, Ok(_)));
        assert_eq!(ret.unwrap().0, "./mock/test/echo.lua");

        let ret = find_mock(MOCK_DIR.into(), &["test", "echo", "hello"]).await;
        assert!(matches!(ret, Ok(_)));
        assert_eq!(ret.unwrap().0, "./mock/test/echo/hello.lua");

        let ret = find_mock(MOCK_DIR.into(), &["test", "nothing"]).await;
        assert!(matches!(ret, Ok(_)));
        assert_eq!(ret.unwrap().0, "./mock/test/_.lua");

        let ret = find_mock(MOCK_DIR.into(), &["test", "nothing", "wildcard"]).await;
        assert!(matches!(ret, Ok(_)));
        assert_eq!(ret.unwrap().0, "./mock/test/_param/_p2.lua");

        let ret = find_mock(MOCK_DIR.into(), &["test", "echo", "nomatch"]).await;
        assert!(matches!(ret, Err(_)));
    }
}
