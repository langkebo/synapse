# Synapse Rust 测试策略文档

> **版本**：1.0.0  
> **编制日期**：2026-01-28  
> **状态**：草稿  
> **适用范围**：Synapse Rust 项目所有测试活动

---

## 一、测试策略概述

### 1.1 测试目标

本测试策略旨在确保 Synapse Rust 项目的代码质量、功能正确性和性能可靠性。通过系统化的测试活动，我们期望达成以下目标：确保所有 API 端点按照 Matrix 规范正确实现；确保模块间的接口调用符合设计规范；确保系统在正常负载和峰值负载下稳定运行；确保代码变更不会引入新的缺陷。

测试策略遵循测试金字塔原则，底层以大量快速的单元测试为主，中层进行模块间的集成测试，顶层进行端到端的系统测试和性能测试。这种分布确保了测试的高效性和覆盖率的平衡。

### 1.2 测试层次

本项目采用四层测试体系，各层测试的目的、范围和执行频率有所不同。

**单元测试层**是测试金字塔的基座，数量最多、速度最快。每个单元测试测试单个函数或方法的行为，测试范围局限于单个模块，不涉及外部依赖。单元测试应覆盖所有公开 API 和复杂的内部逻辑，覆盖率目标不低于 80%。单元测试在每次代码提交时自动执行。

**集成测试层**测试多个模块之间的协作，确保模块间的接口调用正确。集成测试使用真实的数据库实例，验证数据访问层与业务逻辑层的交互。集成测试覆盖率目标不低于 60%，在每次合并请求时执行。

**API 测试层**测试所有 HTTP API 端点，验证请求和响应的格式符合 Matrix 规范。API 测试覆盖所有端点的正常路径和主要错误路径，覆盖率目标为 100%。API 测试在每次合并请求和每日构建时执行。

**性能测试层**测试系统在正常负载和峰值负载下的表现，包括响应时间、吞吐量、资源利用率等指标。性能测试在发布前和重大功能完成后执行，结果用于识别性能瓶颈和优化机会。

### 1.3 测试环境

测试环境的配置应尽可能接近生产环境，以减少环境差异导致的测试结果偏差。开发人员的本地环境使用 Docker Compose 启动测试所需的数据库和缓存服务；CI 环境使用隔离的测试实例，每个测试运行使用独立的数据库；性能测试环境使用专用的测试服务器，配置与生产环境一致。

数据库使用 PostgreSQL 15，缓存使用 Redis 7。测试数据在每次测试运行前初始化，测试结束后清理，确保测试之间的隔离性。

---

## 二、单元测试规范

### 2.1 测试组织

单元测试代码与源代码放在同一模块中，使用 #[cfg(test)] 属性标记。测试模块位于源文件的末尾，与业务代码保持适当的分离。

```rust
// src/storage/user.rs

pub struct UserStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> UserStorage<'a> {
    pub async fn create_user(&self, username: &str, password_hash: &str, admin: bool) -> Result<User, sqlx::Error> {
        // 实现代码...
    }

    pub async fn get_by_id(&self, user_id: &str) -> Result<Option<User>, sqlx::Error> {
        // 实现代码...
    }
}

// 以下是单元测试
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    // 测试夹具：创建测试数据库连接
    async fn setup_test_pool() -> PgPool {
        let url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://synapse:synapse@localhost:5432/synapse_test".to_string());
        PgPool::connect(&url).await.unwrap()
    }

    #[tokio::test]
    async fn test_create_user_success() {
        let pool = setup_test_pool().await;
        let storage = UserStorage::new(&pool);

        let user = storage.create_user("testuser", "hash123", false).await.unwrap();

        assert_eq!(user.username, "testuser");
        assert!(!user.admin);
        assert!(user.user_id.starts_with("@testuser:"));
    }

    #[tokio::test]
    async fn test_create_duplicate_user_fails() {
        let pool = setup_test_pool().await;
        let storage = UserStorage::new(&pool);

        // 创建第一个用户成功
        storage.create_user("duplicate", "hash1", false).await.unwrap();

        // 创建同名用户失败
        let result = storage.create_user("duplicate", "hash2", false).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_existing_user() {
        let pool = setup_test_pool().await;
        let storage = UserStorage::new(&pool);

        // 创建用户
        let created = storage.create_user("getuser", "hash", false).await.unwrap();

        // 查询用户
        let fetched = storage.get_by_id(&created.user_id).await.unwrap();

        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().username, "getuser");
    }

    #[tokio::test]
    async fn test_get_nonexistent_user() {
        let pool = setup_test_pool().await;
        let storage = UserStorage::new(&pool);

        let result = storage.get_by_id("@nonexistent:localhost").await.unwrap();

        assert!(result.is_none());
    }
}
```

### 2.2 测试数据管理

单元测试应避免相互干扰，每个测试独立运行。使用测试夹具（fixtures）管理测试数据的创建和清理。对于需要特定状态的测试，在测试开始时设置所需数据，在测试结束后清理。

对于涉及数据库的测试，建议使用事务回滚而非物理删除：在测试开始时启动事务，在测试结束时回滚事务。这种方式既保证了测试隔离性，又避免了频繁的表操作。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{Acquire, Postgres};

    // 使用事务回滚的测试夹具
    async fn with_transaction<F, T>(pool: &PgPool, test: F) -> T
    where
        F: FnOnce() -> T,
        F: std::future::Future<Output = T>,
    {
        let mut conn = pool.acquire().await.unwrap();
        let tx = conn.begin().await.unwrap();

        // 在事务中执行测试
        let result = test().await;

        // 回滚事务，撤销所有更改
        tx.rollback().await.unwrap();

        result
    }

    #[tokio::test]
    async fn test_user_operations_with_rollback() {
        let pool = setup_test_pool().await;

        with_transaction(&pool, async {
            let storage = UserStorage::new(&pool);

            // 这个测试中的所有数据库操作都会在测试结束时回滚
            let user = storage.create_user("txuser", "hash", false).await.unwrap();
            assert_eq!(user.username, "txuser");

            // 验证用户存在
            let fetched = storage.get_by_id(&user.user_id).await.unwrap();
            assert!(fetched.is_some());
        }).await;

        // 事务已回滚，用户记录不存在
        let storage = UserStorage::new(&pool);
        let result = storage.get_by_id("@txuser:localhost").await.unwrap();
        assert!(result.is_none());
    }
}
```

### 2.3 Mock 和 Stub

对于外部依赖（如 HTTP 服务、第三方库），使用 Mock 或 Stub 隔离测试。使用 mockall 或其他模拟库创建测试替身。

```rust
// 使用 mockall 创建 Mock Storage
#[cfg(test)]
mod mock_tests {
    use mockall::{mock, predicate::*};

    mock! {
        pub UserStorage {
            async fn create_user(&self, username: &str, password_hash: &str, admin: bool) -> Result<User, sqlx::Error>;
            async fn get_by_id(&self, user_id: &str) -> Result<Option<User>, sqlx::Error>;
        }
    }

    #[tokio::test]
    async fn test_auth_service_with_mock_storage() {
        let mut storage = MockUserStorage::new();

        // 设置 Mock 行为
        let expected_user = User {
            user_id: "@test:localhost".to_string(),
            username: "test".to_string(),
            password_hash: Some("$argon2$...".to_string()),
            admin: false,
            ..Default::default()
        };

        storage.expect_create_user()
            .with(eq("test"), eq("hashed_password"), eq(false))
            .returning(Ok(expected_user.clone()));

        storage.expect_get_by_id()
            .returning(Ok(Some(expected_user)));

        // 测试使用 Mock Storage
        let auth_service = AuthService::new(Box::new(storage), /* 其他依赖 */);

        let result = auth_service.login("test", "password").await;
        assert!(result.is_ok());
    }
}
```

---

## 三、集成测试规范

### 3.1 模块集成测试

集成测试验证多个模块协同工作的正确性。测试从存储层到服务层到 Web 层的完整调用链路。

```rust
// tests/integration/user_service_test.rs

use synapse_rust::prelude::*;
use test_context::TestContext;

struct UserServiceContext {
    pool: PgPool,
    user_storage: UserStorage<'static>,
    auth_service: AuthService,
}

impl TestContext for UserServiceContext {
    fn setup() -> Self {
        let pool = setup_test_db().await;
        let cache = Arc::new(CacheManager::new(CacheConfig::default()));
        let user_storage = UserStorage::new(&pool);
        let auth_service = AuthService::new(&pool, cache, "test_secret", "localhost");

        Self {
            pool,
            user_storage,
            auth_service,
        }
    }

    fn teardown(&self) {
        // 清理测试数据
        cleanup_test_data(&self.pool);
    }
}

#[tokio::test]
async fn test_full_registration_flow(ctx: &UserServiceContext) {
    // 1. 测试注册流程
    let register_result = ctx.auth_service.register(
        "newuser",
        "securepassword123",
        false,
        Some("New User"),
    ).await;

    assert!(register_result.is_ok());
    let (user, access_token, refresh_token, device_id) = register_result.unwrap();

    assert_eq!(user.username, "newuser");
    assert!(!access_token.is_empty());
    assert!(!refresh_token.is_empty());
    assert!(!device_id.is_empty());

    // 2. 测试登录流程
    let login_result = ctx.auth_service.login(
        "newuser",
        "securepassword123",
        None,
        None,
    ).await;

    assert!(login_result.is_ok());
    let (logged_in_user, login_token, _, _) = login_result.unwrap();

    assert_eq!(logged_in_user.user_id, user.user_id);
    assert_ne!(login_token, access_token); // 每次登录生成新令牌

    // 3. 验证令牌有效性
    let token_claims = ctx.auth_service.validate_token(&login_token).await;
    assert!(token_claims.is_ok());
    assert_eq!(token_claims.unwrap().user_id, user.user_id);
}
```

### 3.2 数据库集成测试

数据库集成测试验证 SQL 查询和事务的正确性。测试应覆盖正常的 CRUD 操作、边界条件和错误处理。

```rust
// tests/integration/database_test.rs

use sqlx::{PgPool, Postgres};

async fn setup_test_tables(pool: &PgPool) {
    sqlx::query("TRUNCATE TABLE users CASCADE").execute(pool).await.unwrap();
    sqlx::query("TRUNCATE TABLE devices CASCADE").execute(pool).await.unwrap();
}

#[tokio::test]
async fn test_user_device_relationship() {
    let pool = setup_test_db().await;
    setup_test_tables(&pool).await;

    let user_storage = UserStorage::new(&pool);
    let device_storage = DeviceStorage::new(&pool);

    // 1. 创建用户
    let user = user_storage.create_user("reluser", "hash", false).await.unwrap();

    // 2. 创建设备（关联用户）
    let device1 = device_storage.create_device("DEVICE1", &user.user_id, Some("Phone")).await.unwrap();
    let device2 = device_storage.create_device("DEVICE2", &user.user_id, Some("Desktop")).await.unwrap();

    assert_eq!(device1.user_id, user.user_id);
    assert_eq!(device2.user_id, user.user_id);

    // 3. 查询用户的设备列表
    let devices = device_storage.get_user_devices(&user.user_id).await.unwrap();
    assert_eq!(devices.len(), 2);

    // 4. 删除用户（应级联删除设备）
    user_storage.delete_user(&user.user_id).await.unwrap();

    let devices_after_delete = device_storage.get_user_devices(&user.user_id).await.unwrap();
    assert!(devices_after_delete.is_empty());
}
```

---

## 四、API 测试规范

### 4.1 HTTP API 测试

API 测试验证 HTTP 端点的请求处理和响应格式。使用 reqwest 库发送 HTTP 请求，使用 wiremock 或实际服务器进行测试。

```rust
// tests/api/client_api_test.rs

use reqwest;
use serde_json::json;
use testcontainers::clients::Cli;
use testcontainers::images::postgres::Postgres;
use testcontainers::images::redis::Redis;

struct ApiTestContext {
    docker: Cli,
    postgres: testcontainers::Container<'static, Postgres>,
    redis: testcontainers::Container<'static, Redis>,
    server_url: String,
    admin_token: String,
}

impl ApiTestContext {
    async fn new() -> Self {
        let docker = Cli::default();

        // 启动 PostgreSQL
        let postgres = docker.run(Postgres::default());
        let postgres_url = format!(
            "postgres://postgres:postgres@localhost:{}/postgres",
            postgres.get_host_port_ipv4(5432)
        );

        // 启动 Redis
        let redis = docker.run(Redis::default());
        let redis_url = format!(
            "redis://localhost:{}",
            redis.get_host_port_ipv4(6379)
        );

        // 启动 Synapse 服务器
        let server = SynapseServer::new(
            &postgres_url,
            "localhost",
            "test_secret",
            "0.0.0.0:0",
            std::path::PathBuf::from("/tmp/media"),
        ).await.unwrap();

        let server_url = format!("http://localhost:{}", server.port());

        // 创建管理员用户
        let admin_token = create_admin_user(&server_url).await;

        Self {
            docker,
            postgres,
            redis,
            server_url,
            admin_token,
        }
    }

    async fn cleanup(&self) {
        // 清理资源
    }
}

#[tokio::test]
async fn test_client_version_endpoint() {
    let ctx = ApiTestContext::new().await;
    defer { ctx.cleanup(); }

    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/_matrix/client/versions", ctx.server_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["versions"].is_array());
}

#[tokio::test]
async fn test_user_registration() {
    let ctx = ApiTestContext::new().await;
    defer { ctx.cleanup(); }

    let client = reqwest::Client::new();

    // 1. 测试注册新用户
    let register_response = client
        .post(&format!("{}/_matrix/client/r0/register", ctx.server_url))
        .json(&json!({
            "username": "testuser",
            "password": "testpassword123",
            "displayname": "Test User"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(register_response.status(), 200);
    let register_body: serde_json::Value = register_response.json().await.unwrap();

    assert_eq!(register_body["user_id"], "@testuser:localhost");
    assert!(register_body["access_token"].is_string());
    assert!(register_body["device_id"].is_string());

    // 2. 测试重复注册应失败
    let duplicate_response = client
        .post(&format!("{}/_matrix/client/r0/register", ctx.server_url))
        .json(&json!({
            "username": "testuser",
            "password": "anotherpassword"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(duplicate_response.status(), 400);
    let error_body: serde_json::Value = duplicate_response.json().await.unwrap();
    assert_eq!(error_body["errcode"], "M_USER_IN_USE");
}

#[tokio::test]
async fn test_user_login() {
    let ctx = ApiTestContext::new().await;
    defer { ctx.cleanup(); }

    // 先注册用户
    let client = reqwest::Client::new();
    client
        .post(&format!("{}/_matrix/client/r0/register", ctx.server_url))
        .json(&json!({
            "username": "loginuser",
            "password": "password123"
        }))
        .send()
        .await
        .unwrap();

    // 登录
    let login_response = client
        .post(&format!("{}/_matrix/client/r0/login", ctx.server_url))
        .json(&json!({
            "type": "m.login.password",
            "user": "loginuser",
            "password": "password123"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(login_response.status(), 200);
    let login_body: serde_json::Value = login_response.json().await.unwrap();

    assert!(login_body["access_token"].is_string());
    assert!(login_body["device_id"].is_string());
    assert_eq!(login_body["user_id"], "@loginuser:localhost");
}

#[tokio::test]
async fn test_room_creation() {
    let ctx = ApiTestContext::new().await;
    defer { ctx.cleanup(); }

    let client = reqwest::Client::new();

    // 注册并登录
    let register_response = client
        .post(&format!("{}/_matrix/client/r0/register", ctx.server_url))
        .json(&json!({
            "username": "roomuser",
            "password": "password123"
        }))
        .send()
        .await
        .unwrap();
    let register_body: serde_json::Value = register_response.json().await.unwrap();
    let access_token = register_body["access_token"].as_str().unwrap();

    // 创建房间
    let room_response = client
        .post(&format!("{}/_matrix/client/r0/createRoom", ctx.server_url))
        .bearer_auth(access_token)
        .json(&json!({
            "visibility": "private",
            "name": "Test Room",
            "topic": "A test room for API testing"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(room_response.status(), 200);
    let room_body: serde_json::Value = room_response.json().await.unwrap();

    assert!(room_body["room_id"].is_string());
    assert_eq!(room_body["room_id"], format!("!{}:localhost", room_body["room_id"].as_str().unwrap().trim_start_matches('!')));
}
```

### 4.2 错误响应测试

测试各种错误情况下的响应格式和状态码，确保错误处理的一致性。

```rust
#[tokio::test]
async fn test_error_responses() {
    let ctx = ApiTestContext::new().await;
    defer { ctx.cleanup(); }

    let client = reqwest::Client::new();

    // 测试 400 Bad Request
    let bad_request = client
        .post(&format!("{}/_matrix/client/r0/register", ctx.server_url))
        .json(&json!({
            "username": "",  // 空用户名
            "password": "short"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(bad_request.status(), 400);
    let error: serde_json::Value = bad_request.json().await.unwrap();
    assert!(error["errcode"].is_string());
    assert!(error["error"].is_string());

    // 测试 401 Unauthorized（无效令牌）
    let unauthorized = client
        .get(&format!("{}/_matrix/client/r0/account/whoami", ctx.server_url))
        .bearer_auth("invalid_token")
        .send()
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), 401);

    // 测试 404 Not Found
    let not_found = client
        .get(&format!("{}/_matrix/client/r0/rooms/!nonexistent:localhost/messages", ctx.server_url))
        .bearer_auth(ctx.admin_token)
        .send()
        .await
        .unwrap();
    assert_eq!(not_found.status(), 404);
}
```

---

## 五、性能测试规范

### 5.1 性能指标定义

性能测试关注以下核心指标：

**响应时间**衡量从请求发送到响应接收的端到端延迟。使用百分位数指标：P50（中位数）、P95、p99，反映不同负载水平下的典型响应时间。目标为 P95 响应时间不超过 10 毫秒。

**吞吐量**衡量系统单位时间内处理的请求数量。以每秒请求数（RPS）计量，目标是峰值负载下维持 10000 RPS。

**资源利用率**衡量 CPU、内存、网络、磁盘的使用情况。目标是 CPU 利用率不超过 80%（保留余量应对突发流量），内存占用稳定后不超过 300MB。

**并发能力**衡量系统同时处理的最大用户数。目标是支持 1000 并发用户同时在线。

### 5.2 负载测试

负载测试验证系统在预期负载下的表现。模拟正常负载和峰值负载，验证系统稳定性和性能指标。

```rust
// tests/performance/load_test.rs

use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::Semaphore;
use rand::Rng;

struct LoadTestConfig {
    pub target_rps: u32,           // 目标每秒请求数
    pub duration_secs: u64,        // 测试持续时间
    pub concurrent_users: usize,   // 并发用户数
    pub warmup_secs: u64,          // 预热时间
}

async fn run_load_test(config: &LoadTestConfig, endpoint: &str, token: &str) -> LoadTestResult {
    let client = Arc::new(reqwest::Client::new());
    let semaphore = Arc::new(Semaphore::new(config.concurrent_users));
    let start_time = Instant::now();

    let mut handles = Vec::new();
    let mut request_count = 0;
    let mut error_count = 0;
    let mut latencies = Vec::new();
    let lock = Arc::new(tokio::sync::Mutex::new(0));

    // 预热阶段
    println!("Warming up for {} seconds...", config.warmup_secs);
    tokio::time::sleep(Duration::from_secs(config.warmup_secs)).await;

    // 负载测试阶段
    let test_end = start_time.elapsed().as_secs() + config.duration_secs;

    while start_time.elapsed().as_secs() < test_end {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client = client.clone();
        let endpoint = endpoint.to_string();
        let token = token.to_string();
        let lock = lock.clone();

        let handle = tokio::spawn(async move {
            let req_start = Instant::now();

            let result = client
                .get(&endpoint)
                .bearer_auth(&token)
                .send()
                .await;

            let req_duration = req_start.elapsed();

            let mut guard = lock.lock().await;
            match result {
                Ok(_) => {
                    request_count += 1;
                    latencies.push(req_duration);
                }
                Err(_) => {
                    error_count += 1;
                }
            }
            drop(guard);
            drop(permit);
        });

        handles.push(handle);

        // 控制请求速率
        if request_count % 100 == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    // 等待所有请求完成
    for handle in handles {
        handle.await.unwrap();
    }

    // 计算结果
    let total_duration = start_time.elapsed();
    let actual_rps = request_count as f64 / total_duration.as_secs_f64();

    let mut latencies_ms: Vec<f64> = latencies.iter()
        .map(|d| d.as_secs_f64() * 1000.0)
        .collect();
    latencies_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let p50 = percentile(&latencies_ms, 50.0);
    let p95 = percentile(&latencies_ms, 95.0);
    let p99 = percentile(&latencies_ms, 99.0);

    LoadTestResult {
        total_requests: request_count,
        error_count,
        actual_rps,
        duration_secs: total_duration.as_secs(),
        p50_ms: p50,
        p95_ms: p95,
        p99_ms: p99,
    }
}

fn percentile(data: &[f64], p: f64) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let idx = (p / 100.0 * (data.len() - 1) as f64) as usize;
    let fraction = p / 100.0 * (data.len() - 1) as f64 - idx as f64;
    data[idx] * (1.0 - fraction) + data[idx + 1] * fraction
}

struct LoadTestResult {
    total_requests: usize,
    error_count: usize,
    actual_rps: f64,
    duration_secs: u64,
    p50_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
}
```

### 5.3 压力测试

压力测试验证系统在超出正常负载情况下的行为，确定系统的极限容量和故障模式。

```rust
async fn run_stress_test() {
    let config = LoadTestConfig {
        target_rps: 50000,  // 高负载
        duration_secs: 60,
        concurrent_users: 1000,
        warmup_secs: 5,
    };

    // 逐步增加负载
    let mut current_rps = 1000;
    while current_rps <= 50000 {
        println!("Testing at {} RPS...", current_rps);
        let result = run_load_test(&config, "/_matrix/client/versions", "").await;

        println!("  RPS: {:.2}, P95: {:.2}ms, Errors: {}",
            result.actual_rps, result.p95_ms, result.error_count);

        if result.error_count as f64 / result.total_requests as f64 > 0.01 {
            println!("  Error rate exceeds 1%, stopping stress test");
            break;
        }

        current_rps *= 2;
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
```

---

## 六、测试工具与环境

### 6.1 测试工具清单

| 工具 | 用途 | 安装方式 |
|------|------|----------|
| cargo test | 运行单元测试和集成测试 | Rust 工具链自带 |
| cargo nextest | 增强的测试运行器，支持并行执行 | cargo install cargo-nextest |
| cargo bench | 运行性能基准测试 | Rust 工具链自带 |
| reqwest | HTTP 客户端，用于 API 测试 | Cargo 依赖 |
| sqlx-cli | SQLx 命令行工具，用于数据库迁移 | cargo install sqlx-cli |
| mockall | Rust Mock 库 | Cargo 依赖 |
| testcontainers | Docker 容器管理，用于集成测试 | Cargo 依赖 |

### 6.2 测试配置

环境变量用于配置测试行为：

| 环境变量 | 说明 | 默认值 |
|----------|------|--------|
| TEST_DATABASE_URL | 测试数据库连接 URL | postgres://synapse:synapse@localhost:5432/synapse_test |
| TEST_REDIS_URL | 测试 Redis 连接 URL | redis://localhost:6379 |
| TEST_SERVER_URL | 测试服务器 URL | 从测试服务器获取 |
| TEST_LOG_LEVEL | 测试日志级别 | debug |

### 6.3 CI 测试流程

在持续集成环境中，测试流程分为以下步骤：

第一步进行代码检查，运行 cargo fmt 检查代码格式，cargo clippy 检查代码质量，确保代码符合规范。第二步进行单元测试，运行 cargo test --lib 和 cargo test --bins，执行所有单元测试，生成覆盖率报告。第三步进行集成测试，运行 cargo test --test '*' 执行集成测试，使用 testcontainers 启动测试数据库。第四步进行 API 测试，执行预编译的测试服务器，运行 API 测试套件。第五步进行性能测试，在专用环境中运行负载测试，记录性能指标与基准对比。

---

## 七、测试覆盖率要求

### 7.1 覆盖率指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 行覆盖率 | 80% | 被测试执行覆盖的代码行数占比 |
| 分支覆盖率 | 70% | 条件分支被覆盖的比例 |
| 函数覆盖率 | 90% | 被测试调用的函数占比 |
| 路径覆盖率 | 60% | 代码执行路径被覆盖的比例 |

### 7.2 覆盖率报告

使用 tarpaulin 生成测试覆盖率报告：

```bash
# 生成覆盖率报告
cargo tarpaulin --out Html --output-dir ./coverage

# 查看覆盖率
open ./coverage/tarpaulin-report.html
```

覆盖率报告应在每次合并请求时生成，并与基准覆盖率对比。覆盖率下降超过 5% 应触发审查警告。

---

## 八、附录

### 8.1 常用测试命令

```bash
# 运行所有测试
cargo test

# 运行指定模块的测试
cargo test --lib storage::user

# 运行集成测试
cargo test --test integration

# 运行 API 测试
cargo test --test api

# 运行性能测试
cargo bench

# 运行测试并生成覆盖率报告
cargo tarpaulin --out Html

# 使用 nextest 运行测试（更快）
cargo nextest run

# 并行运行测试
cargo test --jobs 4
```

### 8.2 测试夹具定义

测试夹具（Fixtures）定义了一组标准的测试数据和状态，用于确保测试的一致性和可重复性。

| 夹具名称 | 说明 | 包含数据 |
|----------|------|----------|
| empty_db | 空数据库 | 无用户、无房间、无事件 |
| users | 包含测试用户 | 5 个普通用户，2 个管理员用户 |
| rooms | 包含测试房间 | 3 个公开房间，2 个私有房间 |
| messages | 包含测试消息 | 每个房间 10 条消息 |
| federation | 联邦测试数据 | 远程服务器信息、跨房间事件 |

### 8.3 测试数据生成器

测试数据生成器用于创建大量测试数据，支持性能测试和边界测试：

```rust
// tests/utils/data_generator.rs

pub struct TestDataGenerator {
    base_index: AtomicUsize::new(0),
}

impl TestDataGenerator {
    pub fn new() -> Self {
        Self {
            base_index: AtomicUsize::new(0),
        }
    }

    pub fn next_user(&self) -> (String, String, String) {
        let index = self.base_index.fetch_add(1, Ordering::SeqCst);
        let username = format!("testuser_{:04x}", index);
        let user_id = format!("@{}:localhost", username);
        let password = format!("password_{:04x}", index);
        (user_id, username, password)
    }

    pub fn next_room(&self) -> (String, String) {
        let index = self.base_index.fetch_add(1, Ordering::SeqCst);
        let room_id = format!("!room_{:04x}:localhost", index);
        let room_name = format!("Test Room {}", index);
        (room_id, room_name)
    }

    pub fn generate_users(&self, count: usize) -> Vec<(String, String, String)> {
        (0..count).map(|_| self.next_user()).collect()
    }

    pub fn generate_rooms(&self, count: usize) -> Vec<(String, String)> {
        (0..count).map(|_| self.next_room()).collect()
    }
}
```

---

**编制人**：  
**审核人**：  
**批准人**：  
