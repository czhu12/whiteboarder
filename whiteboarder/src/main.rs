//! Run with
//!
//! ```not_rust
//! cargo run -p example-hello-world
//! ```

use axum::{body::Body, extract::{Path, State}, http::{header, HeaderName, HeaderValue, StatusCode}, middleware, response::{IntoResponse, Response}, routing::{get, post, put}, Json, Router};
use data::AppState;
use serde::{Deserialize, Serialize};
use tower_http::{services::ServeFile, trace::{DefaultMakeSpan, TraceLayer}};
use tower::ServiceExt; // for `.oneshot()`
use tokio_util::io::ReaderStream;

use dotenv::dotenv;
use std::{collections::HashMap, env};
use tower_http::services;
use redis::AsyncCommands;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod data;
mod drawing;
mod websockets;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let state: Arc<AppState> = Arc::new(data::AppState {
        rooms: Mutex::new(HashMap::new()),
        redis_client: Mutex::new(get_redis_client()),
    });
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "example_websockets=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();



    // build our application with a route
    let api_routes = Router::new()
        .route("/api/boards/:id", get(get_board).put(put_board))
        .route("/api/boards", post(create_board))
        .route("/boards/:id", get(serve_file))
        .route("/ws", get(websockets::handler))
        .with_state(state);

    let app = Router::new()
        .nest("/", api_routes)
        .route_service("/", services::ServeFile::new("assets/index.html"))
        .nest_service("/assets", services::ServeDir::new("assets"))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    // run it
    let port = env::var("PORT").unwrap_or("3000".into());

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn serve_file(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // `File` implements `AsyncRead`
    let mut headers = Vec::new();
    let body;
    if id.ends_with(".svg") {
        let parts: Vec<&str> = id.split('.').collect();
        if let Some(board_id) = parts.get(0) {
            let mut con = state.redis_client.lock().await.get_multiplexed_async_connection().await.unwrap();
            let key = format!("board/{}", board_id);
            let result: Result<String, redis::RedisError> = con.get(&key).await;
            let mut board = serde_json::from_str::<data::Board>(&result.unwrap()).unwrap();
            let result = drawing::draw_svg(&mut board);
            body = Body::from(result);
            headers.push((HeaderName::from_static("content-type"), "image/svg+xml"));
        } else {
            return Err((StatusCode::NOT_FOUND, "Something went wrong with ID"))
        }
    } else {
        headers.push((HeaderName::from_static("content-type"), "text/html; charset=utf-8"));
        let file = match tokio::fs::File::open("assets/index.html").await {
            Ok(file) => file,
            Err(err) => return Err((StatusCode::NOT_FOUND, "File not found: {}"))
        };

        body = Body::from_stream(ReaderStream::new(file))
    }
    
    let mut response = Response::builder()
        .status(StatusCode::OK);
    
    for (header_name, header_value) in headers {
        response = response.header(header_name, HeaderValue::try_from(header_value).unwrap());
    }
    
    let response = response
        .body(body)
        .unwrap();

    Ok(response)

}

fn get_redis_client() -> redis::Client {
    let redis_url = env::var("REDIS_URL").unwrap_or("redis://127.0.0.1".into());

    return redis::Client::open(redis_url).unwrap();
}

async fn put_board(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(board): Json<data::Board>,
) -> impl IntoResponse {
    let board_data = serde_json::to_string(&board).unwrap();
    // Send out the board data into redis
    let mut rooms = state.rooms.lock().await;
    let channel = format!("boards/{}", id);
    let room = rooms.get(&channel);
    match room {
        Some(r) => {
            let payload = data::WebSocketMessage {
                messagetype: "board".to_string(),
                channel: channel,
                payload: data::WebSocketPayload::BoardUpdate(board)
            };
            let value = serde_json::to_string(&payload).unwrap();
            let _ = r.tx.clone().send(value);
        },
        None => (),
    }

    let mut conn = state.redis_client.lock().await.get_multiplexed_async_connection().await.unwrap();
    let _: () = conn.set(format!("board/{}", id), board_data).await.unwrap();
    StatusCode::OK.into_response()
}

async fn create_board(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let id = Uuid::new_v4().to_string();
    let board = data::Board {
        id: id.clone(),
        strokes: vec![],
    };
    let board_data = serde_json::to_string(&board).unwrap();
    let mut conn = state.redis_client.lock().await.get_multiplexed_async_connection().await.unwrap();
    let _: () = conn.set(format!("board/{}", id), board_data).await.unwrap();

    Json(board)
}

async fn get_board(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut con = state.redis_client.lock().await.get_multiplexed_async_connection().await.unwrap();

    let key = format!("board/{}", id);
    let result: Result<String, redis::RedisError> = con.get(&key).await;
    match result {
        Ok(value) => {
            (StatusCode::OK, Json(serde_json::from_str::<data::Board>(&value).unwrap())).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(data::ErrorResponse { error: String::from("Failed to parse JSON") })
        ).into_response()
    }
}