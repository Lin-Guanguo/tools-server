use axum::{
    routing::{any, get},
    Router,
};
use server::echo;

#[tokio::main]
async fn main() {
    println!("server start");

    let app = Router::new()
        .route("/", get(|| async { "Hello, Tools Server!" }))
        .route("/echo/*any", any(echo::echo));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8001").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
