use moka::sync::Cache;
use crate::auth::Claims;
use crate::common::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;

pub struct CacheConfig {
    pub max_capacity: u64,
    pub time_to_live: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: 10000,
            time_to_live: 3600,
        }
    }
}

pub struct LocalCache {
    cache: Cache<String, Claims>,
}

impl LocalCache {
    pub fn new(config: &CacheConfig) -> Self {
        let cache = Cache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(std::time::Duration::from_secs(config.time_to_live))
            .build();
        Self { cache }
    }

    pub fn get(&self, token: &str) -> Option<Claims> {
        self.cache.get(token)
    }

    pub fn set(&self, token: &str, claims: Claims) {
        self.cache.insert(token.to_string(), claims);
    }

    pub fn remove(&self, token: &str) {
        self.cache.remove(token);
    }
}

pub struct RedisCache {
    client: Arc<Mutex<redis::Client>>,
}

impl RedisCache {
    pub async fn new(conn_str: &str) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(conn_str)?;
        Ok(Self {
            client: Arc::new(Mutex::new(client)),
        })
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        let client = self.client.lock().await;
        if let Ok(mut conn) = client.get_async_connection().await {
            if let Ok(val) = redis::cmd("GET").arg(key).query_async(&mut conn).await {
                return Some(val);
            }
        }
        None
    }

    pub async fn set(&self, key: &str, value: &str, ttl: u64) {
        let client = self.client.lock().await;
        if let Ok(mut conn) = client.get_async_connection().await {
            if ttl > 0 {
                redis::cmd("SETEX")
                    .arg(key)
                    .arg(ttl as i64)
                    .arg(value)
                    .query_async(&mut conn)
                    .await
                    .ok();
            } else {
                redis::cmd("SET")
                    .arg(key)
                    .arg(value)
                    .query_async(&mut conn)
                    .await
                    .ok();
            }
        }
    }

    pub async fn delete(&self, key: &str) {
        let client = self.client.lock().await;
        if let Ok(mut conn) = client.get_async_connection().await {
            redis::cmd("DEL").arg(key).query_async(&mut conn).await.ok();
        }
    }
}

pub struct CacheManager {
    local: LocalCache,
    redis: Option<Arc<RedisCache>>,
    use_redis: bool,
}

impl CacheManager {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            local: LocalCache::new(&config),
            redis: None,
            use_redis: false,
        }
    }

    pub async fn with_redis(conn_str: &str, config: CacheConfig) -> Result<Self, redis::RedisError> {
        match RedisCache::new(conn_str).await {
            Ok(redis_cache) => Ok(Self {
                local: LocalCache::new(&config),
                redis: Some(Arc::new(redis_cache)),
                use_redis: true,
            }),
            Err(e) => {
                tracing::warn!("Failed to connect to Redis: {}, using local cache only", e);
                Ok(Self {
                    local: LocalCache::new(&config),
                    redis: None,
                    use_redis: false,
                })
            }
        }
    }

    pub async fn get_token(&self, token: &str) -> Option<Claims> {
        if self.use_redis {
            if let Some(redis) = &self.redis {
                if let Some(val) = redis.get(token).await {
                    if let Ok(claims) = serde_json::from_str(&val) {
                        return Some(claims);
                    }
                }
            }
        }
        self.local.get(token)
    }

    pub async fn set_token(&self, token: &str, claims: &Claims, ttl: u64) {
        self.local.set(token, claims.clone());
        if self.use_redis {
            if let Some(redis) = &self.redis {
                if let Ok(val) = serde_json::to_string(claims) {
                    redis.set(token, &val, ttl).await;
                }
            }
        }
    }

    pub async fn delete_token(&self, token: &str) {
        self.local.remove(token);
        if let Some(redis) = &self.redis {
            redis.delete(token).await;
        }
    }
}
