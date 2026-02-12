use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserExternalService {
    pub id: i32,
    pub user_id: String,
    pub service_type: String,
    pub endpoint: String,
    pub api_key_encrypted: Option<String>,
    pub config: serde_json::Value,
    pub status: String,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateExternalServiceParams {
    pub user_id: String,
    pub service_type: String,
    pub endpoint: String,
    pub api_key_encrypted: Option<String>,
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateExternalServiceParams {
    pub endpoint: String,
    pub api_key_encrypted: Option<String>,
    pub config: Option<serde_json::Value>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalServiceConfig {
    pub endpoint: String,
    pub has_api_key: bool,
    pub config: serde_json::Value,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalServiceCredentials {
    pub endpoint: String,
    pub api_key: String,
    pub config: serde_json::Value,
}

#[derive(Clone)]
pub struct ExternalServiceStorage {
    pub pool: Arc<Pool<Postgres>>,
}

impl ExternalServiceStorage {
    pub fn new(pool: &Arc<Pool<Postgres>>) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn get_service_config(
        &self,
        user_id: &str,
        service_type: &str,
    ) -> Result<Option<ExternalServiceConfig>, sqlx::Error> {
        let result = sqlx::query_as::<_, UserExternalService>(
            r#"SELECT * FROM user_external_services 
               WHERE user_id = $1 AND service_type = $2"#,
        )
        .bind(user_id)
        .bind(service_type)
        .fetch_optional(&*self.pool)
        .await?;

        Ok(result.map(|s| ExternalServiceConfig {
            endpoint: s.endpoint,
            has_api_key: s.api_key_encrypted.is_some(),
            config: s.config,
            status: s.status,
        }))
    }

    pub async fn get_service_credentials(
        &self,
        user_id: &str,
        service_type: &str,
    ) -> Result<Option<ExternalServiceCredentials>, sqlx::Error> {
        let result = sqlx::query_as::<_, UserExternalService>(
            r#"SELECT * FROM user_external_services 
               WHERE user_id = $1 AND service_type = $2 AND status = 'active'"#,
        )
        .bind(user_id)
        .bind(service_type)
        .fetch_optional(&*self.pool)
        .await?;

        Ok(result.and_then(|s| {
            s.api_key_encrypted.map(|key| ExternalServiceCredentials {
                endpoint: s.endpoint,
                api_key: key,
                config: s.config,
            })
        }))
    }

    pub async fn upsert_service(
        &self,
        params: CreateExternalServiceParams,
    ) -> Result<UserExternalService, sqlx::Error> {
        let config = params.config.unwrap_or(serde_json::json!({}));
        
        sqlx::query_as::<_, UserExternalService>(
            r#"INSERT INTO user_external_services 
               (user_id, service_type, endpoint, api_key_encrypted, config, status)
               VALUES ($1, $2, $3, $4, $5, 'active')
               ON CONFLICT (user_id, service_type) 
               DO UPDATE SET 
                 endpoint = $3, 
                 api_key_encrypted = $4, 
                 config = $5, 
                 status = 'active',
                 updated_at = NOW()
               RETURNING *"#,
        )
        .bind(&params.user_id)
        .bind(&params.service_type)
        .bind(&params.endpoint)
        .bind(&params.api_key_encrypted)
        .bind(&config)
        .fetch_one(&*self.pool)
        .await
    }

    pub async fn update_service(
        &self,
        user_id: &str,
        service_type: &str,
        params: UpdateExternalServiceParams,
    ) -> Result<Option<UserExternalService>, sqlx::Error> {
        let status = params.status.unwrap_or_else(|| "active".to_string());
        let config = params.config.unwrap_or(serde_json::json!({}));
        
        sqlx::query_as::<_, UserExternalService>(
            r#"UPDATE user_external_services 
               SET endpoint = $1, 
                   api_key_encrypted = $2, 
                   config = $3, 
                   status = $4,
                   updated_at = NOW()
               WHERE user_id = $5 AND service_type = $6
               RETURNING *"#,
        )
        .bind(&params.endpoint)
        .bind(&params.api_key_encrypted)
        .bind(&config)
        .bind(&status)
        .bind(user_id)
        .bind(service_type)
        .fetch_optional(&*self.pool)
        .await
    }

    pub async fn update_last_used(
        &self,
        user_id: &str,
        service_type: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE user_external_services 
               SET last_used_at = NOW() 
               WHERE user_id = $1 AND service_type = $2"#,
        )
        .bind(user_id)
        .bind(service_type)
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_service(
        &self,
        user_id: &str,
        service_type: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM user_external_services WHERE user_id = $1 AND service_type = $2",
        )
        .bind(user_id)
        .bind(service_type)
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn list_user_services(
        &self,
        user_id: &str,
    ) -> Result<Vec<UserExternalService>, sqlx::Error> {
        sqlx::query_as::<_, UserExternalService>(
            "SELECT * FROM user_external_services WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&*self.pool)
        .await
    }

    pub async fn set_service_status(
        &self,
        user_id: &str,
        service_type: &str,
        status: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"UPDATE user_external_services 
               SET status = $1, updated_at = NOW() 
               WHERE user_id = $2 AND service_type = $3"#,
        )
        .bind(status)
        .bind(user_id)
        .bind(service_type)
        .execute(&*self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
