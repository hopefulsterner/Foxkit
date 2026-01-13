//! Collaboration client - WebSocket communication

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use futures::{StreamExt, SinkExt};
use anyhow::Result;

use crate::{UserId, RoomId, CursorPosition};
use crate::protocol::{Message, ClientMessage, ServerMessage};
use crate::room::RoomInfo;

/// Collaboration client
pub struct CollabClient {
    user_id: UserId,
    /// WebSocket sender
    ws_tx: mpsc::UnboundedSender<WsMessage>,
    /// Message receiver
    msg_rx: Arc<RwLock<mpsc::UnboundedReceiver<ServerMessage>>>,
}

impl CollabClient {
    /// Connect to collaboration server
    pub async fn connect(url: &str, user_id: UserId) -> Result<Self> {
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();
        
        // Channel for outgoing messages
        let (ws_tx, mut ws_rx) = mpsc::unbounded_channel::<WsMessage>();
        
        // Channel for incoming messages
        let (msg_tx, msg_rx) = mpsc::unbounded_channel::<ServerMessage>();
        
        // Spawn writer task
        tokio::spawn(async move {
            while let Some(msg) = ws_rx.recv().await {
                if write.send(msg).await.is_err() {
                    break;
                }
            }
        });
        
        // Spawn reader task
        tokio::spawn(async move {
            while let Some(Ok(msg)) = read.next().await {
                if let WsMessage::Text(text) = msg {
                    if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                        if msg_tx.send(server_msg).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        
        // Send auth message
        let auth_msg = ClientMessage::Auth { user_id };
        let ws_msg = WsMessage::Text(serde_json::to_string(&auth_msg)?);
        ws_tx.send(ws_msg)?;
        
        Ok(Self {
            user_id,
            ws_tx,
            msg_rx: Arc::new(RwLock::new(msg_rx)),
        })
    }

    /// Create a new room
    pub async fn create_room(&self, name: &str) -> Result<RoomId> {
        let msg = ClientMessage::CreateRoom { name: name.to_string() };
        self.send(msg)?;
        
        // Wait for response
        let mut rx = self.msg_rx.write().await;
        while let Some(msg) = rx.recv().await {
            if let ServerMessage::RoomCreated { room_id } = msg {
                return Ok(room_id);
            }
        }
        
        anyhow::bail!("Failed to create room")
    }

    /// Join an existing room
    pub async fn join_room(&self, room_id: RoomId) -> Result<RoomInfo> {
        let msg = ClientMessage::JoinRoom { room_id };
        self.send(msg)?;
        
        // Wait for response
        let mut rx = self.msg_rx.write().await;
        while let Some(msg) = rx.recv().await {
            if let ServerMessage::RoomJoined { room_info } = msg {
                return Ok(room_info);
            }
            if let ServerMessage::Error { message } = msg {
                anyhow::bail!("Failed to join room: {}", message);
            }
        }
        
        anyhow::bail!("Failed to join room")
    }

    /// Leave a room
    pub async fn leave_room(&self, room_id: RoomId) -> Result<()> {
        let msg = ClientMessage::LeaveRoom { room_id };
        self.send(msg)
    }

    /// Share a file with room
    pub async fn share_file(&self, room_id: RoomId, file_path: &str) -> Result<()> {
        let msg = ClientMessage::ShareFile {
            room_id,
            file_path: file_path.to_string(),
        };
        self.send(msg)
    }

    /// Update cursor position
    pub async fn update_cursor(&self, room_id: RoomId, file: &str, position: CursorPosition) -> Result<()> {
        let msg = ClientMessage::CursorUpdate {
            room_id,
            file: file.to_string(),
            position,
        };
        self.send(msg)
    }

    /// Send an operation
    pub async fn send_operation(&self, room_id: RoomId, file: &str, operation: crate::protocol::Operation) -> Result<()> {
        let msg = ClientMessage::Operation {
            room_id,
            file: file.to_string(),
            operation,
        };
        self.send(msg)
    }

    /// Disconnect from server
    pub async fn disconnect(&self) -> Result<()> {
        let msg = ClientMessage::Disconnect;
        self.send(msg)?;
        Ok(())
    }

    fn send(&self, msg: ClientMessage) -> Result<()> {
        let text = serde_json::to_string(&msg)?;
        self.ws_tx.send(WsMessage::Text(text))
            .map_err(|e| anyhow::anyhow!("Failed to send: {}", e))
    }
}
