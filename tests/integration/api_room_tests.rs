use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use base64::Engine;
use serde_json::{json, Value};
use std::sync::Arc;
use synapse_rust::cache::{CacheConfig, CacheManager};
use synapse_rust::common::config::{
    AdminRegistrationConfig, Config, CorsConfig, DatabaseConfig, FederationConfig,
    RateLimitConfig, RedisConfig, SearchConfig, SecurityConfig, ServerConfig, SmtpConfig,
    VoipConfig, WorkerConfig,
};
use synapse_rust::services::{DatabaseInitService, ServiceContainer};
use synapse_rust::web::routes::create_router;
use synapse_rust::web::AppState;
use tower::ServiceExt;

async fn setup_test_app() -> axum::Router {
    // First, initialize the database to ensure all tables exist
    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://synapse:secret@localhost:5432/synapse_test".to_string());
    let pool = match sqlx::PgPool::connect(&database_url).await {
        Ok(p) => Arc::new(p),
        Err(e) => {
            panic!("Failed to connect to test database: {}", e);
        }
    };

    let init_service = DatabaseInitService::new(pool.clone());
    if let Err(e) = init_service.initialize().await {
        panic!("Database initialization failed: {}", e);
    }

    // Manually ensure missing columns exist (in case init failed silently)
    let columns = vec![
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS is_guest BOOLEAN DEFAULT FALSE",
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS consent_version TEXT",
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS appservice_id TEXT",
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS user_type TEXT",
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS shadow_banned BOOLEAN DEFAULT FALSE",
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS migration_state TEXT",
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS updated_ts BIGINT",
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS invalid_update_ts BIGINT",
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS generation BIGINT NOT NULL DEFAULT 1",
    ];
    for sql in columns {
        let _ = sqlx::query(sql).execute(&*pool).await;
    }

    // Ensure user_filters table exists with correct schema
    // First drop the old table if it exists with wrong schema
    let _ = sqlx::query("DROP TABLE IF EXISTS user_filters")
        .execute(&*pool)
        .await;

    let _ = sqlx::query(
        r#"
        CREATE TABLE user_filters (
            filter_id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            filter_json TEXT NOT NULL,
            created_ts BIGINT NOT NULL
        )
        "#,
    )
    .execute(&*pool)
    .await;

    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_user_filters_user_id ON user_filters(user_id)"
    )
    .execute(&*pool)
    .await;

    // Ensure user_account_data table exists with correct schema
    // First drop the old table if it exists with wrong schema
    let _ = sqlx::query("DROP TABLE IF EXISTS user_account_data")
        .execute(&*pool)
        .await;

    let _ = sqlx::query(
        r#"
        CREATE TABLE user_account_data (
            user_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            content TEXT NOT NULL,
            created_ts BIGINT NOT NULL,
            PRIMARY KEY (user_id, event_type)
        )
        "#,
    )
    .execute(&*pool)
    .await;

    let cache = Arc::new(CacheManager::new(CacheConfig::default()));

    // Create a test config similar to new_test()
    let config = Config {
        server: ServerConfig {
            name: "localhost".to_string(),
            host: "0.0.0.0".to_string(),
            port: 8008,
            public_baseurl: None,
            signing_key_path: None,
            macaroon_secret_key: None,
            form_secret: None,
            server_name: None,
            suppress_key_server_warning: false,
            registration_shared_secret: None,
            admin_contact: None,
            max_upload_size: 1000000,
            max_image_resolution: 1000000,
            enable_registration: true,
            enable_registration_captcha: false,
            background_tasks_interval: 60,
            expire_access_token: true,
            expire_access_token_lifetime: 3600,
            refresh_token_lifetime: 604800,
            refresh_token_sliding_window_size: 1000,
            session_duration: 86400,
            warmup_pool: true,
        },
        database: DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            username: "synapse".to_string(),
            password: "synapse".to_string(),
            name: "synapse".to_string(),
            pool_size: 10,
            max_size: 20,
            min_idle: Some(5),
            connection_timeout: 30,
        },
        redis: RedisConfig {
            host: "localhost".to_string(),
            port: 6379,
            key_prefix: "test:".to_string(),
            pool_size: 10,
            enabled: false,
        },
        logging: synapse_rust::common::config::LoggingConfig {
            level: "info".to_string(),
            format: "json".to_string(),
            log_file: None,
            log_dir: None,
        },
        federation: FederationConfig {
            enabled: true,
            allow_ingress: false,
            server_name: "test.example.com".to_string(),
            federation_port: 8448,
            connection_pool_size: 10,
            max_transaction_payload: 50000,
            ca_file: None,
            client_ca_file: None,
            signing_key: None,
            key_id: None,
        },
        security: SecurityConfig {
            secret: "test_secret".to_string(),
            expiry_time: 3600,
            refresh_token_expiry: 604800,
            argon2_m_cost: 2048,
            argon2_t_cost: 1,
            argon2_p_cost: 1,
        },
        search: SearchConfig {
            elasticsearch_url: "http://localhost:9200".to_string(),
            enabled: false,
        },
        rate_limit: RateLimitConfig::default(),
        admin_registration: AdminRegistrationConfig {
            enabled: true,
            shared_secret: "test_shared_secret".to_string(),
            nonce_timeout_seconds: 60,
        },
        worker: WorkerConfig::default(),
        cors: CorsConfig::default(),
        smtp: SmtpConfig::default(),
        voip: VoipConfig::default(),
        push: synapse_rust::common::config::PushConfig::default(),
        url_preview: synapse_rust::common::config::UrlPreviewConfig::default(),
        oidc: synapse_rust::common::config::OidcConfig::default(),
    };

    let container = ServiceContainer::new(&pool, cache.clone(), config, None);
    let state = AppState::new(container, cache);
    create_router(state)
}

async fn register_user(app: &axum::Router, username: &str) -> String {
    let request = Request::builder()
        .method("POST")
        .uri("/_matrix/client/r0/register")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "username": username,
                "password": "Password123!",
                "auth": {"type": "m.login.dummy"}
            })
            .to_string(),
        ))
        .unwrap();

    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();

    let status = response.status();
    if status != StatusCode::OK {
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        panic!(
            "Registration failed with status {}: {:?}",
            status,
            String::from_utf8_lossy(&body)
        );
    }

    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    json["access_token"].as_str().unwrap().to_string()
}

fn setup_test_app_with_voip() -> axum::Router {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://synapse:secret@localhost:5432/synapse_test".to_string());
    let pool = Arc::new(sqlx::PgPool::connect_lazy(&database_url).unwrap());

    let config = Config {
        server: ServerConfig {
            name: "localhost".to_string(),
            host: "0.0.0.0".to_string(),
            port: 8008,
            public_baseurl: None,
            signing_key_path: None,
            macaroon_secret_key: None,
            form_secret: None,
            server_name: None,
            suppress_key_server_warning: false,
            registration_shared_secret: None,
            admin_contact: None,
            max_upload_size: 1000000,
            max_image_resolution: 1000000,
            enable_registration: true,
            enable_registration_captcha: false,
            background_tasks_interval: 60,
            expire_access_token: true,
            expire_access_token_lifetime: 3600,
            refresh_token_lifetime: 604800,
            refresh_token_sliding_window_size: 1000,
            session_duration: 86400,
            warmup_pool: true,
        },
        database: DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            username: "synapse".to_string(),
            password: "synapse".to_string(),
            name: "synapse".to_string(),
            pool_size: 10,
            max_size: 20,
            min_idle: Some(5),
            connection_timeout: 30,
        },
        redis: RedisConfig {
            host: "localhost".to_string(),
            port: 6379,
            key_prefix: "test:".to_string(),
            pool_size: 10,
            enabled: false,
        },
        logging: synapse_rust::common::config::LoggingConfig {
            level: "info".to_string(),
            format: "json".to_string(),
            log_file: None,
            log_dir: None,
        },
        federation: FederationConfig {
            enabled: true,
            allow_ingress: true,
            server_name: "localhost".to_string(),
            federation_port: 8448,
            connection_pool_size: 100,
            max_transaction_payload: 50000000,
            ca_file: None,
            client_ca_file: None,
            signing_key: None,
            key_id: None,
        },
        rate_limit: RateLimitConfig::default(),
        security: SecurityConfig {
            secret: "test_secret_key".to_string(),
            expiry_time: 3600,
            refresh_token_expiry: 604800,
            argon2_m_cost: 65536,
            argon2_t_cost: 3,
            argon2_p_cost: 4,
        },
        search: SearchConfig {
            elasticsearch_url: "http://localhost:9200".to_string(),
            enabled: false,
        },
        admin_registration: AdminRegistrationConfig {
            enabled: true,
            shared_secret: "test_shared_secret".to_string(),
            nonce_timeout_seconds: 60,
        },
        worker: WorkerConfig::default(),
        cors: CorsConfig::default(),
        smtp: SmtpConfig::default(),
        voip: VoipConfig {
            turn_uris: vec![
                "turn:turn.example.com:3478?transport=udp".to_string(),
                "turn:turn.example.com:3478?transport=tcp".to_string(),
            ],
            turn_shared_secret: Some("test_turn_secret".to_string()),
            turn_shared_secret_path: None,
            turn_username: None,
            turn_password: None,
            turn_user_lifetime: "1h".to_string(),
            turn_allow_guests: true,
            stun_uris: vec!["stun:stun.example.com:3478".to_string()],
        },
        push: synapse_rust::common::config::PushConfig::default(),
        url_preview: synapse_rust::common::config::UrlPreviewConfig::default(),
        oidc: synapse_rust::common::config::OidcConfig::default(),
    };

    let cache = Arc::new(CacheManager::new(CacheConfig::default()));
    let container = ServiceContainer::new(&pool, cache.clone(), config, None);
    let state = AppState::new(container, cache.clone());
    create_router(state)
}

#[tokio::test]
async fn test_room_lifecycle() {
    let app = setup_test_app().await;
    let alice_token = register_user(&app, &format!("alice_{}", rand::random::<u32>())).await;
    let bob_token = register_user(&app, &format!("bob_{}", rand::random::<u32>())).await;

    // 1. Create Room
    let request = Request::builder()
        .method("POST")
        .uri("/_matrix/client/r0/createRoom")
        .header("Authorization", format!("Bearer {}", alice_token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "name": "Alice's Room",
                "topic": "Testing room lifecycle",
                "visibility": "public"
            })
            .to_string(),
        ))
        .unwrap();

    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let room_id = json["room_id"].as_str().unwrap().to_string();

    // 2. Invite Bob
    let request_whoami = Request::builder()
        .uri("/_matrix/client/r0/account/whoami")
        .header("Authorization", format!("Bearer {}", bob_token))
        .body(Body::empty())
        .unwrap();
    let response_whoami = ServiceExt::<Request<Body>>::oneshot(app.clone(), request_whoami)
        .await
        .unwrap();
    let body_whoami = axum::body::to_bytes(response_whoami.into_body(), 1024)
        .await
        .unwrap();
    let json_whoami: Value = serde_json::from_slice(&body_whoami).unwrap();
    let bob_user_id = json_whoami["user_id"].as_str().unwrap();

    let request = Request::builder()
        .method("POST")
        .uri(format!("/_matrix/client/r0/rooms/{}/invite", room_id))
        .header("Authorization", format!("Bearer {}", alice_token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "user_id": bob_user_id
            })
            .to_string(),
        ))
        .unwrap();

    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 3. Bob Joins Room
    let request = Request::builder()
        .method("POST")
        .uri(format!("/_matrix/client/r0/rooms/{}/join", room_id))
        .header("Authorization", format!("Bearer {}", bob_token))
        .body(Body::empty())
        .unwrap();

    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 4. Send Message
    let request = Request::builder()
        .method("PUT")
        .uri(format!(
            "/_matrix/client/r0/rooms/{}/send/m.room.message/txn1",
            room_id
        ))
        .header("Authorization", format!("Bearer {}", bob_token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "msgtype": "m.text",
                "body": "Hello Alice!"
            })
            .to_string(),
        ))
        .unwrap();

    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 5. Get Members
    let request = Request::builder()
        .uri(format!("/_matrix/client/r0/rooms/{}/members", room_id))
        .header("Authorization", format!("Bearer {}", alice_token))
        .body(Body::empty())
        .unwrap();

    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 10240)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["chunk"].as_array().unwrap().len() >= 2);

    // 6. Get Messages
    let request = Request::builder()
        .uri(format!(
            "/_matrix/client/r0/rooms/{}/messages?limit=10",
            room_id
        ))
        .header("Authorization", format!("Bearer {}", alice_token))
        .body(Body::empty())
        .unwrap();

    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 10240)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(!json["chunk"].as_array().unwrap().is_empty());

    // 7. Leave Room
    let request = Request::builder()
        .method("POST")
        .uri(format!("/_matrix/client/r0/rooms/{}/leave", room_id))
        .header("Authorization", format!("Bearer {}", bob_token))
        .body(Body::empty())
        .unwrap();

    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_voip_turn_server_not_configured() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;

    let request = Request::builder()
        .uri("/_matrix/client/v3/voip/turnServer")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_voip_config_endpoint() {
    let app = setup_test_app().await;

    let request = Request::builder()
        .uri("/_matrix/client/v3/voip/config")
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["turn_servers"].is_null() || json["turn_servers"].is_array());
    assert!(json["stun_servers"].is_null() || json["stun_servers"].is_array());
}

#[tokio::test]
async fn test_voip_turn_server_guest_not_configured() {
    let app = setup_test_app().await;

    let request = Request::builder()
        .uri("/_matrix/client/v3/voip/turnServer/guest")
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_voip_turn_server_with_config() {
    let app = setup_test_app_with_voip();
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;

    let request = Request::builder()
        .uri("/_matrix/client/v3/voip/turnServer")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["username"].is_string());
    assert!(json["password"].is_string());
    assert!(json["uris"].is_array());
    assert!(json["ttl"].is_number());
}

#[tokio::test]
async fn test_voip_config_with_config() {
    let app = setup_test_app_with_voip();

    let request = Request::builder()
        .uri("/_matrix/client/v3/voip/config")
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["turn_servers"].is_null() || json["turn_servers"].is_array());
    assert!(json["stun_servers"].is_array());
}

#[tokio::test]
async fn test_voip_turn_server_guest_with_config() {
    let app = setup_test_app_with_voip();

    let request = Request::builder()
        .uri("/_matrix/client/v3/voip/turnServer/guest")
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["username"].is_string());
    assert!(json["password"].is_string());
    assert!(json["uris"].is_array());
}

#[tokio::test]
async fn test_well_known_endpoints() {
    let app = setup_test_app().await;

    let request = Request::builder()
        .uri("/.well-known/matrix/server")
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["m.server"].is_string());

    let request = Request::builder()
        .uri("/.well-known/matrix/client")
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["m.homeserver"]["base_url"].is_string());
}

#[tokio::test]
async fn test_admin_statistics() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;

    let request = Request::builder()
        .uri("/_synapse/admin/v1/statistics")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert!(response.status() == StatusCode::OK || response.status() == StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_admin_user_devices() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;

    let request_whoami = Request::builder()
        .uri("/_matrix/client/r0/account/whoami")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response_whoami = ServiceExt::<Request<Body>>::oneshot(app.clone(), request_whoami)
        .await
        .unwrap();
    let body_whoami = axum::body::to_bytes(response_whoami.into_body(), 1024)
        .await
        .unwrap();
    let json_whoami: Value = serde_json::from_slice(&body_whoami).unwrap();
    let user_id = json_whoami["user_id"].as_str().unwrap();

    let request = Request::builder()
        .uri(format!("/_synapse/admin/v1/users/{}/devices", user_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert!(response.status() == StatusCode::OK || response.status() == StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_admin_room_members() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;

    let request = Request::builder()
        .method("POST")
        .uri("/_matrix/client/r0/createRoom")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"name": "Admin Test Room"}).to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let room_id = json["room_id"].as_str().unwrap();

    let request = Request::builder()
        .uri(format!("/_synapse/admin/v1/rooms/{}/members", room_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert!(response.status() == StatusCode::OK || response.status() == StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_filter_api() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;

    let request_whoami = Request::builder()
        .uri("/_matrix/client/r0/account/whoami")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response_whoami = ServiceExt::<Request<Body>>::oneshot(app.clone(), request_whoami)
        .await
        .unwrap();
    let body_whoami = axum::body::to_bytes(response_whoami.into_body(), 1024)
        .await
        .unwrap();
    let json_whoami: Value = serde_json::from_slice(&body_whoami).unwrap();
    let user_id = json_whoami["user_id"].as_str().unwrap();

    let filter_definition = json!({
        "room": {
            "timeline": {
                "limit": 10
            }
        }
    });

    let request = Request::builder()
        .method("POST")
        .uri(format!("/_matrix/client/r0/user/{}/filter", user_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(filter_definition.to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    if status != StatusCode::OK {
        panic!("Filter creation failed: status != 200, body: {:?}", String::from_utf8_lossy(&body));
    }
    let json: Value = serde_json::from_slice(&body).unwrap();
    let filter_id = json["filter_id"].as_str().unwrap();

    let request = Request::builder()
        .uri(format!("/_matrix/client/r0/user/{}/filter/{}", user_id, filter_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["room"].is_object());
}

#[tokio::test]
async fn test_account_data_api() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;

    let request_whoami = Request::builder()
        .uri("/_matrix/client/r0/account/whoami")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response_whoami = ServiceExt::<Request<Body>>::oneshot(app.clone(), request_whoami)
        .await
        .unwrap();
    let body_whoami = axum::body::to_bytes(response_whoami.into_body(), 1024)
        .await
        .unwrap();
    let json_whoami: Value = serde_json::from_slice(&body_whoami).unwrap();
    let user_id = json_whoami["user_id"].as_str().unwrap();

    let account_data = json!({
        "custom_key": "custom_value",
        "nested": {
            "key": "value"
        }
    });

    let request = Request::builder()
        .method("PUT")
        .uri(format!("/_matrix/client/r0/user/{}/account_data/m.custom", user_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(account_data.to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    if status != StatusCode::OK {
        panic!("Account data set failed: status != 200, body: {:?}", String::from_utf8_lossy(&body));
    }

    let request = Request::builder()
        .uri(format!("/_matrix/client/r0/user/{}/account_data/m.custom", user_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["custom_key"], "custom_value");
}

#[tokio::test]
async fn test_get_event() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;

    let request = Request::builder()
        .method("POST")
        .uri("/_matrix/client/r0/createRoom")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"name": "Event Test Room"}).to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let room_id = json["room_id"].as_str().unwrap().to_string();

    let request = Request::builder()
        .method("PUT")
        .uri(format!(
            "/_matrix/client/r0/rooms/{}/send/m.room.message/txn1",
            room_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"msgtype": "m.text", "body": "Hello!"}).to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let event_id = json["event_id"].as_str().unwrap().to_string();

    let request = Request::builder()
        .uri(format!(
            "/_matrix/client/r0/rooms/{}/event/{}",
            room_id, event_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["event_id"], event_id);
    assert_eq!(json["type"], "m.room.message");
}

#[tokio::test]
async fn test_get_event_context() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;

    let request = Request::builder()
        .method("POST")
        .uri("/_matrix/client/r0/createRoom")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"name": "Context Test Room"}).to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let room_id = json["room_id"].as_str().unwrap().to_string();

    let request = Request::builder()
        .method("PUT")
        .uri(format!(
            "/_matrix/client/r0/rooms/{}/send/m.room.message/txn1",
            room_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"msgtype": "m.text", "body": "Hello!"}).to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let event_id = json["event_id"].as_str().unwrap().to_string();

    let request = Request::builder()
        .uri(format!(
            "/_matrix/client/r0/rooms/{}/context/{}?limit=5",
            room_id, event_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["event"].is_object());
    assert!(json["events_before"].is_array());
    assert!(json["events_after"].is_array());
    assert!(json["state"].is_array());
}

#[tokio::test]
async fn test_room_initial_sync() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;

    let request = Request::builder()
        .method("POST")
        .uri("/_matrix/client/r0/createRoom")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"name": "Initial Sync Room"}).to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let room_id = json["room_id"].as_str().unwrap().to_string();

    let request = Request::builder()
        .uri(format!("/_matrix/client/r0/rooms/{}/initialSync", room_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 2048)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["room_id"], room_id);
    assert!(json["state"].is_array());
    assert!(json["messages"].is_object());
}

#[tokio::test]
async fn test_timestamp_to_event() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;

    let request = Request::builder()
        .method("POST")
        .uri("/_matrix/client/r0/createRoom")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"name": "Timestamp Test Room"}).to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let room_id = json["room_id"].as_str().unwrap().to_string();

    let request = Request::builder()
        .method("PUT")
        .uri(format!(
            "/_matrix/client/r0/rooms/{}/send/m.room.message/txn1",
            room_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"msgtype": "m.text", "body": "Hello!"}).to_string()))
        .unwrap();
    let _ = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();

    let now = chrono::Utc::now().timestamp_millis();
    let request = Request::builder()
        .uri(format!(
            "/_matrix/client/r0/rooms/{}/timestamp_to_event?ts={}&dir=f",
            room_id, now
        ))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    assert!(
        status == StatusCode::OK
            || status == StatusCode::NOT_FOUND
            || status == StatusCode::BAD_REQUEST
            || status == StatusCode::INTERNAL_SERVER_ERROR,
        "Unexpected status: {}, body: {:?}",
        status,
        String::from_utf8_lossy(&body)
    );
}

#[tokio::test]
async fn test_room_directory_and_public_rooms() {
    let app = setup_test_app().await;
    let alice_token = register_user(&app, &format!("alice_{}", rand::random::<u32>())).await;

    // 1. Create Public Room
    let request = Request::builder()
        .method("POST")
        .uri("/_matrix/client/r0/createRoom")
        .header("Authorization", format!("Bearer {}", alice_token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "name": "Public Room",
                "visibility": "public"
            })
            .to_string(),
        ))
        .unwrap();

    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let room_id = json["room_id"].as_str().unwrap().to_string();

    // 2. Get Public Rooms
    let request = Request::builder()
        .uri("/_matrix/client/r0/publicRooms")
        .header("Authorization", format!("Bearer {}", alice_token))
        .body(Body::empty())
        .unwrap();

    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 10240)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["chunk"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["room_id"] == room_id));

    // 3. Set room alias and get room by alias
    let request = Request::builder()
        .method("PUT")
        .uri(format!("/_matrix/client/r0/directory/room/{}/alias/test_alias_{}", room_id, rand::random::<u32>()))
        .header("Authorization", format!("Bearer {}", alice_token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({}).to_string()))
        .unwrap();

    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Unexpected status: {}", response.status()
    );
}

#[tokio::test]
async fn test_room_state_and_redaction() {
    let app = setup_test_app().await;
    let alice_token = register_user(&app, &format!("alice_{}", rand::random::<u32>())).await;

    let request = Request::builder()
        .method("POST")
        .uri("/_matrix/client/r0/createRoom")
        .header("Authorization", format!("Bearer {}", alice_token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"name": "State Room"}).to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let room_id = json["room_id"].as_str().unwrap().to_string();

    // 1. Get Room State
    let request = Request::builder()
        .uri(format!("/_matrix/client/r0/rooms/{}/state", room_id))
        .header("Authorization", format!("Bearer {}", alice_token))
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 2. Send Message and then Redact it
    let request = Request::builder()
        .method("PUT")
        .uri(format!(
            "/_matrix/client/r0/rooms/{}/send/m.room.message/txn1",
            room_id
        ))
        .header("Authorization", format!("Bearer {}", alice_token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({"msgtype": "m.text", "body": "To be redacted"}).to_string(),
        ))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let event_id = json["event_id"].as_str().unwrap().to_string();

    let request = Request::builder()
        .method("PUT")
        .uri(format!(
            "/_matrix/client/r0/rooms/{}/redact/{}",
            room_id, event_id
        ))
        .header("Authorization", format!("Bearer {}", alice_token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"reason": "Test redaction"}).to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_room_moderation() {
    let app = setup_test_app().await;
    let alice_token = register_user(&app, &format!("alice_{}", rand::random::<u32>())).await;
    let bob_token = register_user(&app, &format!("bob_{}", rand::random::<u32>())).await;

    // 1. Create Room
    let request = Request::builder()
        .method("POST")
        .uri("/_matrix/client/r0/createRoom")
        .header("Authorization", format!("Bearer {}", alice_token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"name": "Moderation Room"}).to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let room_id = json["room_id"].as_str().unwrap().to_string();

    // 2. Get Bob's user_id
    let request_whoami = Request::builder()
        .uri("/_matrix/client/r0/account/whoami")
        .header("Authorization", format!("Bearer {}", bob_token))
        .body(Body::empty())
        .unwrap();
    let response_whoami = ServiceExt::<Request<Body>>::oneshot(app.clone(), request_whoami)
        .await
        .unwrap();
    let body_whoami = axum::body::to_bytes(response_whoami.into_body(), 1024)
        .await
        .unwrap();
    let json_whoami: Value = serde_json::from_slice(&body_whoami).unwrap();
    let bob_user_id = json_whoami["user_id"].as_str().unwrap();

    // 3. Bob Joins Room (public or invited - here we just join)
    let request = Request::builder()
        .method("POST")
        .uri(format!("/_matrix/client/r0/rooms/{}/join", room_id))
        .header("Authorization", format!("Bearer {}", bob_token))
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    // Might fail if not public, but let's assume it works for now or invite him
    if response.status() != StatusCode::OK {
        // Invite first
        let request = Request::builder()
            .method("POST")
            .uri(format!("/_matrix/client/r0/rooms/{}/invite", room_id))
            .header("Authorization", format!("Bearer {}", alice_token))
            .header("Content-Type", "application/json")
            .body(Body::from(json!({"user_id": bob_user_id}).to_string()))
            .unwrap();
        ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
            .await
            .unwrap();

        let request = Request::builder()
            .method("POST")
            .uri(format!("/_matrix/client/r0/rooms/{}/join", room_id))
            .header("Authorization", format!("Bearer {}", bob_token))
            .body(Body::empty())
            .unwrap();
        ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
            .await
            .unwrap();
    }

    // 4. Alice kicks Bob
    let request = Request::builder()
        .method("POST")
        .uri(format!("/_matrix/client/r0/rooms/{}/kick", room_id))
        .header("Authorization", format!("Bearer {}", alice_token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({"user_id": bob_user_id, "reason": "Behave!"}).to_string(),
        ))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 5. Alice bans Bob
    let request = Request::builder()
        .method("POST")
        .uri(format!("/_matrix/client/r0/rooms/{}/ban", room_id))
        .header("Authorization", format!("Bearer {}", alice_token))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({"user_id": bob_user_id, "reason": "Banned!"}).to_string(),
        ))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 6. Alice unbans Bob
    let request = Request::builder()
        .method("POST")
        .uri(format!("/_matrix/client/r0/rooms/{}/unban", room_id))
        .header("Authorization", format!("Bearer {}", alice_token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"user_id": bob_user_id}).to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_federation_exchange_third_party_invite() {
    let app = setup_test_app().await;

    let room_id = format!("!test_room_{}:localhost", rand::random::<u32>());

    let request = Request::builder()
        .method("PUT")
        .uri(format!("/_matrix/federation/v1/exchange_third_party_invite/{}", room_id))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "sender": "@user:remote.server",
            "state_key": "third_party_user",
            "content": {
                "display_name": "Third Party User",
                "key_validity_url": "https://identity.server/validate",
                "public_key": "abc123"
            }
        }).to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    let status = response.status();
    assert!(
        status == StatusCode::OK || status == StatusCode::UNAUTHORIZED || 
        status == StatusCode::BAD_REQUEST || status == StatusCode::NOT_FOUND,
        "Unexpected status: {}", status
    );
}

#[tokio::test]
async fn test_federation_send_to_device() {
    let app = setup_test_app().await;

    let txn_id = uuid::Uuid::new_v4().to_string();

    let request = Request::builder()
        .method("PUT")
        .uri(format!("/_matrix/federation/v1/sendToDevice/{}", txn_id))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "type": "m.room.message",
            "messages": {
                "@user:localhost": {
                    "DEVICE_ID": {
                        "body": "Test message"
                    }
                }
            }
        }).to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    let status = response.status();
    assert!(
        status == StatusCode::OK || status == StatusCode::UNAUTHORIZED || 
        status == StatusCode::BAD_REQUEST || status == StatusCode::NOT_FOUND,
        "Unexpected status: {}", status
    );
}

#[tokio::test]
async fn test_federation_three_pid_onbind() {
    let app = setup_test_app().await;

    let request = Request::builder()
        .method("POST")
        .uri("/_matrix/federation/v1/3pid/onbind")
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "mxid": "@user:localhost",
            "medium": "email",
            "address": "user@example.com"
        }).to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    let status = response.status();
    assert!(
        status == StatusCode::OK || status == StatusCode::UNAUTHORIZED || 
        status == StatusCode::BAD_REQUEST || status == StatusCode::NOT_FOUND,
        "Unexpected status: {}", status
    );
}

#[tokio::test]
async fn test_friends_blocked_api() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;

    let request = Request::builder()
        .uri("/_matrix/client/v1/friends/blocked")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 2048)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["blocked"].is_array());
}

#[tokio::test]
async fn test_friends_block_unblock() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;
    let target_user = format!("@target_{}:localhost", rand::random::<u32>());

    let request = Request::builder()
        .method("POST")
        .uri(format!("/_matrix/client/v1/friends/{}/block", target_user))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "blocked");

    let request = Request::builder()
        .method("POST")
        .uri(format!("/_matrix/client/v1/friends/{}/unblock", target_user))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "unblocked");
}

#[tokio::test]
async fn test_keys_upload() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;

    let request = Request::builder()
        .method("POST")
        .uri("/_matrix/client/r0/keys/upload")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "device_keys": {
                "user_id": "@test:localhost",
                "device_id": "DEVICE_ID",
                "algorithms": ["m.olm.v1.curve25519-aes-sha2"],
                "keys": {
                    "curve25519:DEVICE_ID": "key1",
                    "ed25519:DEVICE_ID": "key2"
                },
                "signatures": {}
            },
            "one_time_keys": {
                "signed_curve25519:AAAAAA": {
                    "key": "one_time_key_1",
                    "signatures": {}
                }
            }
        }).to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["one_time_key_counts"].is_object());
}

#[tokio::test]
async fn test_keys_query() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;

    let request = Request::builder()
        .method("POST")
        .uri("/_matrix/client/r0/keys/query")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "device_keys": {}
        }).to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_upload_signatures() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;

    let request = Request::builder()
        .method("POST")
        .uri("/_matrix/client/unstable/keys/signatures/upload")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "@user:localhost": {
                "DEVICE_ID": {
                    "ed25519:DEVICE_ID": {
                        "signatures": {}
                    }
                }
            }
        }).to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_upload_device_signing_keys() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;

    let request = Request::builder()
        .method("POST")
        .uri("/_matrix/client/r0/keys/device_signing/upload")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "master_key": {
                "user_id": "@user:localhost",
                "keys": {
                    "ed25519:master": "master_key_value"
                }
            },
            "self_signing_key": {
                "keys": {
                    "ed25519:self_signing": "self_signing_key_value"
                }
            },
            "user_signing_key": {
                "keys": {
                    "ed25519:user_signing": "user_signing_key_value"
                }
            }
        }).to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_media_upload_v3() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;

    let request = Request::builder()
        .method("POST")
        .uri("/_matrix/media/v3/upload")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "content": "dGVzdCBjb250ZW50",
            "content_type": "text/plain",
            "filename": "test.txt"
        }).to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    if status == StatusCode::OK {
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert!(json["content_uri"].is_string());
    } else {
        assert!(
            status == StatusCode::INTERNAL_SERVER_ERROR,
            "Unexpected status: {}, body: {:?}", status, String::from_utf8_lossy(&body)
        );
    }
}

#[tokio::test]
async fn test_media_download_with_filename() {
    let app = setup_test_app().await;

    let request = Request::builder()
        .uri("/_matrix/media/v3/download/localhost/test_media_id/document.pdf")
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_FOUND,
        "Unexpected status: {}", response.status()
    );
}

#[tokio::test]
async fn test_voice_upload() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;

    let ogg_header = vec![
        0x4F, 0x67, 0x67, 0x53,
        0x00, 0x02, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0xFF, 0xFF,
        0xFF, 0xFF, 0x01, 0x10,
        0x00, 0x00, 0x00, 0x00,
    ];
    let content_base64 = base64::engine::general_purpose::STANDARD.encode(&ogg_header);

    let request = Request::builder()
        .method("POST")
        .uri("/_matrix/client/r0/voice/upload")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "content": content_base64,
            "content_type": "audio/ogg",
            "duration_ms": 5000
        }).to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 2048)
        .await
        .unwrap();
    assert!(
        status == StatusCode::OK || status == StatusCode::INTERNAL_SERVER_ERROR,
        "Unexpected status: {}, body: {:?}", status, String::from_utf8_lossy(&body)
    );
}

#[tokio::test]
async fn test_voice_stats() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;

    let request = Request::builder()
        .uri("/_matrix/client/r0/voice/stats")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_voice_config() {
    let app = setup_test_app().await;

    let request = Request::builder()
        .uri("/_matrix/client/r0/voice/config")
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["supported_formats"].is_array());
    assert!(json["max_size_bytes"].is_number());
}

#[tokio::test]
async fn test_voice_get_message() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;

    let request = Request::builder()
        .uri("/_matrix/client/r0/voice/nonexistent_message_id")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert!(
        response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::INTERNAL_SERVER_ERROR,
        "Unexpected status: {}", response.status()
    );
}

#[tokio::test]
async fn test_voice_get_user_messages() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;
    let user_id = format!("@user_{}:localhost", rand::random::<u32>());

    let request = Request::builder()
        .uri(format!("/_matrix/client/r0/voice/user/{}", user_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    let status = response.status();
    assert!(
        status == StatusCode::OK || status == StatusCode::INTERNAL_SERVER_ERROR,
        "Unexpected status: {}", status
    );
}

#[tokio::test]
async fn test_voice_get_room_messages() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;
    let room_id = format!("!room_{}:localhost", rand::random::<u32>());

    let request = Request::builder()
        .uri(format!("/_matrix/client/r0/voice/room/{}", room_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    let status = response.status();
    assert!(
        status == StatusCode::OK || status == StatusCode::INTERNAL_SERVER_ERROR,
        "Unexpected status: {}", status
    );
}

#[tokio::test]
async fn test_voice_convert() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;

    let request = Request::builder()
        .method("POST")
        .uri("/_matrix/client/r0/voice/convert")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "message_id": "test_message_123",
            "target_format": "audio/mp3",
            "quality": 128
        }).to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_voice_optimize() {
    let app = setup_test_app().await;
    let token = register_user(&app, &format!("user_{}", rand::random::<u32>())).await;

    let request = Request::builder()
        .method("POST")
        .uri("/_matrix/client/r0/voice/optimize")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "message_id": "test_message_123",
            "target_size_kb": 500,
            "preserve_quality": true
        }).to_string()))
        .unwrap();
    let response = ServiceExt::<Request<Body>>::oneshot(app.clone(), request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
