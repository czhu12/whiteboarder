use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::sync::{broadcast};
use serde_json::Value;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Point {
    pub x: i32,
    pub y: i32
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Stroke {
    pub timestamp: i64,
    pub color: String,
    pub size: i32,
    pub points: Vec<Point>,
}

impl Stroke {
    pub fn renderable(&self, x_offset: i32, y_offset: i32) -> TeraStroke {
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
pub struct TeraStroke {
    pub color: String,
    pub size: i32,
    pub points: Vec<Point>,
    pub polyline: String,
}

impl TeraStroke {
    pub fn new(color: String, size: i32, points: Vec<Point>, x_offset: i32, y_offset: i32) -> Self {
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Board {
    pub id: String,
    pub strokes: Vec<Stroke>
}

impl Board {
    pub fn width(&self) -> i32 {
        let all_x: Vec<i32> = self.strokes.iter().flat_map(|stroke| stroke.points.iter().map(|point| point.x)).collect();
        all_x.iter().max().unwrap_or(&0) - all_x.iter().min().unwrap_or(&0)
    }
    pub fn height(&self) -> i32 {
        let all_y: Vec<i32> = self.strokes.iter().flat_map(|stroke| stroke.points.iter().map(|point| point.y)).collect();
        all_y.iter().max().unwrap_or(&0) - all_y.iter().min().unwrap_or(&0)
    }

    pub fn x_offset(&self) -> i32 {
        let all_x: Vec<i32> = self.strokes.iter().flat_map(|stroke| stroke.points.iter().map(|point| point.x)).collect();
        *(all_x.iter().min().unwrap_or(&0))
    }

    pub fn y_offset(&self) -> i32 {
        let all_y: Vec<i32> = self.strokes.iter().flat_map(|stroke| stroke.points.iter().map(|point| point.y)).collect();
        *(all_y.iter().min().unwrap_or(&0))
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ErrorResponse {
    pub error: String,
}

pub struct AppState {
    pub rooms: Mutex<HashMap<String, RoomState>>,
    pub redis_client: Mutex<redis::Client>
}

pub struct RoomState {
    pub users: Mutex<HashSet<String>>,
    pub tx: broadcast::Sender<String>,
}

impl RoomState {
    pub fn new() -> Self {
        Self {
            users: Mutex::new(HashSet::new()),
            tx: broadcast::channel(69).0,
        }
    }
}

#[derive(Deserialize)]
pub struct WebSocketConnect {
    pub username: String,
    pub channel: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub messagetype: String,
    pub channel: String,
    pub payload: WebSocketPayload,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "messagetype", content = "payload")]
pub enum WebSocketPayload {
    #[serde(rename = "cursor")]
    Cursor(CursorPayload),

    #[serde(rename = "board")]
    BoardUpdate(Board),

    #[serde(rename = "unknown")]
    Unknown(Value), // Fallback for unknown message types
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CursorPayload {
    username: String,
    x: i32,
    y: i32,
}

