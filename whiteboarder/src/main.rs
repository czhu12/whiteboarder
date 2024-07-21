//! Run with
//!
//! ```not_rust
//! cargo run -p example-hello-world
//! ```

use axum::{response::{Html, IntoResponse}, routing::{get, post, put}, Router, extract::{State, Path}, Json, http::StatusCode};
use serde::{Deserialize, Serialize};

use dotenv::dotenv;
use std::env;
use tower_http::services;
use redis::AsyncCommands;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
struct Point {
    x: i32,
    y: i32
}

#[derive(Serialize, Deserialize, Clone)]
struct Stroke {
    color: String,
    size: i32,
    points: Vec<Point>
}

#[derive(Serialize, Deserialize, Clone)]
struct Board {
    id: String,
    strokes: Vec<Stroke>
}

#[derive(Serialize, Deserialize, Clone)]
struct ErrorResponse {
    error: String,
}

type SharedState = Arc<Mutex<redis::Client>>;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let state: SharedState = Arc::new(Mutex::new(get_redis_client()));

    // build our application with a route
    let api_routes = Router::new()
        .route("/api/boards/:id", get(get_board).put(put_board))
        .route("/api/boards", post(create_board))
        .with_state(state);

    let app = Router::new()
        .nest("/", api_routes)
        .route_service("/", services::ServeFile::new("assets/index.html"))
        .route_service("/boards/:id", services::ServeFile::new("assets/index.html"))
        .nest_service("/assets", services::ServeDir::new("assets"));
    
    // run it
    let port = env::var("PORT").unwrap_or("3000".into());

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

fn get_redis_client() -> redis::Client {
    let redis_url = env::var("REDIS_URL").unwrap_or("redis://127.0.0.1".into());

    return redis::Client::open(redis_url).unwrap();
}

async fn put_board(
    Path(id): Path<String>,
    State(state): State<SharedState>,
    Json(board): Json<Board>,
) -> impl IntoResponse {
    let board_data = serde_json::to_string(&board).unwrap();
    let mut conn = state.lock().await.get_multiplexed_async_connection().await.unwrap();
    let _: () = conn.set(format!("board/{}", id), board_data).await.unwrap();
    StatusCode::OK.into_response()
}

async fn create_board(
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let id = Uuid::new_v4().to_string();
    let board = Board {
        id: id.clone(),
        strokes: vec![],
    };
    let board_data = serde_json::to_string(&board).unwrap();
    let mut conn = state.lock().await.get_multiplexed_async_connection().await.unwrap();
    let _: () = conn.set(format!("board/{}", id), board_data).await.unwrap();

    Json(board)
}

async fn get_board(
    Path(id): Path<String>,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let mut con = state.lock().await.get_multiplexed_async_connection().await.unwrap();

    let key = format!("board/{}", id);
    let result: Result<String, redis::RedisError> = con.get(&key).await;
    match result {
        Ok(value) => {
            (StatusCode::OK, Json(serde_json::from_str::<Board>(&value).unwrap())).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: String::from("Failed to parse JSON") })
        ).into_response()
    }
}