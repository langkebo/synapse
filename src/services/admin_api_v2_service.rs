use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Admin API version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminApiVersion {
    V1,
    V2,
}

/// Admin user info (v2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminUserInfo {
    pub user_id: String,
    pub displayname: Option<String>,
    pub avatar_url: Option<String>,
    pub creation_ts: i64,
    pub last_seen_ts: Option<i64>,
    pub last_seen_ip: Option<String>,
    pub deactivated: bool,
    pub suspended: bool,
    pub erased: bool,
    pub admin: bool,
    pub guest: bool,
    pub user_type: Option<String>,
    pub consent_version: Option<String>,
    pub consent_server_notice_sent: Option<String>,
    pub appservice_id: Option<String>,
}

/// Admin room info (v2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminRoomInfo {
    pub room_id: String,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub avatar_url: Option<String>,
    pub canonical_alias: Option<String>,
    pub join_rules: String,
    pub room_version: String,
    pub creator: String,
    pub creation_ts: i64,
    pub joined_members: u64,
    pub invited_members: u64,
    pub banned_members: u64,
    pub state_events: u64,
    pub public: bool,
    pub federatable: bool,
    pub encryption: Option<String>,
}

/// Admin server info (v2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminServerInfo {
    pub server_name: String,
    pub version: String,
    pub python_version: Option<String>,
    pub uptime_seconds: u64,
    pub total_users: u64,
    pub total_rooms: u64,
    pub total_events: u64,
    pub monthly_active_users: u64,
    pub registered_users: u64,
    pub daily_active_users: u64,
    pub daily_sent_messages: u64,
    pub daily_sent_e2ee_messages: u64,
}

/// Batch operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperationResult {
    pub operation_id: String,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub status: BatchOperationStatus,
    pub total_items: u64,
    pub processed_items: u64,
    pub successful_items: u64,
    pub failed_items: u64,
    pub errors: Vec<BatchOperationError>,
}

impl BatchOperationResult {
    pub fn new(operation_id: String, total_items: u64) -> Self {
        Self {
            operation_id,
            started_at: Utc::now().timestamp_millis(),
            completed_at: None,
            status: BatchOperationStatus::Pending,
            total_items,
            processed_items: 0,
            successful_items: 0,
            failed_items: 0,
            errors: Vec::new(),
        }
    }

    pub fn increment_success(&mut self) {
        self.processed_items += 1;
        self.successful_items += 1;
        self.check_completion();
    }

    pub fn increment_failure(&mut self, error: BatchOperationError) {
        self.processed_items += 1;
        self.failed_items += 1;
        self.errors.push(error);
        self.check_completion();
    }

    fn check_completion(&mut self) {
        if self.processed_items >= self.total_items {
            self.status = if self.failed_items == 0 {
                BatchOperationStatus::Completed
            } else if self.successful_items == 0 {
                BatchOperationStatus::Failed
            } else {
                BatchOperationStatus::Partial
            };
            self.completed_at = Some(Utc::now().timestamp_millis());
        }
    }
}

/// Batch operation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatchOperationStatus {
    Pending,
    InProgress,
    Completed,
    Partial,
    Failed,
    Cancelled,
}

/// Batch operation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperationError {
    pub item: String,
    pub error: String,
    pub code: Option<String>,
}

/// User role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum UserRole {
    #[default]
    User,
    Moderator,
    Admin,
    ServerAdmin,
}


/// User role assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRoleAssignment {
    pub user_id: String,
    pub role: UserRole,
    pub assigned_by: String,
    pub assigned_at: i64,
    pub scope: Option<String>,
}

impl UserRoleAssignment {
    pub fn new(user_id: String, role: UserRole, assigned_by: String) -> Self {
        Self {
            user_id,
            role,
            assigned_by,
            assigned_at: Utc::now().timestamp_millis(),
            scope: None,
        }
    }
}

/// Room transfer request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomTransferRequest {
    pub room_id: String,
    pub new_owner: String,
    pub force: bool,
    pub notify_users: bool,
}

/// Room transfer result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomTransferResult {
    pub room_id: String,
    pub old_owner: String,
    pub new_owner: String,
    pub transferred_at: i64,
    pub success: bool,
    pub message: Option<String>,
}

/// Admin API v2 service
pub struct AdminApiV2Service {
    users: Arc<RwLock<HashMap<String, AdminUserInfo>>>,
    rooms: Arc<RwLock<HashMap<String, AdminRoomInfo>>>,
    roles: Arc<RwLock<HashMap<String, UserRoleAssignment>>>,
    batch_operations: Arc<RwLock<HashMap<String, BatchOperationResult>>>,
    server_info: Arc<RwLock<AdminServerInfo>>,
}

impl AdminApiV2Service {
    pub fn new(server_name: String) -> Self {
        let server_info = AdminServerInfo {
            server_name,
            version: "0.1.0".to_string(),
            python_version: None,
            uptime_seconds: 0,
            total_users: 0,
            total_rooms: 0,
            total_events: 0,
            monthly_active_users: 0,
            registered_users: 0,
            daily_active_users: 0,
            daily_sent_messages: 0,
            daily_sent_e2ee_messages: 0,
        };
        
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            rooms: Arc::new(RwLock::new(HashMap::new())),
            roles: Arc::new(RwLock::new(HashMap::new())),
            batch_operations: Arc::new(RwLock::new(HashMap::new())),
            server_info: Arc::new(RwLock::new(server_info)),
        }
    }

    pub async fn get_user(&self, user_id: &str) -> Option<AdminUserInfo> {
        self.users.read().await.get(user_id).cloned()
    }

    pub async fn get_users(&self, limit: usize, from: Option<&str>) -> Vec<AdminUserInfo> {
        let users = self.users.read().await;
        let mut result: Vec<_> = users.values().cloned().collect();
        
        if let Some(start_id) = from {
            let start_pos = result.iter().position(|u| u.user_id == start_id);
            if let Some(pos) = start_pos {
                result = result.into_iter().skip(pos + 1).collect();
            }
        }
        
        result.into_iter().take(limit).collect()
    }

    pub async fn create_or_update_user(&self, user: AdminUserInfo) {
        self.users.write().await.insert(user.user_id.clone(), user);
    }

    pub async fn deactivate_user(&self, user_id: &str) -> Result<(), AdminApiError> {
        let mut users = self.users.write().await;
        
        if let Some(user) = users.get_mut(user_id) {
            user.deactivated = true;
            info!(user_id = %user_id, "User deactivated via Admin API");
            return Ok(());
        }
        
        Err(AdminApiError::UserNotFound)
    }

    pub async fn suspend_user(&self, user_id: &str, suspended: bool) -> Result<(), AdminApiError> {
        let mut users = self.users.write().await;
        
        if let Some(user) = users.get_mut(user_id) {
            user.suspended = suspended;
            info!(user_id = %user_id, suspended = suspended, "User suspension status updated");
            return Ok(());
        }
        
        Err(AdminApiError::UserNotFound)
    }

    pub async fn set_user_admin(&self, user_id: &str, admin: bool) -> Result<(), AdminApiError> {
        let mut users = self.users.write().await;
        
        if let Some(user) = users.get_mut(user_id) {
            user.admin = admin;
            info!(user_id = %user_id, admin = admin, "User admin status updated");
            return Ok(());
        }
        
        Err(AdminApiError::UserNotFound)
    }

    pub async fn get_room(&self, room_id: &str) -> Option<AdminRoomInfo> {
        self.rooms.read().await.get(room_id).cloned()
    }

    pub async fn get_rooms(&self, limit: usize, from: Option<&str>) -> Vec<AdminRoomInfo> {
        let rooms = self.rooms.read().await;
        let mut result: Vec<_> = rooms.values().cloned().collect();
        
        if let Some(start_id) = from {
            let start_pos = result.iter().position(|r| r.room_id == start_id);
            if let Some(pos) = start_pos {
                result = result.into_iter().skip(pos + 1).collect();
            }
        }
        
        result.into_iter().take(limit).collect()
    }

    pub async fn create_or_update_room(&self, room: AdminRoomInfo) {
        self.rooms.write().await.insert(room.room_id.clone(), room);
    }

    pub async fn delete_room(&self, room_id: &str) -> Result<(), AdminApiError> {
        let mut rooms = self.rooms.write().await;
        
        if rooms.remove(room_id).is_some() {
            info!(room_id = %room_id, "Room deleted via Admin API");
            return Ok(());
        }
        
        Err(AdminApiError::RoomNotFound)
    }

    pub async fn transfer_room(&self, request: RoomTransferRequest) -> Result<RoomTransferResult, AdminApiError> {
        let mut rooms = self.rooms.write().await;
        
        if let Some(room) = rooms.get_mut(&request.room_id) {
            let old_owner = room.creator.clone();
            room.creator = request.new_owner.clone();
            
            info!(
                room_id = %request.room_id,
                old_owner = %old_owner,
                new_owner = %request.new_owner,
                "Room transferred via Admin API"
            );
            
            return Ok(RoomTransferResult {
                room_id: request.room_id,
                old_owner,
                new_owner: request.new_owner,
                transferred_at: Utc::now().timestamp_millis(),
                success: true,
                message: None,
            });
        }
        
        Err(AdminApiError::RoomNotFound)
    }

    pub async fn assign_role(&self, user_id: String, role: UserRole, assigned_by: String) -> UserRoleAssignment {
        let assignment = UserRoleAssignment::new(user_id.clone(), role, assigned_by);
        
        self.roles.write().await.insert(user_id, assignment.clone());
        
        debug!(user_id = %assignment.user_id, role = ?assignment.role, "User role assigned");
        
        assignment
    }

    pub async fn get_user_role(&self, user_id: &str) -> Option<UserRoleAssignment> {
        self.roles.read().await.get(user_id).cloned()
    }

    pub async fn remove_role(&self, user_id: &str) -> bool {
        self.roles.write().await.remove(user_id).is_some()
    }

    pub async fn start_batch_operation(&self, operation_id: String, total_items: u64) -> BatchOperationResult {
        let result = BatchOperationResult::new(operation_id.clone(), total_items);
        
        self.batch_operations.write().await.insert(operation_id, result.clone());
        
        result
    }

    pub async fn get_batch_operation(&self, operation_id: &str) -> Option<BatchOperationResult> {
        self.batch_operations.read().await.get(operation_id).cloned()
    }

    pub async fn update_batch_operation<F>(&self, operation_id: &str, f: F) -> Result<(), AdminApiError>
    where
        F: FnOnce(&mut BatchOperationResult),
    {
        let mut operations = self.batch_operations.write().await;
        
        if let Some(operation) = operations.get_mut(operation_id) {
            f(operation);
            return Ok(());
        }
        
        Err(AdminApiError::OperationNotFound)
    }

    pub async fn cancel_batch_operation(&self, operation_id: &str) -> Result<(), AdminApiError> {
        let mut operations = self.batch_operations.write().await;
        
        if let Some(operation) = operations.get_mut(operation_id) {
            operation.status = BatchOperationStatus::Cancelled;
            operation.completed_at = Some(Utc::now().timestamp_millis());
            return Ok(());
        }
        
        Err(AdminApiError::OperationNotFound)
    }

    pub async fn get_server_info(&self) -> AdminServerInfo {
        self.server_info.read().await.clone()
    }

    pub async fn update_server_info<F>(&self, f: F)
    where
        F: FnOnce(&mut AdminServerInfo),
    {
        let mut info = self.server_info.write().await;
        f(&mut info);
    }

    pub async fn get_statistics(&self) -> AdminStatistics {
        let users = self.users.read().await;
        let rooms = self.rooms.read().await;
        
        AdminStatistics {
            total_users: users.len() as u64,
            active_users: users.values().filter(|u| !u.deactivated && !u.suspended).count() as u64,
            admin_users: users.values().filter(|u| u.admin).count() as u64,
            guest_users: users.values().filter(|u| u.guest).count() as u64,
            total_rooms: rooms.len() as u64,
            public_rooms: rooms.values().filter(|r| r.public).count() as u64,
            encrypted_rooms: rooms.values().filter(|r| r.encryption.is_some()).count() as u64,
        }
    }
}

/// Admin statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminStatistics {
    pub total_users: u64,
    pub active_users: u64,
    pub admin_users: u64,
    pub guest_users: u64,
    pub total_rooms: u64,
    pub public_rooms: u64,
    pub encrypted_rooms: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum AdminApiError {
    #[error("User not found")]
    UserNotFound,
    #[error("Room not found")]
    RoomNotFound,
    #[error("Operation not found")]
    OperationNotFound,
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Batch operation failed")]
    BatchOperationFailed,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_user(user_id: &str) -> AdminUserInfo {
        AdminUserInfo {
            user_id: user_id.to_string(),
            displayname: Some("Test User".to_string()),
            avatar_url: None,
            creation_ts: Utc::now().timestamp_millis(),
            last_seen_ts: None,
            last_seen_ip: None,
            deactivated: false,
            suspended: false,
            erased: false,
            admin: false,
            guest: false,
            user_type: None,
            consent_version: None,
            consent_server_notice_sent: None,
            appservice_id: None,
        }
    }

    fn create_test_room(room_id: &str) -> AdminRoomInfo {
        AdminRoomInfo {
            room_id: room_id.to_string(),
            name: Some("Test Room".to_string()),
            topic: None,
            avatar_url: None,
            canonical_alias: None,
            join_rules: "invite".to_string(),
            room_version: "6".to_string(),
            creator: "@admin:example.com".to_string(),
            creation_ts: Utc::now().timestamp_millis(),
            joined_members: 1,
            invited_members: 0,
            banned_members: 0,
            state_events: 10,
            public: false,
            federatable: true,
            encryption: None,
        }
    }

    #[tokio::test]
    async fn test_create_user() {
        let service = AdminApiV2Service::new("example.com".to_string());
        
        let user = create_test_user("@alice:example.com");
        service.create_or_update_user(user.clone()).await;
        
        let retrieved = service.get_user("@alice:example.com").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().user_id, "@alice:example.com");
    }

    #[tokio::test]
    async fn test_deactivate_user() {
        let service = AdminApiV2Service::new("example.com".to_string());
        
        let user = create_test_user("@alice:example.com");
        service.create_or_update_user(user).await;
        
        service.deactivate_user("@alice:example.com").await.unwrap();
        
        let retrieved = service.get_user("@alice:example.com").await.unwrap();
        assert!(retrieved.deactivated);
    }

    #[tokio::test]
    async fn test_set_user_admin() {
        let service = AdminApiV2Service::new("example.com".to_string());
        
        let user = create_test_user("@alice:example.com");
        service.create_or_update_user(user).await;
        
        service.set_user_admin("@alice:example.com", true).await.unwrap();
        
        let retrieved = service.get_user("@alice:example.com").await.unwrap();
        assert!(retrieved.admin);
    }

    #[tokio::test]
    async fn test_create_room() {
        let service = AdminApiV2Service::new("example.com".to_string());
        
        let room = create_test_room("!room1:example.com");
        service.create_or_update_room(room.clone()).await;
        
        let retrieved = service.get_room("!room1:example.com").await;
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_delete_room() {
        let service = AdminApiV2Service::new("example.com".to_string());
        
        let room = create_test_room("!room1:example.com");
        service.create_or_update_room(room).await;
        
        service.delete_room("!room1:example.com").await.unwrap();
        
        let retrieved = service.get_room("!room1:example.com").await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_transfer_room() {
        let service = AdminApiV2Service::new("example.com".to_string());
        
        let room = create_test_room("!room1:example.com");
        service.create_or_update_room(room).await;
        
        let request = RoomTransferRequest {
            room_id: "!room1:example.com".to_string(),
            new_owner: "@bob:example.com".to_string(),
            force: false,
            notify_users: true,
        };
        
        let result = service.transfer_room(request).await.unwrap();
        assert!(result.success);
        assert_eq!(result.new_owner, "@bob:example.com");
    }

    #[tokio::test]
    async fn test_assign_role() {
        let service = AdminApiV2Service::new("example.com".to_string());
        
        let assignment = service.assign_role(
            "@alice:example.com".to_string(),
            UserRole::Admin,
            "@admin:example.com".to_string(),
        ).await;
        
        assert_eq!(assignment.role, UserRole::Admin);
        
        let retrieved = service.get_user_role("@alice:example.com").await.unwrap();
        assert_eq!(retrieved.role, UserRole::Admin);
    }

    #[tokio::test]
    async fn test_batch_operation() {
        let service = AdminApiV2Service::new("example.com".to_string());
        
        let result = service.start_batch_operation("op1".to_string(), 10).await;
        assert_eq!(result.total_items, 10);
        assert_eq!(result.status, BatchOperationStatus::Pending);
        
        service.update_batch_operation("op1", |op| {
            op.increment_success();
        }).await.unwrap();
        
        let updated = service.get_batch_operation("op1").await.unwrap();
        assert_eq!(updated.successful_items, 1);
        assert_eq!(updated.processed_items, 1);
    }

    #[tokio::test]
    async fn test_get_statistics() {
        let service = AdminApiV2Service::new("example.com".to_string());
        
        service.create_or_update_user(create_test_user("@alice:example.com")).await;
        service.create_or_update_room(create_test_room("!room1:example.com")).await;
        
        let stats = service.get_statistics().await;
        assert_eq!(stats.total_users, 1);
        assert_eq!(stats.total_rooms, 1);
    }

    #[tokio::test]
    async fn test_get_server_info() {
        let service = AdminApiV2Service::new("example.com".to_string());
        
        let info = service.get_server_info().await;
        assert_eq!(info.server_name, "example.com");
    }
}
