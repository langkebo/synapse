use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Space {
    pub space_id: String,
    pub name: String,
    pub topic: Option<String>,
    pub avatar_url: Option<String>,
    pub creator: String,
    pub is_public: bool,
    pub join_rule: SpaceJoinRule,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SpaceJoinRule {
    Public,
    Knock,
    Invite,
    Restricted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceChild {
    pub space_id: String,
    pub child_room_id: String,
    pub order: Option<String>,
    pub suggested: bool,
    pub via_servers: Vec<String>,
    pub added_by: String,
    pub added_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceHierarchyRoomSummary {
    pub room_id: String,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub avatar_url: Option<String>,
    pub num_joined_members: i64,
    pub room_type: Option<RoomType>,
    pub is_space: bool,
    pub join_rule: SpaceJoinRule,
    pub world_readable: bool,
    pub guest_can_join: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RoomType {
    Space,
    Room,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceHierarchy {
    pub space: Space,
    pub children: Vec<SpaceHierarchyChild>,
    pub next_batch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceHierarchyChild {
    pub room_id: String,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub avatar_url: Option<String>,
    pub num_joined_members: i64,
    pub room_type: Option<RoomType>,
    pub via_servers: Vec<String>,
    pub order: Option<String>,
    pub suggested: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSpaceRequest {
    pub name: String,
    pub topic: Option<String>,
    pub avatar_url: Option<String>,
    pub is_public: bool,
    pub join_rule: SpaceJoinRule,
    pub initial_children: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddChildRequest {
    pub child_room_id: String,
    pub order: Option<String>,
    pub suggested: bool,
    pub via_servers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceSummary {
    pub space_id: String,
    pub name: String,
    pub topic: Option<String>,
    pub avatar_url: Option<String>,
    pub member_count: i64,
    pub child_count: i64,
}

pub struct SpaceManager {
    spaces: Arc<RwLock<HashMap<String, Space>>>,
    children: Arc<RwLock<HashMap<String, Vec<SpaceChild>>>>,
    user_spaces: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl SpaceManager {
    pub fn new() -> Self {
        Self {
            spaces: Arc::new(RwLock::new(HashMap::new())),
            children: Arc::new(RwLock::new(HashMap::new())),
            user_spaces: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_space(
        &self,
        creator: &str,
        request: CreateSpaceRequest,
    ) -> Result<Space, SpaceError> {
        if request.name.is_empty() || request.name.len() > 255 {
            return Err(SpaceError::InvalidName);
        }

        let space_id = format!("!space_{}:{}", uuid::Uuid::new_v4(), "example.com");
        let now = chrono::Utc::now().timestamp_millis();

        let space = Space {
            space_id: space_id.clone(),
            name: request.name,
            topic: request.topic,
            avatar_url: request.avatar_url,
            creator: creator.to_string(),
            is_public: request.is_public,
            join_rule: request.join_rule,
            created_at: now,
            updated_at: now,
        };

        self.spaces.write().await.insert(space_id.clone(), space.clone());

        self.user_spaces
            .write()
            .await
            .entry(creator.to_string())
            .or_default()
            .push(space_id.clone());

        for child_room_id in request.initial_children {
            let child = SpaceChild {
                space_id: space_id.clone(),
                child_room_id,
                order: None,
                suggested: false,
                via_servers: vec!["example.com".to_string()],
                added_by: creator.to_string(),
                added_at: now,
            };
            self.children
                .write()
                .await
                .entry(space_id.clone())
                .or_default()
                .push(child);
        }

        Ok(space)
    }

    pub async fn get_space(&self, space_id: &str) -> Option<Space> {
        self.spaces.read().await.get(space_id).cloned()
    }

    pub async fn add_child(
        &self,
        space_id: &str,
        added_by: &str,
        request: AddChildRequest,
    ) -> Result<SpaceChild, SpaceError> {
        let spaces = self.spaces.read().await;
        if !spaces.contains_key(space_id) {
            return Err(SpaceError::SpaceNotFound);
        }
        drop(spaces);

        let now = chrono::Utc::now().timestamp_millis();
        let child = SpaceChild {
            space_id: space_id.to_string(),
            child_room_id: request.child_room_id,
            order: request.order,
            suggested: request.suggested,
            via_servers: request.via_servers,
            added_by: added_by.to_string(),
            added_at: now,
        };

        self.children
            .write()
            .await
            .entry(space_id.to_string())
            .or_default()
            .push(child.clone());

        Ok(child)
    }

    pub async fn remove_child(&self, space_id: &str, child_room_id: &str) -> Result<(), SpaceError> {
        let mut children = self.children.write().await;
        if let Some(space_children) = children.get_mut(space_id) {
            space_children.retain(|c| c.child_room_id != child_room_id);
            Ok(())
        } else {
            Err(SpaceError::SpaceNotFound)
        }
    }

    pub async fn get_children(&self, space_id: &str) -> Vec<SpaceChild> {
        self.children
            .read()
            .await
            .get(space_id)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn get_hierarchy(
        &self,
        space_id: &str,
        _max_depth: usize,
        limit: usize,
    ) -> Option<SpaceHierarchy> {
        let space = self.get_space(space_id).await?;
        let children = self.get_children(space_id).await;

        let hierarchy_children: Vec<SpaceHierarchyChild> = children
            .into_iter()
            .take(limit)
            .map(|c| SpaceHierarchyChild {
                room_id: c.child_room_id,
                name: None,
                topic: None,
                avatar_url: None,
                num_joined_members: 0,
                room_type: Some(RoomType::Room),
                via_servers: c.via_servers,
                order: c.order,
                suggested: c.suggested,
            })
            .collect();

        Some(SpaceHierarchy {
            space,
            children: hierarchy_children,
            next_batch: None,
        })
    }

    pub async fn get_user_spaces(&self, user_id: &str) -> Vec<SpaceSummary> {
        let user_spaces = self
            .user_spaces
            .read()
            .await
            .get(user_id)
            .cloned()
            .unwrap_or_default();

        let spaces = self.spaces.read().await;
        let children = self.children.read().await;

        user_spaces
            .into_iter()
            .filter_map(|space_id| {
                spaces.get(&space_id).map(|space| SpaceSummary {
                    space_id: space.space_id.clone(),
                    name: space.name.clone(),
                    topic: space.topic.clone(),
                    avatar_url: space.avatar_url.clone(),
                    member_count: 1,
                    child_count: children.get(&space_id).map(|c| c.len() as i64).unwrap_or(0),
                })
            })
            .collect()
    }

    pub async fn update_space(
        &self,
        space_id: &str,
        name: Option<String>,
        topic: Option<String>,
        avatar_url: Option<String>,
    ) -> Result<Space, SpaceError> {
        let mut spaces = self.spaces.write().await;
        if let Some(space) = spaces.get_mut(space_id) {
            if let Some(n) = name {
                if n.is_empty() || n.len() > 255 {
                    return Err(SpaceError::InvalidName);
                }
                space.name = n;
            }
            if let Some(t) = topic {
                space.topic = Some(t);
            }
            if let Some(a) = avatar_url {
                space.avatar_url = Some(a);
            }
            space.updated_at = chrono::Utc::now().timestamp_millis();
            Ok(space.clone())
        } else {
            Err(SpaceError::SpaceNotFound)
        }
    }

    pub async fn delete_space(&self, space_id: &str) -> Result<(), SpaceError> {
        let mut spaces = self.spaces.write().await;
        if spaces.remove(space_id).is_some() {
            drop(spaces);
            self.children.write().await.remove(space_id);
            Ok(())
        } else {
            Err(SpaceError::SpaceNotFound)
        }
    }
}

impl Default for SpaceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SpaceError {
    #[error("Space not found")]
    SpaceNotFound,
    #[error("Invalid space name")]
    InvalidName,
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Child already exists")]
    ChildExists,
    #[error("Invalid child room")]
    InvalidChild,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_space() {
        let manager = SpaceManager::new();
        let request = CreateSpaceRequest {
            name: "Test Space".to_string(),
            topic: Some("A test space".to_string()),
            avatar_url: None,
            is_public: true,
            join_rule: SpaceJoinRule::Public,
            initial_children: vec![],
        };

        let space = manager.create_space("@alice:example.com", request).await;
        assert!(space.is_ok());
        let space = space.unwrap();
        assert_eq!(space.name, "Test Space");
        assert!(space.space_id.starts_with("!space_"));
    }

    #[tokio::test]
    async fn test_add_child() {
        let manager = SpaceManager::new();
        let create_req = CreateSpaceRequest {
            name: "Parent Space".to_string(),
            topic: None,
            avatar_url: None,
            is_public: true,
            join_rule: SpaceJoinRule::Public,
            initial_children: vec![],
        };

        let space = manager.create_space("@alice:example.com", create_req).await.unwrap();

        let add_req = AddChildRequest {
            child_room_id: "!child:example.com".to_string(),
            order: Some("001".to_string()),
            suggested: true,
            via_servers: vec!["example.com".to_string()],
        };

        let result = manager.add_child(&space.space_id, "@alice:example.com", add_req).await;
        assert!(result.is_ok());

        let children = manager.get_children(&space.space_id).await;
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].child_room_id, "!child:example.com");
    }

    #[tokio::test]
    async fn test_get_hierarchy() {
        let manager = SpaceManager::new();
        let create_req = CreateSpaceRequest {
            name: "Root Space".to_string(),
            topic: None,
            avatar_url: None,
            is_public: true,
            join_rule: SpaceJoinRule::Public,
            initial_children: vec!["!child1:example.com".to_string()],
        };

        let space = manager.create_space("@alice:example.com", create_req).await.unwrap();

        let hierarchy = manager.get_hierarchy(&space.space_id, 1, 10).await;
        assert!(hierarchy.is_some());

        let hierarchy = hierarchy.unwrap();
        assert_eq!(hierarchy.space.name, "Root Space");
        assert_eq!(hierarchy.children.len(), 1);
    }

    #[tokio::test]
    async fn test_invalid_name() {
        let manager = SpaceManager::new();
        let request = CreateSpaceRequest {
            name: "".to_string(),
            topic: None,
            avatar_url: None,
            is_public: true,
            join_rule: SpaceJoinRule::Public,
            initial_children: vec![],
        };

        let result = manager.create_space("@alice:example.com", request).await;
        assert!(matches!(result, Err(SpaceError::InvalidName)));
    }
}
