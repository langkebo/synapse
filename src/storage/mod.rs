use sqlx::{Pool, Postgres, Row};
use crate::common::*;
use std::sync::Arc;

pub struct Database {
    pub pool: Pool<Postgres>,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = sqlx::PgPool::connect(database_url).await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &Pool<Postgres> {
        &self.pool
    }
}

pub async fn initialize_database(pool: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS users (
            user_id TEXT NOT NULL PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT,
            is_admin BOOLEAN DEFAULT FALSE,
            is_guest BOOLEAN DEFAULT FALSE,
            consent_version TEXT,
            appservice_id TEXT,
            creation_ts BIGINT NOT NULL,
            user_type TEXT,
            deactivated BOOLEAN DEFAULT FALSE,
            shadow_banned BOOLEAN DEFAULT FALSE,
            generation BIGINT NOT NULL,
            avatar_url TEXT,
            displayname TEXT,
            invalid_update_ts BIGINT,
            migration_state TEXT
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS devices (
            device_id TEXT NOT NULL PRIMARY KEY,
            user_id TEXT NOT NULL,
            display_name TEXT,
            last_seen_ts TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            last_seen_ip TEXT,
            created_ts BIGINT NOT NULL,
            user_agent TEXT,
            keys JSONB,
            device_display_name TEXT,
            FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS access_tokens (
            id BIGSERIAL PRIMARY KEY,
            token TEXT NOT NULL UNIQUE,
            user_id TEXT NOT NULL,
            device_id TEXT,
            created_ts BIGINT NOT NULL,
            expired_ts BIGINT,
            invalidated BOOLEAN DEFAULT FALSE,
            FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE,
            FOREIGN KEY (device_id) REFERENCES devices(device_id) ON DELETE CASCADE
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS refresh_tokens (
            id BIGSERIAL PRIMARY KEY,
            token TEXT NOT NULL UNIQUE,
            user_id TEXT NOT NULL,
            device_id TEXT NOT NULL,
            created_ts BIGINT NOT NULL,
            expired_ts BIGINT,
            invalidated BOOLEAN DEFAULT FALSE,
            FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE,
            FOREIGN KEY (device_id) REFERENCES devices(device_id) ON DELETE CASCADE
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS rooms (
            room_id TEXT NOT NULL PRIMARY KEY,
            is_public BOOLEAN NOT NULL DEFAULT FALSE,
            creator TEXT NOT NULL,
            creation_ts BIGINT NOT NULL,
            federate BOOLEAN NOT NULL DEFAULT TRUE,
            version TEXT NOT NULL DEFAULT '1',
            name TEXT,
            topic TEXT,
            avatar TEXT,
            canonical_alias TEXT,
            guest_access BOOLEAN DEFAULT FALSE,
            history_visibility TEXT DEFAULT 'shared',
            encryption TEXT,
            is_flaged BOOLEAN DEFAULT FALSE,
            is_spotlight BOOLEAN DEFAULT FALSE,
            deleted_ts BIGINT
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS room_memberships (
            room_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            sender TEXT NOT NULL,
            membership TEXT NOT NULL,
            event_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            display_name TEXT,
            avatar_url TEXT,
            is_banned BOOLEAN DEFAULT FALSE,
            invite_token TEXT,
            inviter TEXT,
            updated_ts BIGINT,
            joined_ts BIGINT,
            left_ts BIGINT,
            reason TEXT,
            PRIMARY KEY (room_id, user_id),
            FOREIGN KEY (room_id) REFERENCES rooms(room_id) ON DELETE CASCADE,
            FOREIGN KEY (user_id) REFERENCES users(name) ON DELETE CASCADE
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS room_events (
            event_id TEXT NOT NULL PRIMARY KEY,
            room_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            content TEXT NOT NULL,
            state_key TEXT,
            depth BIGINT NOT NULL DEFAULT 0,
            origin_server_ts BIGINT NOT NULL,
            processed_ts BIGINT NOT NULL,
            not_before BIGINT DEFAULT 0,
            status TEXT DEFAULT NULL,
            reference_image TEXT,
            origin TEXT NOT NULL,
            PRIMARY KEY (event_id),
            FOREIGN KEY (room_id) REFERENCES rooms(room_id) ON DELETE CASCADE,
            FOREIGN KEY (user_id) REFERENCES users(name) ON DELETE CASCADE
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS presence (
            user_id TEXT NOT NULL PRIMARY KEY,
            status_msg TEXT,
            presence TEXT NOT NULL DEFAULT 'offline',
            last_active_ts BIGINT NOT NULL DEFAULT 0,
            status_from TEXT,
            created_ts BIGINT NOT NULL,
            updated_ts BIGINT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(name) ON DELETE CASCADE
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS user_directory (
            user_id TEXT NOT NULL PRIMARY KEY,
            room_id TEXT NOT NULL,
            visibility TEXT NOT NULL DEFAULT 'private',
            added_by TEXT,
            created_ts BIGINT NOT NULL,
            PRIMARY KEY (user_id, room_id),
            FOREIGN KEY (user_id) REFERENCES users(name) ON DELETE CASCADE,
            FOREIGN KEY (room_id) REFERENCES rooms(room_id) ON DELETE CASCADE
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS push_rules (
            id BIGSERIAL PRIMARY KEY,
            user_id TEXT NOT NULL,
            rule_id TEXT NOT NULL,
            priority_class INTEGER NOT NULL DEFAULT 0,
            priority INTEGER NOT NULL DEFAULT 0,
            conditions TEXT,
            actions TEXT,
            is_default_rule BOOLEAN DEFAULT FALSE,
            is_enabled BOOLEAN DEFAULT TRUE,
            is_user_created BOOLEAN DEFAULT FALSE,
            created_ts BIGINT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(name) ON DELETE CASCADE
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS push_rules_user_sent_rules (
            id BIGSERIAL PRIMARY KEY,
            user_id TEXT NOT NULL,
            rule_id TEXT NOT NULL,
            enable BOOLEAN DEFAULT TRUE,
            FOREIGN KEY (user_id) REFERENCES users(name) ON DELETE CASCADE
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS痍
            sender TEXT NOT NULL,
            sent_to TEXT NOT NULL,
            room_id TEXT NOT NULL,
            event_id TEXT NOT NULL,
            sent_ts BIGINT NOT NULL,
            receipt_type TEXT NOT NULL,
            PRIMARY KEY (sent_to, sender, room_id),
            FOREIGN KEY (sender) REFERENCES users(name) ON DELETE CASCADE,
            FOREIGN KEY (sent_to) REFERENCES users(name) ON DELETE CASCADE,
            FOREIGN KEY (room_id) REFERENCES rooms(room_id) ON DELETE CASCADE
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS pusher_throttle (
            pusher TEXT NOT NULL PRIMARY KEY,
            last_sent_ts BIGINT NOT NULL,
            throttle_ms INTEGER NOT NULL DEFAULT 0
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS pushers (
            id BIGSERIAL PRIMARY KEY,
            user_id TEXT NOT NULL,
            access_token TEXT NOT NULL,
            profile_tag TEXT,
            kind TEXT NOT NULL,
            app_id TEXT NOT NULL,
            app_display_name TEXT,
            device_name TEXT,
            pushkey TEXT NOT NULL,
            ts BIGINT NOT NULL,
            language TEXT,
            data TEXT,
            expiry_ts BIGINT,
            FOREIGN KEY (user_id) REFERENCES users(name) ON DELETE CASCADE
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS ratelimit_shard (
            user_id TEXT NOT NULL PRIMARY KEY,
            shard_id INTEGER NOT NULL
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS user_filters (
            user_id TEXT NOT NULL,
            filter_id BIGINT NOT NULL,
            filter_definition TEXT NOT NULL,
            PRIMARY KEY (user_id, filter_id),
            FOREIGN KEY (user_id) REFERENCES users(name) ON DELETE CASCADE
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS user_ips (
            user_id TEXT NOT NULL,
            access_token TEXT NOT NULL,
            ip TEXT NOT NULL,
            user_agent TEXT,
            device_id TEXT NOT NULL,
            last_seen BIGINT NOT NULL,
            first_seen BIGINT NOT NULL DEFAULT 0
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS current_state_events (
            room_id TEXT NOT NULL,
            type TEXT NOT NULL,
            state_key TEXT NOT NULL,
            event_id TEXT NOT NULL,
            membership TEXT,
            depth BIGINT NOT NULL,
            stream_ordering BIGINT,
            PRIMARY KEY (room_id, type, state_key),
            FOREIGN KEY (room_id) REFERENCES rooms(room_id) ON DELETE CASCADE
        )
    "#).execute(pool).await?;

    Ok(())
}

pub mod user;
pub mod device;
pub mod token;
pub mod room;
pub mod membership;
pub mod event;

pub use user::*;
pub use device::*;
pub use token::*;
pub use room::*;
pub use membership::*;
pub use event::*;
