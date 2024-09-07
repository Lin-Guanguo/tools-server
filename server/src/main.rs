use axum::{
    routing::{any, get},
    Router,
};
use server::{echo, encrypt, state::ServerState};
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .finish();

    subscriber.init();

    let state = ServerState::new();

    let app = Router::new()
        .route("/", get(|| async { "Hello, Tools Server!" }))
        .route("/echo", any(echo::echo))
        .route("/echo/*any", any(echo::echo))
        .route("/encrypt", any(encrypt::encrypt))
        .with_state(state);

    println!("server start");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8001").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
