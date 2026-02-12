#![cfg(test)]

use serde_json::json;
    use sqlx::{Pool, Postgres};
    use std::sync::Arc;
    use tokio::runtime::Runtime;

    use synapse_rust::cache::{CacheConfig, CacheManager};
    use synapse_rust::common::validation::Validator;
    use synapse_rust::services::room_service::{CreateRoomConfig, RoomService};
    use synapse_rust::services::sync_service::SyncService;
    use synapse_rust::services::PresenceStorage;
    use synapse_rust::storage::device::DeviceStorage;
    use synapse_rust::storage::event::EventStorage;
    use synapse_rust::storage::membership::RoomMemberStorage;
    use synapse_rust::storage::room::RoomStorage;
    use synapse_rust::storage::user::UserStorage;

    async fn setup_test_database() -> Option<Pool<Postgres>> {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .unwrap_or_else(|_| {
                "postgresql://synapse:secret@localhost:5432/synapse_test".to_string()
            });

        let pool = match sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .connect(&database_url)
            .await
        {
            Ok(pool) => pool,
            Err(error) => {
                eprintln!(
                    "Skipping sync service tests because test database is unavailable: {}",
                    error
                );
                return None;
            }
        };

        Some(pool)
    }

    async fn create_test_user(pool: &Pool<Postgres>, user_id: &str, username: &str) {
        sqlx::query(
            r#"
            INSERT INTO users (user_id, username, creation_ts)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(user_id)
        .bind(username)
        .bind(chrono::Utc::now().timestamp())
        .execute(pool)
        .await
        .expect("Failed to create test user");
    }

    #[test]
    fn test_sync_success() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let pool = match setup_test_database().await {
                Some(pool) => Arc::new(pool),
                None => return,
            };
            create_test_user(&pool, "@alice:localhost", "alice").await;

            let cache = Arc::new(CacheManager::new(CacheConfig::default()));
            let presence_storage = PresenceStorage::new(pool.clone(), cache.clone());
            let member_storage = RoomMemberStorage::new(&pool, "localhost");
            let event_storage = EventStorage::new(&pool);
            let room_storage = RoomStorage::new(&pool);
            let user_storage = UserStorage::new(&pool, cache.clone());

            let room_service = RoomService::new(
                room_storage.clone(),
                member_storage.clone(),
                event_storage.clone(),
                user_storage.clone(),
                Arc::new(Validator::default()),
                "localhost".to_string(),
                None,
            );

            let sync_service = SyncService::new(
                presence_storage,
                member_storage,
                event_storage,
                room_storage,
                DeviceStorage::new(&pool),
            );

            // Create a room and send a message
            let config = CreateRoomConfig {
                name: Some("Test Room".to_string()),
                ..Default::default()
            };
            let room_val = room_service
                .create_room("@alice:localhost", config)
                .await
                .unwrap();
            let room_id = room_val["room_id"].as_str().unwrap();

            let content = json!({"body": "Hello"});
            room_service
                .send_message(room_id, "@alice:localhost", "m.text", &content)
                .await
                .unwrap();

            let result = sync_service
                .sync("@alice:localhost", 0, false, "online", None)
                .await;
            assert!(result.is_ok());
            let val = result.unwrap();
            assert!(val["rooms"]["join"].is_object());

            assert!(val["rooms"]["join"].as_object().unwrap().contains_key(room_id));
            let room_data = &val["rooms"]["join"][room_id];
            assert_eq!(room_data["timeline"]["events"].as_array().unwrap().len(), 1);
        });
    }
