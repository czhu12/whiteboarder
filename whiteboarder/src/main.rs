//! Run with
//!
//! ```not_rust
//! cargo run -p example-hello-world
//! ```

use axum::{body::Body, extract::{Path, State}, http::{header, HeaderName, HeaderValue, StatusCode}, response::{IntoResponse, Response}, routing::{get, post, put}, Json, Router};
use serde::{Deserialize, Serialize};
use tera::{Context, Tera};
use tower_http::services::ServeFile;
use tower::ServiceExt; // for `.oneshot()`
use tokio_util::io::ReaderStream;

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
    points: Vec<Point>,
}

impl Stroke {
    fn renderable(&self, x_offset: i32, y_offset: i32) -> TeraStroke {
        TeraStroke::new(
            self.color.clone(),
            self.size,
            self.points.clone(),
            x_offset,
            y_offset
        )
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct TeraStroke {
    color: String,
    size: i32,
    points: Vec<Point>,
    polyline: String,
}

impl TeraStroke {
    fn new(color: String, size: i32, points: Vec<Point>, x_offset: i32, y_offset: i32) -> Self {
        let polyline = points.iter()
            .map(|point| format!("{},{}", point.x - x_offset, point.y - y_offset))
            .collect::<Vec<String>>()
            .join(" ");
        Self {
            color,
            size,
            points,
            polyline
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct Board {
    id: String,
    strokes: Vec<Stroke>
}

impl Board {
    fn width(&self) -> i32 {
        let all_x: Vec<i32> = self.strokes.iter().flat_map(|stroke| stroke.points.iter().map(|point| point.x)).collect();
        all_x.iter().max().unwrap_or(&0) - all_x.iter().min().unwrap_or(&0)
    }
    fn height(&self) -> i32 {
        let all_y: Vec<i32> = self.strokes.iter().flat_map(|stroke| stroke.points.iter().map(|point| point.y)).collect();
        all_y.iter().max().unwrap_or(&0) - all_y.iter().min().unwrap_or(&0)
    }

    fn x_offset(&self) -> i32 {
        let all_x: Vec<i32> = self.strokes.iter().flat_map(|stroke| stroke.points.iter().map(|point| point.x)).collect();
        *(all_x.iter().min().unwrap_or(&0))
    }

    fn y_offset(&self) -> i32 {
        let all_y: Vec<i32> = self.strokes.iter().flat_map(|stroke| stroke.points.iter().map(|point| point.y)).collect();
        *(all_y.iter().min().unwrap_or(&0))
    }
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
        .route("/boards/:id", get(serve_file))
        .with_state(state);

    let app = Router::new()
        .nest("/", api_routes)
        .route_service("/", services::ServeFile::new("assets/index.html"))
        .nest_service("/assets", services::ServeDir::new("assets"));
    
    // run it
    let port = env::var("PORT").unwrap_or("3000".into());

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

fn draw_svg(board: &mut Board) -> String {
    let mut context = Context::new();
    context.insert("width", &board.width());
    context.insert("height", &board.height());
    let strokes: Vec<TeraStroke> = board.strokes.iter().map(|s| s.renderable(board.x_offset(), board.y_offset())).collect();
    context.insert("strokes", &strokes);
    let template = Tera::new("assets/*.svg").expect("Failed to parse templates");
    template.render("template.svg", &context).expect("Failed to render template.svg")
}

async fn serve_file(
    Path(id): Path<String>,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    // `File` implements `AsyncRead`
    let mut headers = Vec::new();
    let body;
    if id.ends_with(".svg") {
        let parts: Vec<&str> = id.split('.').collect();
        if let Some(board_id) = parts.get(0) {
            let mut con = state.lock().await.get_multiplexed_async_connection().await.unwrap();
            let key = format!("board/{}", board_id);
            let result: Result<String, redis::RedisError> = con.get(&key).await;
            let mut board = serde_json::from_str::<Board>(&result.unwrap()).unwrap();
            let result = draw_svg(&mut board);
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