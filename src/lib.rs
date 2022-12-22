pub mod command;
pub mod echo;
pub mod mock;

pub use command::command;
pub use echo::echo;

pub enum Reply {
    UTF8(String),
    Binary(Vec<u8>),
}

impl warp::Reply for Reply {
    fn into_response(self) -> warp::reply::Response {
        match self {
            Reply::UTF8(x) => x.into_response(),
            Reply::Binary(x) => x.into_response(),
        }
    }
}
