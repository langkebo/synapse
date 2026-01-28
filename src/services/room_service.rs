use crate::services::*;
use crate::common::*;
use serde_json::json;

pub struct RoomService<'a> {
    services: &'a ServiceContainer,
}

impl<'a> RoomService<'a> {
    pub fn new(services: &'a ServiceContainer) -> Self {
        Self { services }
    }

    pub async fn create_room(
        &self,
        user_id: &str,
        visibility: Option<&str>,
        room_alias_name: Option<&str>,
        name: Option<&str>,
        topic: Option<&str>,
        invite_list: Option<&Vec<String>>,
        preset: Option<&str>,
    ) -> ApiResult<serde_json::Value> {
        let room_id = generate_room_id(&self.services.server_name);
        let is_public = visibility.unwrap_or("private") == "public";
        let join_rule = match preset {
            Some("public_chat") => "public",
            Some("private_chat") => "invite",
            _ => "invite",
        };

        self.services.room_storage.create_room(&room_id, user_id, join_rule, "1", is_public).await
            .map_err(|e| ApiError::internal(format!("Failed to create room: {}", e)))?;

        self.services.member_storage.add_member(&room_id, user_id, "join", None, None).await
            .map_err(|e| ApiError::internal(format!("Failed to add room member: {}", e)))?;

        self.services.room_storage.increment_member_count(&room_id).await
            .map_err(|e| ApiError::internal(format!("Failed to update member count: {}", e)))?;

        if let Some(room_name) = name {
            self.services.room_storage.update_room_name(&room_id, room_name).await
                .map_err(|e| ApiError::internal(format!("Failed to update room name: {}", e)))?;
        }

        if let Some(room_topic) = topic {
            self.services.room_storage.update_room_topic(&room_id, room_topic).await
                .map_err(|e| ApiError::internal(format!("Failed to update room topic: {}", e)))?;
        }

        let room_alias = room_alias_name.map(|a| format!("#{}:{}", a, self.services.server_name));

        Ok(json!({
            "room_id": room_id,
            "room_alias": room_alias
        }))
    }

    pub async fn send_message(
        &self,
        room_id: &str,
        user_id: &str,
        message_type: &str,
        content: &serde_json::Value,
    ) -> ApiResult<serde_json::Value> {
        if !self.services.member_storage.is_member(room_id, user_id).await
            .map_err(|e| ApiError::internal(format!("Failed to check membership: {}", e)))? {
            return Err(ApiError::forbidden("You are not a member of this room".to_string()));
        }

        let event_id = generate_event_id(&self.services.server_name);
        let now = chrono::Utc::now();

        let event_content = json!({
            "type": message_type,
            "content": content
        });

        self.services.event_storage.create_event(
            &event_id,
            room_id,
            "m.room.message",
            &event_content,
            user_id,
            now,
        ).await
            .map_err(|e| ApiError::internal(format!("Failed to send message: {}", e)))?;

        Ok(json!({
            "event_id": event_id
        }))
    }

    pub async fn join_room(&self, room_id: &str, user_id: &str) -> ApiResult<()> {
        if !self.services.room_storage.room_exists(room_id).await
            .map_err(|e| ApiError::internal(format!("Failed to check room: {}", e)))? {
            return Err(ApiError::not_found("Room not found".to_string()));
        }

        self.services.member_storage.add_member(room_id, user_id, "join", None, None).await
            .map_err(|e| ApiError::internal(format!("Failed to join room: {}", e)))?;

        self.services.room_storage.increment_member_count(room_id).await
            .map_err(|e| ApiError::internal(format!("Failed to update member count: {}", e)))?;

        Ok(())
    }

    pub async fn leave_room(&self, room_id: &str, user_id: &str) -> ApiResult<()> {
        self.services.member_storage.remove_member(room_id, user_id).await
            .map_err(|e| ApiError::internal(format!("Failed to leave room: {}", e)))?;

        self.services.room_storage.decrement_member_count(room_id).await
            .map_err(|e| ApiError::internal(format!("Failed to update member count: {}", e)))?;

        Ok(())
    }

    pub async fn get_room_members(&self, room_id: &str, user_id: &str) -> ApiResult<serde_json::Value> {
        if !self.services.member_storage.is_member(room_id, user_id).await
            .map_err(|e| ApiError::internal(format!("Failed to check membership: {}", e)))? {
            return Err(ApiError::forbidden("You are not a member of this room".to_string()));
        }

        let members = self.services.member_storage.get_room_members(room_id, "join").await
            .map_err(|e| ApiError::internal(format!("Failed to get members: {}", e)))?;

        Ok(json!(members))
    }

    pub async fn get_room_state(&self, room_id: &str, user_id: &str) -> ApiResult<serde_json::Value> {
        if !self.services.member_storage.is_member(room_id, user_id).await
            .map_err(|e| ApiError::internal(format!("Failed to check membership: {}", e)))? {
            return Err(ApiError::forbidden("You are not a member of this room".to_string()));
        }

        let room = self.services.room_storage.get_room(room_id).await
            .map_err(|e| ApiError::internal(format!("Failed to get room: {}", e)))?;

        match room {
            Some(r) => Ok(json!({
                "room_id": r.room_id,
                "name": r.name,
                "topic": r.topic,
                "canonical_alias": r.canonical_alias,
                "is_public": r.is_public,
                "member_count": r.member_count,
                "creator": r.creator
            })),
            None => Err(ApiError::not_found("Room not found".to_string()))
        }
    }

    pub async fn get_user_rooms(&self, user_id: &str) -> ApiResult<serde_json::Value> {
        let room_ids = self.services.member_storage.get_joined_rooms(user_id).await
            .map_err(|e| ApiError::internal(format!("Failed to get rooms: {}", e)))?;

        let mut rooms = Vec::new();
        for room_id in room_ids {
            if let Ok(Some(room)) = self.services.room_storage.get_room(&room_id).await {
                rooms.push(json!({
                    "room_id": room.room_id,
                    "name": room.name,
                    "topic": room.topic,
                    "is_public": room.is_public,
                    "member_count": room.member_count
                }));
            }
        }

        Ok(json!(rooms))
    }
}
