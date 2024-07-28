use tokio::sync::broadcast;
use axum::extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State};
use axum::response::{IntoResponse};

use std::sync::{Arc};
use futures::{SinkExt, StreamExt};


use crate::data::{AppState, RoomState, WebSocketConnect};


pub(crate) async fn handler(ws: WebSocketUpgrade,
                 State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
  let (mut sender, mut receiver) = socket.split();
  let mut username = String::new();
  let mut channel = String::new();
  let mut tx = None::<broadcast::Sender<String>>;

  while let Some(Ok(msg)) = receiver.next().await {
    if let Message::Text(name) = msg {
      let connect: WebSocketConnect = match serde_json::from_str::<WebSocketConnect>(&name) {
        Ok(connect) => connect,
        Err(err) => {
          println!("{}", &name);
          println!("{}", err);
          let _ = sender.send(Message::from("Failed to connect to room!")).await;
          break;
        }
      };
      username = connect.username.clone();
      channel = connect.channel.clone();
      let mut rooms = state.rooms.lock().await;
      let room = rooms.entry(connect.channel).or_insert_with(RoomState::new);
      tx = Some(room.tx.clone());
    }
  }
  let tx = tx.unwrap();
  let mut rx = tx.subscribe();

  // Whenever someone else sends a message, forward it to the client.
  let mut recv_messages = tokio::spawn(async move {
    while let Ok(msg) = rx.recv().await {
        if sender.send(Message::Text(msg)).await.is_err() {
            break;
        }
    }
  });

  // Whenever the connected clients sends something, we broadcast out to the tx
  let mut send_messages = {
    let tx = tx.clone();
    let name = username.clone();
    tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            let _ = tx.send(format!("{}: {}", name, text));
        }
    })
  };

  tokio::select! {
    _ = (&mut send_messages) => recv_messages.abort(),
    _ = (&mut recv_messages) => send_messages.abort(),
  };

  let left = format!("{} left the chat!", username);
  let _ = tx.send(left);
  let mut rooms = state.rooms.lock().await;
  rooms.get_mut(&channel).unwrap().users.lock().await.remove(&username);

  if rooms.get_mut(&channel).unwrap().users.lock().await.len() == 0 {
    rooms.remove(&channel);
  }

}
