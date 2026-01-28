use sqlx::{Pool, Postgres};
use serde_json::Value;
use crate::common::*;

#[derive(Debug, Clone)]
pub struct RoomEvent {
    pub event_id: String,
    pub room_id: String,
    pub r#type: String,
    pub content: Value,
    pub sender: String,
    pub unsigned: Option<Value>,
    pub redacted: bool,
    pub origin_server_ts: chrono::DateTime<chrono::Utc>,
}

pub struct EventStorage<'a> {
    pool: &'a Pool<Postgres>,
}

impl<'a> EventStorage<'a> {
    pub fn new(pool: &'a Pool<Postgres>) -> Self {
        Self { pool }
    }

    pub async fn create_event(
        &self,
        event_id: &str,
        room_id: &str,
        event_type: &str,
        content: &Value,
        sender: &str,
        origin_server_ts: chrono::DateTime<chrono::Utc>,
    ) -> Result<RoomEvent, sqlx::Error> {
        sqlx::query_as!(
            RoomEvent,
            r#"
            INSERT INTO events (event_id, room_id, type, content, sender, origin_server_ts)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
            event_id,
            room_id,
            event_type,
            content,
            sender,
            origin_server_ts
        ).fetch_one(self.pool).await
    }

    pub async fn get_event(&self, event_id: &str) -> Result<Option<RoomEvent>, sqlx::Error> {
        sqlx::query_as!(
            RoomEvent,
            r#"
            SELECT * FROM events WHERE event_id = $1
            "#,
            event_id
        ).fetch_optional(self.pool).await
    }

    pub async fn get_room_events(&self, room_id: &str, limit: i64) -> Result<Vec<RoomEvent>, sqlx::Error> {
        sqlx::query_as!(
            RoomEvent,
            r#"
            SELECT * FROM events WHERE room_id = $1
            ORDER BY origin_server_ts DESC
            LIMIT $2
            "#,
            room_id,
            limit
        ).fetch_all(self.pool).await
    }

    pub async fn get_room_events_by_type(&self, room_id: &str, event_type: &str, limit: i64) -> Result<Vec<RoomEvent>, sqlx::Error> {
        sqlx::query_as!(
            RoomEvent,
            r#"
            SELECT * FROM events WHERE room_id = $1 AND type = $2
            ORDER BY origin_server_ts DESC
            LIMIT $3
            "#,
            room_id,
            event_type,
            limit
        ).fetch_all(self.pool).await
    }

    pub async fn get_sender_events(&self, sender: &str, limit: i64) -> Result<Vec<RoomEvent>, sqlx::Error> {
        sqlx::query_as!(
            RoomEvent,
            r#"
            SELECT * FROM events WHERE sender = $1
            ORDER BY origin_server_ts DESC
            LIMIT $2
            "#,
            sender,
            limit
        ).fetch_all(self.pool).await
    }

    pub async fn get_room_message_count(&self, room_id: &str) -> Result<i64, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            SELECT COUNT(*) as count FROM events WHERE room_id = $1 AND type = 'm.room.message'
            "#,
            room_id
        ).fetch_one(self.pool).await?;
        Ok(result.count.unwrap_or(0))
    }

    pub async fn get_total_message_count(&self) -> Result<i64, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            SELECT COUNT(*) as count FROM events WHERE type = 'm.room.message'
            "#
        ).fetch_one(self.pool).await?;
        Ok(result.count.unwrap_or(0))
    }

    pub async fn redact_event(&self, event_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            UPDATE events SET redacted = TRUE, content = '{}'::jsonb WHERE event_id = $1
            "#,
            event_id
        ).execute(self.pool).await?;
        Ok(())
    }

    pub async fn delete_room_events(&self, room_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            DELETE FROM events WHERE room_id = $1
            "#,
            room_id
        ).execute(self.pool).await?;
        Ok(())
    }
}
