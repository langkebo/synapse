use sqlx::{Pool, Postgres};
use crate::common::*;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Device {
    pub device_id: String,
    pub user_id: String,
    pub display_name: Option<String>,
    pub last_seen_ts: i64,
    pub last_seen_ip: Option<String>,
    pub created_ts: i64,
    pub ignored_user_list: Option<String>,
    pub appservice_id: Option<String>,
    pub first_seen_ts: i64,
}

pub struct DeviceStorage<'a> {
    pool: &'a Pool<Postgres>,
}

impl<'a> DeviceStorage<'a> {
    pub fn new(pool: &'a Pool<Postgres>) -> Self {
        Self { pool }
    }

    pub async fn create_device(
        &self,
        device_id: &str,
        user_id: &str,
        display_name: Option<&str>,
    ) -> Result<Device, sqlx::Error> {
        let now = chrono::Utc::now().timestamp_millis();
        sqlx::query_as!(
            Device,
            r#"
            INSERT INTO devices (device_id, user_id, display_name, first_seen_ts, last_seen_ts, created_ts)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
            device_id,
            user_id,
            display_name,
            now,
            now,
            now
        ).fetch_one(self.pool).await
    }

    pub async fn get_device(&self, device_id: &str) -> Result<Option<Device>, sqlx::Error> {
        sqlx::query_as!(
            Device,
            r#"
            SELECT * FROM devices WHERE device_id = $1
            "#,
            device_id
        ).fetch_optional(self.pool).await
    }

    pub async fn get_user_devices(&self, user_id: &str) -> Result<Vec<Device>, sqlx::Error> {
        sqlx::query_as!(
            Device,
            r#"
            SELECT * FROM devices WHERE user_id = $1 ORDER BY last_seen_ts DESC
            "#,
            user_id
        ).fetch_all(self.pool).await
    }

    pub async fn update_device_display_name(&self, device_id: &str, display_name: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            UPDATE devices SET display_name = $1 WHERE device_id = $2
            "#,
            display_name,
            device_id
        ).execute(self.pool).await?;
        Ok(())
    }

    pub async fn update_device_last_seen(&self, device_id: &str) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().timestamp_millis();
        sqlx::query!(
            r#"
            UPDATE devices SET last_seen_ts = $1 WHERE device_id = $2
            "#,
            now,
            device_id
        ).execute(self.pool).await?;
        Ok(())
    }

    pub async fn delete_device(&self, device_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            DELETE FROM devices WHERE device_id = $1
            "#,
            device_id
        ).execute(self.pool).await?;
        Ok(())
    }

    pub async fn delete_user_devices(&self, user_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            DELETE FROM devices WHERE user_id = $1
            "#,
            user_id
        ).execute(self.pool).await?;
        Ok(())
    }

    pub async fn device_exists(&self, device_id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            SELECT 1 FROM devices WHERE device_id = $1 LIMIT 1
            "#,
            device_id
        ).fetch_optional(self.pool).await?;
        Ok(result.is_some())
    }
}
