use bstr::{BString, ByteSlice};
use rlua::Lua;
use std::{collections::HashMap, path::Path};
use tokio::fs;
use warp::path::FullPath;

use bytes::Bytes;
use hyper::http::Response;
use hyper::HeaderMap;

use crate::Reply;

const MOCK_DIR: &str = "./mock";

const LUA_ARGS_PATH: &str = "path";
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
    let mock = find_mock(
        MOCK_DIR.into(),
        &path_split[2..], /* skip "" and "mock" */
    )
    .await?;
    let reply = execute_mock(mock, path, query, headers, body).await?;
    Ok(reply)
}

async fn find_mock(cur_dir: String, path: &[&str]) -> Result<String, Error> {
    let mut cur_dir = cur_dir;
    let mut path = path;
    loop {
        let is_file = path.len() == 1;
        let target = path[0];
        let mut dir = fs::read_dir(&cur_dir).await?;

        let mut wildcard = None;
        let mut match_dir = None;

        while let Some(entry) = dir.next_entry().await? {
            if (is_file && entry.file_type().await?.is_file())
                || (!is_file && entry.file_type().await?.is_dir())
            {
                let file_name = entry.file_name();
                let base_name = is_file
                    .then(|| Path::new(&file_name).file_stem().unwrap_or(&file_name))
                    .unwrap_or(&file_name);

                if base_name == target {
                    match_dir.replace(entry);
                    break;
                } else if base_name == "_" {
                    wildcard.replace(entry);
                }
            }
        }
        let find = match_dir.or(wildcard);
        match (find, is_file) {
            (None, _) => {
                cur_dir.extend("/".chars().chain(target.chars()));
                break Err(Error::NotFound(cur_dir));
            }
            (Some(entry), true) => {
                cur_dir.extend(
                    "/".chars()
                        .chain(entry.file_name().to_str().unwrap().chars()),
                );
                break Ok(cur_dir);
            }
            (Some(entry), false) => {
                cur_dir.extend(
                    "/".chars()
                        .chain(entry.file_name().to_str().unwrap().chars()),
                );
                path = &path[1..]
            }
        }
    }
}

async fn execute_mock(
    mock: String,
    path: FullPath,
    query: HashMap<String, String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Reply, Error> {
    let script = fs::read(mock).await?;
    let task = tokio::task::spawn_blocking(move || {
        let vm = Lua::new();
        vm.context(|ctx| -> Result<Reply, Error> {
            let globals = ctx.globals();
            globals.set(LUA_ARGS_PATH, path.as_str())?;
            globals.set(LUA_ARGS_QUERY, query)?;
            globals.set(LUA_ARGS_HEADER, headers2map(&headers))?;
            globals.set(LUA_ARGS_BODY, body.as_bstr())?;

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
        assert_eq!(ret.unwrap(), "./mock/test/echo.lua");

        let ret = find_mock(MOCK_DIR.into(), &["test", "echo", "hello"]).await;
        assert!(matches!(ret, Ok(_)));
        assert_eq!(ret.unwrap(), "./mock/test/echo/hello.lua");

        let ret = find_mock(MOCK_DIR.into(), &["test", "nothing"]).await;
        assert!(matches!(ret, Ok(_)));
        assert_eq!(ret.unwrap(), "./mock/test/_.lua");

        let ret = find_mock(MOCK_DIR.into(), &["test", "nothing", "wildcard"]).await;
        assert!(matches!(ret, Ok(_)));
        assert_eq!(ret.unwrap(), "./mock/test/_/wildcard.lua");

        let ret = find_mock(MOCK_DIR.into(), &["test", "echo", "nomatch"]).await;
        assert!(matches!(ret, Err(_)));
    }
}
