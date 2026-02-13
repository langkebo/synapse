use bytes::{BufMut, BytesMut};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::io;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, info};

pub const PROTOCOL_VERSION: u8 = 1;
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;
pub const MAGIC_BYTES: &[u8; 4] = b"SYRP";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    Ping = 0,
    Pong = 1,
    Event = 2,
    Command = 3,
    Response = 4,
    Error = 5,
    Subscribe = 6,
    Unsubscribe = 7,
    StreamPosition = 8,
    Heartbeat = 9,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationMessage {
    pub msg_type: MessageType,
    pub msg_id: u64,
    pub timestamp: i64,
    pub payload: Vec<u8>,
}

impl ReplicationMessage {
    pub fn new(msg_type: MessageType, payload: Vec<u8>) -> Self {
        Self {
            msg_type,
            msg_id: rand::random(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            payload,
        }
    }

    pub fn ping() -> Self {
        Self::new(MessageType::Ping, vec![])
    }

    pub fn pong() -> Self {
        Self::new(MessageType::Pong, vec![])
    }

    pub fn heartbeat() -> Self {
        Self::new(MessageType::Heartbeat, vec![])
    }

    pub fn event<T: Serialize>(event: &T) -> Result<Self, ReplicationError> {
        let payload = serde_json::to_vec(event)?;
        Ok(Self::new(MessageType::Event, payload))
    }

    pub fn command<T: Serialize>(cmd: &T) -> Result<Self, ReplicationError> {
        let payload = serde_json::to_vec(cmd)?;
        Ok(Self::new(MessageType::Command, payload))
    }

    pub fn response<T: Serialize>(msg_id: u64, response: &T) -> Result<Self, ReplicationError> {
        let payload = serde_json::to_vec(response)?;
        let mut msg = Self::new(MessageType::Response, payload);
        msg.msg_id = msg_id;
        Ok(msg)
    }

    pub fn error(msg_id: u64, error_msg: &str) -> Self {
        let mut msg = Self::new(MessageType::Error, error_msg.as_bytes().to_vec());
        msg.msg_id = msg_id;
        msg
    }

    pub fn parse_payload<T: DeserializeOwned>(&self) -> Result<T, ReplicationError> {
        Ok(serde_json::from_slice(&self.payload)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPayload {
    pub event_id: String,
    pub room_id: String,
    pub event_type: String,
    pub sender: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPayload {
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamPosition {
    pub stream_name: String,
    pub position: u64,
    pub timestamp: i64,
}

pub struct MessageCodec;

impl MessageCodec {
    pub fn encode(msg: &ReplicationMessage) -> Result<Vec<u8>, ReplicationError> {
        let payload = serde_json::to_vec(msg)?;
        
        let mut buffer = BytesMut::with_capacity(4 + 1 + 8 + 4 + payload.len());
        
        buffer.put_slice(MAGIC_BYTES);
        buffer.put_u8(PROTOCOL_VERSION);
        buffer.put_u64(msg.msg_id);
        buffer.put_u32(payload.len() as u32);
        buffer.put_slice(&payload);
        
        Ok(buffer.to_vec())
    }

    pub fn decode(data: &[u8]) -> Result<ReplicationMessage, ReplicationError> {
        if data.len() < 17 {
            return Err(ReplicationError::InvalidMessage("Message too short".to_string()));
        }

        if &data[0..4] != MAGIC_BYTES {
            return Err(ReplicationError::InvalidMessage("Invalid magic bytes".to_string()));
        }

        let version = data[4];
        if version != PROTOCOL_VERSION {
            return Err(ReplicationError::VersionMismatch(version, PROTOCOL_VERSION));
        }

        let msg_id = u64::from_be_bytes([data[5], data[6], data[7], data[8], data[9], data[10], data[11], data[12]]);
        let payload_len = u32::from_be_bytes([data[13], data[14], data[15], data[16]]) as usize;

        if payload_len > MAX_MESSAGE_SIZE {
            return Err(ReplicationError::MessageTooLarge(payload_len, MAX_MESSAGE_SIZE));
        }

        if data.len() < 17 + payload_len {
            return Err(ReplicationError::InvalidMessage("Incomplete message".to_string()));
        }

        let payload = &data[17..17 + payload_len];
        let mut msg: ReplicationMessage = serde_json::from_slice(payload)?;
        msg.msg_id = msg_id;

        Ok(msg)
    }
}

pub struct ReplicationConnection {
    stream: TcpStream,
    read_buffer: BytesMut,
    #[allow(dead_code)]
    write_buffer: BytesMut,
}

impl ReplicationConnection {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            read_buffer: BytesMut::with_capacity(64 * 1024),
            write_buffer: BytesMut::with_capacity(64 * 1024),
        }
    }

    pub async fn send(&mut self, msg: &ReplicationMessage) -> Result<(), ReplicationError> {
        let encoded = MessageCodec::encode(msg)?;
        self.stream.write_all(&encoded).await?;
        self.stream.flush().await?;
        debug!(msg_type = ?msg.msg_type, msg_id = msg.msg_id, "Message sent");
        Ok(())
    }

    pub async fn receive(&mut self) -> Result<Option<ReplicationMessage>, ReplicationError> {
        let mut temp_buf = [0u8; 4096];
        
        loop {
            if self.read_buffer.len() >= 17 {
                let payload_len = u32::from_be_bytes([
                    self.read_buffer[13],
                    self.read_buffer[14],
                    self.read_buffer[15],
                    self.read_buffer[16],
                ]) as usize;

                if self.read_buffer.len() >= 17 + payload_len {
                    let data = self.read_buffer.split_to(17 + payload_len);
                    let msg = MessageCodec::decode(&data)?;
                    debug!(msg_type = ?msg.msg_type, msg_id = msg.msg_id, "Message received");
                    return Ok(Some(msg));
                }
            }

            let n = self.stream.read(&mut temp_buf).await?;
            if n == 0 {
                return Ok(None);
            }
            self.read_buffer.extend_from_slice(&temp_buf[..n]);
        }
    }

    pub async fn close(&mut self) -> Result<(), ReplicationError> {
        self.stream.shutdown().await?;
        Ok(())
    }
}

pub struct ReplicationServer {
    addr: SocketAddr,
}

impl ReplicationServer {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }

    pub async fn start(&self) -> Result<TcpListener, ReplicationError> {
        let listener = TcpListener::bind(self.addr).await?;
        info!(addr = %self.addr, "Replication server started");
        Ok(listener)
    }

    pub async fn accept_connection(listener: &TcpListener) -> Result<ReplicationConnection, ReplicationError> {
        let (stream, addr) = listener.accept().await?;
        info!(peer_addr = %addr, "New replication connection accepted");
        Ok(ReplicationConnection::new(stream))
    }
}

pub struct ReplicationClient;

impl ReplicationClient {
    pub async fn connect(addr: SocketAddr) -> Result<ReplicationConnection, ReplicationError> {
        let stream = TcpStream::connect(addr).await?;
        info!(addr = %addr, "Connected to replication server");
        Ok(ReplicationConnection::new(stream))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReplicationError {
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Invalid message: {0}")]
    InvalidMessage(String),
    #[error("Version mismatch: got {0}, expected {1}")]
    VersionMismatch(u8, u8),
    #[error("Message too large: {0} bytes, max {1}")]
    MessageTooLarge(usize, usize),
    #[error("Connection closed")]
    ConnectionClosed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_codec_roundtrip() {
        let msg = ReplicationMessage::ping();
        let encoded = MessageCodec::encode(&msg).unwrap();
        let decoded = MessageCodec::decode(&encoded).unwrap();
        
        assert_eq!(decoded.msg_type, MessageType::Ping);
        assert_eq!(decoded.msg_id, msg.msg_id);
    }

    #[test]
    fn test_event_message() {
        let event = EventPayload {
            event_id: "$event1".to_string(),
            room_id: "!room1:example.com".to_string(),
            event_type: "m.room.message".to_string(),
            sender: "@user:example.com".to_string(),
            content: b"hello".to_vec(),
        };

        let msg = ReplicationMessage::event(&event).unwrap();
        assert_eq!(msg.msg_type, MessageType::Event);

        let decoded_event: EventPayload = msg.parse_payload().unwrap();
        assert_eq!(decoded_event.event_id, event.event_id);
    }

    #[test]
    fn test_command_message() {
        let cmd = CommandPayload {
            command: "sync".to_string(),
            args: vec!["room1".to_string()],
        };

        let msg = ReplicationMessage::command(&cmd).unwrap();
        assert_eq!(msg.msg_type, MessageType::Command);

        let decoded_cmd: CommandPayload = msg.parse_payload().unwrap();
        assert_eq!(decoded_cmd.command, "sync");
    }

    #[test]
    fn test_invalid_magic_bytes() {
        let data = b"XXXX\x01\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x00";
        let result = MessageCodec::decode(data);
        assert!(matches!(result, Err(ReplicationError::InvalidMessage(_))));
    }

    #[test]
    fn test_version_mismatch() {
        let mut data = vec![b'S', b'Y', b'R', b'P', 99];
        data.extend_from_slice(&[0u8; 12]);
        let result = MessageCodec::decode(&data);
        assert!(matches!(result, Err(ReplicationError::VersionMismatch(99, 1))));
    }

    #[test]
    fn test_message_too_large() {
        let payload_len = (MAX_MESSAGE_SIZE + 1) as u32;
        let mut data = vec![b'S', b'Y', b'R', b'P', 1];
        data.extend_from_slice(&[0u8; 8]);
        data.extend_from_slice(&payload_len.to_be_bytes());
        data.extend_from_slice(&vec![0u8; MAX_MESSAGE_SIZE + 100]);
        
        let result = MessageCodec::decode(&data);
        assert!(matches!(result, Err(ReplicationError::MessageTooLarge(_, _))));
    }

    #[tokio::test]
    async fn test_tcp_connection() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = ReplicationConnection::new(stream);
            
            let msg = conn.receive().await.unwrap().unwrap();
            assert_eq!(msg.msg_type, MessageType::Ping);
            
            conn.send(&ReplicationMessage::pong()).await.unwrap();
        });

        let client_handle = tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            let mut conn = ReplicationClient::connect(addr).await.unwrap();
            
            conn.send(&ReplicationMessage::ping()).await.unwrap();
            
            let response = conn.receive().await.unwrap().unwrap();
            assert_eq!(response.msg_type, MessageType::Pong);
        });

        let _ = tokio::try_join!(server_handle, client_handle);
    }
}
