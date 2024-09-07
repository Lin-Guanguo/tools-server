use std::sync::Arc;

#[derive(Clone)]
pub struct ServerState {
    pub share: Arc<ServerStateShare>,
    pub copy: ServerStateCopy,
}

#[derive(Clone)]
pub struct ServerStateShare {}

#[derive(Clone)]
pub struct ServerStateCopy {
    pub http_client: reqwest::Client,
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerState {
    pub fn new() -> Self {
        let http_client = reqwest::Client::new();

        let share = ServerStateShare {};
        let copy = ServerStateCopy { http_client };
        Self {
            share: Arc::new(share),
            copy,
        }
    }
}
