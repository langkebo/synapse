#![cfg(test)]

use sqlx::{Pool, Postgres};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::runtime::Runtime;

use synapse_rust::auth::AuthService;
use synapse_rust::cache::{CacheConfig, CacheManager};
use synapse_rust::common::config::SecurityConfig;
use synapse_rust::common::metrics::MetricsCollector;
use synapse_rust::common::ApiError;

static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

fn unique_id() -> u64 {
    TEST_COUNTER.fetch_add(1, Ordering::SeqCst)
}

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
                "Skipping auth service tests because test database is unavailable: {}",
                error
            );
            return None;
        }
    };

    Some(pool)
}

#[test]
fn test_auth_service_register_invalid_username() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let pool = match setup_test_database().await {
            Some(pool) => Arc::new(pool),
            None => return,
        };
        let security = SecurityConfig {
            secret: "test_secret".to_string(),
            expiry_time: 3600,
            refresh_token_expiry: 604800,
            argon2_m_cost: 2048,
            argon2_t_cost: 1,
            argon2_p_cost: 1,
        };
        let cache = Arc::new(CacheManager::new(CacheConfig::default()));
        let metrics = Arc::new(MetricsCollector::new());
        let auth = AuthService::new(&pool, cache, metrics, &security, "localhost");

        let id = unique_id();
        let invalid_username = format!("user@{}", id);
        let result = auth.register(&invalid_username, "password", false, None).await;
        assert!(matches!(result, Err(ApiError::BadRequest(_))));

        let result = auth.register("", "password", false, None).await;
        assert!(matches!(result, Err(ApiError::BadRequest(_))));
    });
}

#[test]
fn test_auth_service_login_invalid_credentials() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let pool = match setup_test_database().await {
            Some(pool) => Arc::new(pool),
            None => return,
        };
        let security = SecurityConfig {
            secret: "test_secret".to_string(),
            expiry_time: 3600,
            refresh_token_expiry: 604800,
            argon2_m_cost: 2048,
            argon2_t_cost: 1,
            argon2_p_cost: 1,
        };
        let cache = Arc::new(CacheManager::new(CacheConfig::default()));
        let metrics = Arc::new(MetricsCollector::new());
        let auth = AuthService::new(&pool, cache, metrics, &security, "localhost");

        let id = unique_id();
        let nonexistent = format!("non_existent_{}", id);
        let result = auth.login(&nonexistent, "password", None, None).await;
        assert!(result.is_err());
    });
}
