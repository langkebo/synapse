use crate::services::*;
use crate::common::*;

pub struct SyncService<'a> {
    services: &'a ServiceContainer,
}

impl<'a> SyncService<'a> {
    pub fn new(services: &'a ServiceContainer) -> Self {
        Self { services }
    }

    pub async fn sync(
        &self,
        user_id: &str,
        timeout: u64,
        full_state: bool,
        set_presence: &str,
    ) -> ApiResult<serde_json::Value> {
        self.services.presence_storage.set_presence(user_id, set_presence, None).await
            .ok();

        let room_ids = self.services.member_storage.get_joined_rooms(user_id).await
            .map_err(|e| ApiError::internal(format!("Failed to get rooms: {}", e)))?;

        let mut rooms = serde_json::Map::new();
        for room_id in room_ids {
            let events = self.services.event_storage.get_room_events(&room_id, 20).await
                .map_err(|e| ApiError::internal(format!("Failed to get events: {}", e)))?;

            let event_list: Vec<serde_json::Value> = events.iter().map(|e| json!({
                "type": e.r#type,
                "content": e.content,
                "sender": e.sender,
                "origin_server_ts": e.origin_server_ts.timestamp_millis(),
                "event_id": e.event_id,
                "unsigned": e.unsigned
            })).collect();

            let prev_batch = events.first()
                .map(|e| format!("t{}", e.origin_server_ts.timestamp_millis()))
                .unwrap_or_else(|| format!("t{}", chrono::Utc::now().timestamp_millis()));

            rooms.insert(room_id, json!({
                "timeline": {
                    "events": event_list,
                    "limited": true,
                    "prev_batch": prev_batch
                },
                "state": json!({}),
                "ephemeral": json!({}),
                "account_data": json!({}),
                "unread_notifications": json!({
                    "highlight_count": 0,
                    "notification_count": 0
                })
            }));
        }

        Ok(json!({
            "next_batch": format!("s{}", chrono::Utc::now().timestamp_millis()),
            "rooms": rooms,
            "presence": json!({
                "events": []
            }),
            "account_data": json!({
                "events": []
            }),
            "to_device": json!({
                "events": []
            })
        }))
    }

    pub async fn get_room_messages(
        &self,
        room_id: &str,
        user_id: &str,
        from: &str,
        limit: i64,
        dir: &str,
    ) -> ApiResult<serde_json::Value> {
        if !self.services.member_storage.is_member(room_id, user_id).await
            .map_err(|e| ApiError::internal(format!("Failed to check membership: {}", e)))? {
            return Err(ApiError::forbidden("You are not a member of this room".to_string()));
        }

        let events = self.services.event_storage.get_room_events(room_id, limit).await
            .map_err(|e| ApiError::internal(format!("Failed to get messages: {}", e)))?;

        let event_list: Vec<serde_json::Value> = events.iter().map(|e| json!({
            "type": e.r#type,
            "content": e.content,
            "sender": e.sender,
            "origin_server_ts": e.origin_server_ts.timestamp_millis(),
            "event_id": e.event_id,
            "unsigned": e.unsigned
        })).collect();

        Ok(json!({
            "chunk": event_list,
            "start": from,
            "end": format!("e{}", chrono::Utc::now().timestamp_millis())
        }))
    }

    pub async fn get_public_rooms(&self, limit: i64, since: Option<&str>) -> ApiResult<serde_json::Value> {
        let rooms = self.services.room_storage.get_public_rooms(limit).await
            .map_err(|e| ApiError::internal(format!("Failed to get public rooms: {}", e)))?;

        let room_list: Vec<serde_json::Value> = rooms.iter().map(|r| json!({
            "room_id": r.room_id,
            "name": r.name,
            "topic": r.topic,
            "canonical_alias": r.canonical_alias,
            "is_public": r.is_public,
            "member_count": r.member_count
        })).collect();

        Ok(json!({
            "chunk": room_list,
            "total_room_count_estimate": room_list.len() as i64
        }))
    }
}
