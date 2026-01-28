# Synapse Rust 增强功能标准化参考文档

> **版本**：1.0.0  
> **编制日期**：2026年1月28日  
> **文档状态**：正式发布  
> **适用范围**：各阶段开发工作指导

---

## 一、编码规范参考

### 1.1 文件组织规范

每个Rust模块应遵循统一的文件组织结构。模块根文件（mod.rs或模块名.rs）负责导出子模块和定义公共接口。子模块按功能划分，每个子模块包含实现代码和对应的测试代码。测试代码可以放在同一文件中（使用#[cfg(test)]标记）或单独的tests目录中。

对于增强功能模块，推荐的目录结构如下：

```
enhanced/
├── Cargo.toml                 # 模块配置文件
├── src/
│   ├── lib.rs                 # 库入口文件
│   ├── common/                # 公共模块
│   │   ├── mod.rs
│   │   ├── error.rs           # 错误类型定义
│   │   ├── config.rs          # 配置类型定义
│   │   └── crypto.rs          # 加密工具函数
│   ├── models/                # 数据模型
│   │   ├── mod.rs
│   │   ├── friend.rs          # 好友相关模型
│   │   ├── private_chat.rs    # 私聊相关模型
│   │   └── voice_message.rs   # 语音消息模型
│   ├── repository/            # 数据仓储
│   │   ├── mod.rs
│   │   ├── friend/            # 好友仓储实现
│   │   │   ├── mod.rs
│   │   │   ├── friend_repository.rs
│   │   │   └── ...
│   │   └── ...
│   ├── service/               # 业务服务
│   │   ├── mod.rs
│   │   ├── friend/            # 好友服务实现
│   │   │   ├── mod.rs
│   │   │   ├── friend_service.rs
│   │   │   └── ...
│   │   └── ...
│   ├── web/                   # Web层
│   │   ├── mod.rs
│   │   └── routes/            # API路由
│   │       ├── mod.rs
│   │       ├── friend.rs
│   │       └── ...
│   ├── crypto/                # 加密模块
│   │   ├── mod.rs
│   │   ├── nacl.rs
│   │   └── ...
│   └── media/                 # 媒体处理
│       ├── mod.rs
│       ├── audio_processor.rs
│       └── storage.rs
└── tests/                     # 集成测试
    ├── mod.rs
    └── ...
```

### 1.2 命名规范

**模块命名**：使用蛇形小写（snake_case），例如user_friend、private_chat。模块目录名与模块名一致。

**结构体命名**：使用帕斯卡命名（PascalCase），例如UserFriend、PrivateSession。数据模型、结构体和服务类都遵循此规范。

**枚举命名**：使用帕斯卡命名，例如ThreatSeverity、MessageType。枚举变体也使用帕斯卡命名，例如ThreatSeverity::Low。

**函数命名**：使用蛇形小写（snake_case），例如get_friends、send_message。测试函数可以使用更描述性的命名。

**常量命名**：使用全大写蛇形（SCREAMING_SNAKE_CASE），例如MAX_PARTICIPANTS、DEFAULT_TTL。

**类型参数命名**：使用简短驼峰（SingleCase），例如T、U、V。当类型参数有明确含义时，可以使用描述性名称，如T: Into<String>。

**特征命名**：使用形容词或名词，例如Storage、Service、Repository。特征名称应表达该特征提供的功能。

### 1.3 注释规范

**文档注释**：使用///标记的三斜线注释，用于文档生成。文档注释应包含功能说明、参数说明、返回值说明、示例代码和错误说明。

**行内注释**：使用//标记的注释，用于解释复杂的逻辑或不明显的代码。注释应解释「为什么」而非「是什么」。

**TODOs注释**：使用// TODO标记需要后续处理的问题。TODO注释应包含问题描述和预期处理时间。

**示例代码**：文档注释中的示例代码应使用```rust代码块标记，并确保代码可以正常编译运行。

```rust
/// 创建新的好友关系
///
/// 此方法会创建双向的好友关系记录。
///
/// # 参数
///
/// * `user_id` - 用户ID，格式为 @username:servername
/// * `friend_id` - 好友ID，格式为 @username:servername
/// * `remark` - 备注名，可选
///
/// # 返回
///
/// 返回创建的好友关系记录
///
/// # 错误
///
/// 如果好友关系已存在，返回 [`crate::common::ApiError::conflict`]
///
/// # 示例
///
/// ```rust
/// use crate::models::UserFriend;
///
/// let friend = UserFriend::new(
///     "@alice:localhost",
///     "@bob:localhost",
///     Some("Bob"),
/// );
/// ```
pub async fn create_friend(
    user_id: &str,
    friend_id: &str,
    remark: Option<&str>,
) -> Result<UserFriend, ApiError> {
    // 检查好友关系是否已存在
    // 使用事务锁防止并发创建冲突
    let mut tx = pool.begin().await?;
    // ... 实现代码
}
```

### 1.4 错误处理规范

**错误类型定义**：所有模块应定义模块专属的错误枚举，包含所有可能的错误类型。错误枚举应实现std::fmt::Display和std::error::Error trait。

**错误转换**：使用From trait实现错误类型之间的转换。标准错误（如sqlx::Error、redis::RedisError）应能转换为模块错误。

**错误传播**：使用?操作符传播错误，保持代码简洁。需要在特定位置添加上下文时，使用map_err或with_context方法。

```rust
#[derive(Debug, thiserror::Error)]
pub enum FriendError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Friend relation already exists: {user_id} <-> {friend_id}")]
    AlreadyExists {
        user_id: String,
        friend_id: String,
    },

    #[error("Friend relation not found: {user_id} <-> {friend_id}")]
    NotFound {
        user_id: String,
        friend_id: String,
    },

    #[error("Invalid user ID format: {0}")]
    InvalidUserId(String),

    #[error("Request processing failed: {0}")]
    RequestFailed(String),
}

pub type FriendResult<T> = Result<T, FriendError>;
```

---

## 二、数据库操作参考

### 2.1 SQL查询规范

**查询参数化**：所有用户输入必须使用参数化查询，防止SQL注入。参数使用$1、$2等占位符，不使用字符串拼接。

**类型匹配**：确保Rust类型与SQL类型匹配。使用sqlx::FromRow从查询结果转换为Rust类型。使用合适的类型转换函数。

**事务处理**：复合操作应使用事务包装，确保原子性。事务应尽快提交，避免长时间持有锁。

**错误处理**：数据库操作返回的错误应转换为模块自定义错误类型。提供足够的上下文信息便于问题诊断。

```rust
// 查询示例
pub async fn get_friend(
    &self,
    user_id: &str,
    friend_id: &str,
) -> Result<Option<UserFriend>, FriendError> {
    sqlx::query_as!(
        UserFriend,
        r#"
        SELECT id, user_id, friend_id, category_id, remark, status,
               created_at, updated_at
        FROM user_friends
        WHERE user_id = $1 AND friend_id = $2
        "#,
        user_id,
        friend_id
    )
    .fetch_optional(&self.pool)
    .await
    .map_err(FriendError::from)
}

// 插入示例
pub async fn create_friend(
    &self,
    user_id: &str,
    friend_id: &str,
    remark: Option<&str>,
) -> Result<UserFriend, FriendError> {
    let category_id = remark.map(|_| "default".to_string());

    sqlx::query_as!(
        UserFriend,
        r#"
        INSERT INTO user_friends (user_id, friend_id, category_id, remark)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
        user_id,
        friend_id,
        category_id,
        remark
    )
    .fetch_one(&self.pool)
    .await
    .map_err(FriendError::from)
}

// 事务示例
pub async fn add_friend_bidirectional(
    &self,
    user_id: &str,
    friend_id: &str,
    remark: Option<&str>,
) -> Result<(UserFriend, UserFriend), FriendError> {
    let mut tx = self.pool.begin().await?;

    let friend1 = sqlx::query_as!(
        UserFriend,
        r#"
        INSERT INTO user_friends (user_id, friend_id, remark)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
        user_id,
        friend_id,
        remark
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(FriendError::from)?;

    let friend2 = sqlx::query_as!(
        UserFriend,
        r#"
        INSERT INTO user_friends (user_id, friend_id, remark)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
        friend_id,
        user_id,
        remark.map(|r| format!("(回关){}", r))
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(FriendError::from)?;

    tx.commit().await?;
    Ok((friend1, friend2))
}
```

### 2.2 迁移脚本规范

每个迁移文件应包含向上迁移（up）和向下迁移（down）两个函数。迁移文件命名格式为{V序号}_{描述}.sql，例如2026012801_create_friend_tables.sql。

迁移脚本应检查表是否存在后再创建，避免重复执行失败。使用IF NOT EXISTS和CREATE INDEX IF NOT EXISTS语句。

```sql
-- 文件：migrations/2026012801_create_friend_tables.sql

-- 向上迁移：创建好友系统相关表
CREATE TABLE IF NOT EXISTS user_friends (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id TEXT NOT NULL,
    friend_id TEXT NOT NULL,
    category_id TEXT DEFAULT 'default',
    remark TEXT,
    status TEXT DEFAULT 'accepted',
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    UNIQUE(user_id, friend_id)
);

CREATE INDEX IF NOT EXISTS idx_user_friends_user_id ON user_friends(user_id);
CREATE INDEX IF NOT EXISTS idx_user_friends_friend_id ON user_friends(friend_id);
CREATE INDEX IF NOT EXISTS idx_user_friends_category ON user_friends(category_id);

CREATE TABLE IF NOT EXISTS friend_requests (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid(),
    requester_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    message TEXT,
    category_id TEXT,
    status TEXT DEFAULT 'pending',
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    expires_at TIMESTAMP DEFAULT NOW() + INTERVAL '7 days'
);

CREATE INDEX IF NOT EXISTS idx_friend_requests_requester ON friend_requests(requester_id);
CREATE INDEX IF NOT EXISTS idx_friend_requests_target ON friend_requests(target_id);
CREATE INDEX IF NOT EXISTS idx_friend_requests_status ON friend_requests(status);
CREATE INDEX IF NOT EXISTS idx_friend_requests_expires ON friend_requests(expires_at);

-- 向下迁移：删除好友系统相关表
DROP TABLE IF EXISTS friend_requests;
DROP TABLE IF EXISTS user_friends;
```

### 2.3 仓储模式实现参考

仓储模式将数据访问逻辑封装在独立的层中，实现业务逻辑与数据访问的分离。仓储trait定义数据访问接口，仓储实现提供具体的数据访问逻辑。

```rust
// 仓储 trait 定义
#[async_trait::async_trait]
pub trait FriendRepository: Send + Sync {
    async fn create(&self, friend: &UserFriend) -> Result<UserFriend, FriendError>;
    async fn get(&self, user_id: &str, friend_id: &str) -> Result<Option<UserFriend>, FriendError>;
    async fn delete(&self, user_id: &str, friend_id: &str) -> Result<bool, FriendError>;
    async fn list_by_user(
        &self,
        user_id: &str,
        category_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UserFriend>, FriendError>;
    async fn count_by_user(&self, user_id: &str, category_id: Option<&str>) -> Result<i64, FriendError>;
    async fn exists(&self, user_id: &str, friend_id: &str) -> Result<bool, FriendError>;
}

// 仓储实现
pub struct FriendRepositoryImpl {
    pool: PgPool,
}

impl FriendRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl FriendRepository for FriendRepositoryImpl {
    async fn create(&self, friend: &UserFriend) -> Result<UserFriend, FriendError> {
        // 实现代码...
    }

    async fn get(&self, user_id: &str, friend_id: &str) -> Result<Option<UserFriend>, FriendError> {
        // 实现代码...
    }

    // 其他方法...
}
```

---

## 三、API开发参考

### 3.1 Handler函数规范

每个API端点对应一个handler函数，handler负责从请求中提取参数、调用服务层执行业务逻辑、返回响应结果。handler应保持简洁，不包含业务逻辑。

```rust
// Handler 函数签名示例
async fn create_friend(
    State(state): State<AppState>,
    Json(body): Json<CreateFriendRequest>,
) -> Result<Json<FriendResponse>, ApiError> {
    // 参数验证
    if body.user_id.is_empty() {
        return Err(ApiError::bad_request("user_id cannot be empty".to_string()));
    }

    if body.friend_id.is_empty() {
        return Err(ApiError::bad_request("friend_id cannot be empty".to_string()));
    }

    // 调用服务层
    let friend = state
        .services
        .friend_service
        .add_friend(&body.user_id, &body.friend_id, body.remark.as_deref())
        .await
        .map_err(ApiError::from)?;

    // 返回响应
    Ok(Json(FriendResponse {
        id: friend.id,
        user_id: friend.user_id,
        friend_id: friend.friend_id,
        remark: friend.remark,
        status: friend.status,
        created_at: friend.created_at,
    }))
}
```

### 3.2 请求响应格式

**请求格式**：请求体使用JSON格式，字段命名使用snake_case。必填字段应在文档中明确标注。可选字段应提供合理的默认值。

**响应格式**：成功响应包含data字段存放业务数据，可选meta字段存放元信息。分页响应应包含limit、next_cursor等分页信息。

**错误响应**：错误响应包含errcode和error字段。errcode使用大写下划线格式，如M_NOT_FOUND。error是人类可读的错误描述。

```rust
// 请求结构体
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CreateFriendRequest {
    pub user_id: String,
    pub friend_id: String,
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SendFriendRequestRequest {
    pub requester_id: String,
    pub target_id: String,
    pub message: Option<String>,
    pub category_id: Option<String>,
}

// 响应结构体
#[derive(Debug, Serialize)]
pub struct FriendResponse {
    pub id: String,
    pub user_id: String,
    pub friend_id: String,
    pub remark: Option<String>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct FriendListResponse {
    pub friends: Vec<FriendResponse>,
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub next_cursor: Option<String>,
}

// 错误响应
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub errcode: String,
    pub error: String,
}
```

### 3.3 路由注册示例

```rust
use axum::{routing::{get, post, delete}, Router, extract::State, Json};
use serde_json::json;

// 创建好友路由
pub fn create_friend_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/friends", post(create_friend))
        .route("/friends/:user_id", get(list_friends))
        .route("/friends/:user_id/:friend_id", get(get_friend))
        .route("/friends/:user_id/:friend_id", delete(delete_friend))
        .route("/friend/requests", post(send_request))
        .route("/friend/requests/:request_id", post(respond_request))
        .route("/friend/requests/:user_id", get(list_requests))
        .route("/friend/categories", get(list_categories))
        .route("/friend/categories", post(create_category))
        .route("/friend/categories/:category_id", delete(delete_category))
        .with_state(state)
}
```

---

## 四、测试规范参考

### 4.1 单元测试示例

单元测试应覆盖核心业务逻辑，使用mock隔离外部依赖。测试应覆盖正常路径、边界条件和错误情况。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;
    use crate::common::ApiError;

    // 测试夹具
    struct TestFixture {
        pool: PgPool,
        repository: FriendRepositoryImpl,
    }

    async fn setup_test() -> TestFixture {
        let pool = setup_test_db().await;
        let repository = FriendRepositoryImpl::new(pool.clone());
        TestFixture { pool, repository }
    }

    #[tokio::test]
    async fn test_create_friend_success() {
        let fixture = setup_test().await;

        let friend = fixture
            .repository
            .create(&UserFriend::new(
                "@alice:localhost",
                "@bob:localhost",
                Some("Bob"),
            ))
            .await
            .expect("Failed to create friend");

        assert_eq!(friend.user_id, "@alice:localhost");
        assert_eq!(friend.friend_id, "@bob:localhost");
        assert_eq!(friend.remark, Some("Bob".to_string()));
        assert_eq!(friend.status, "accepted");
    }

    #[tokio::test]
    async fn test_create_duplicate_friend_fails() {
        let fixture = setup_test().await;

        // 创建第一个好友关系
        fixture
            .repository
            .create(&UserFriend::new("@alice:localhost", "@bob:localhost", None))
            .await
            .expect("Failed to create first friend");

        // 创建重复的好友关系应失败
        let result = fixture
            .repository
            .create(&UserFriend::new("@alice:localhost", "@bob:localhost", None))
            .await;

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, FriendError::AlreadyExists { .. }));
        }
    }

    #[tokio::test]
    async fn test_list_friends_with_pagination() {
        let fixture = setup_test().await;

        // 创建多个好友
        for i in 0..5 {
            fixture
                .repository
                .create(&UserFriend::new(
                    "@alice:localhost",
                    format!("@friend{}:localhost", i),
                    None,
                ))
                .await
                .expect("Failed to create friend");
        }

        // 分页查询
        let friends = fixture
            .repository
            .list_by_user("@alice:localhost", None, 2, 0)
            .await
            .expect("Failed to list friends");

        assert_eq!(friends.len(), 2);

        // 偏移查询
        let friends = fixture
            .repository
            .list_by_user("@alice:localhost", None, 2, 2)
            .await
            .expect("Failed to list friends");

        assert_eq!(friends.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_friend() {
        let fixture = setup_test().await;

        // 创建好友关系
        fixture
            .repository
            .create(&UserFriend::new("@alice:localhost", "@bob:localhost", None))
            .await
            .expect("Failed to create friend");

        // 删除好友关系
        let deleted = fixture
            .repository
            .delete("@alice:localhost", "@bob:localhost")
            .await
            .expect("Failed to delete friend");

        assert!(deleted);

        // 验证已删除
        let friend = fixture
            .repository
            .get("@alice:localhost", "@bob:localhost")
            .await
            .expect("Failed to get friend");

        assert!(friend.is_none());
    }
}
```

### 4.2 集成测试示例

集成测试验证完整的API流程，使用真实的数据库实例。测试应覆盖API端点的请求和响应。

```rust
#[tokio::test]
async fn test_create_friend_api() {
    let state = setup_test_state().await;
    let client = reqwest::Client::new();

    let response = client
        .post(&format!("{}/enhanced/friends", state.base_url))
        .json(&json!({
            "user_id": "@alice:localhost",
            "friend_id": "@bob:localhost",
            "remark": "Test Friend"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert_eq!(body["user_id"], "@alice:localhost");
    assert_eq!(body["friend_id"], "@bob:localhost");
    assert_eq!(body["remark"], "Test Friend");
}

#[tokio::test]
async fn test_list_friends_api() {
    let state = setup_test_state().await;
    let client = reqwest::Client::new();

    // 先创建一些好友关系
    create_test_friends(&state).await;

    let response = client
        .get(&format!(
            "{}/enhanced/friends/@alice:localhost?limit=10",
            state.base_url
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert!(body["friends"].is_array());
    assert!(body["friends"].as_array().unwrap().len() <= 10);
}

#[tokio::test]
async fn test_create_friend_validation() {
    let state = setup_test_state().await;
    let client = reqwest::Client::new();

    // 测试空用户ID
    let response = client
        .post(&format!("{}/enhanced/friends", state.base_url))
        .json(&json!({
            "user_id": "",
            "friend_id": "@bob:localhost"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 400);

    // 测试无效的好友ID格式
    let response = client
        .post(&format!("{}/enhanced/friends", state.base_url))
        .json(&json!({
            "user_id": "@alice:localhost",
            "friend_id": "invalid_id"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 400);
}
```

---

## 五、安全编码参考

### 5.1 输入验证

所有用户输入必须经过验证，确保类型、格式、大小符合预期。使用正则表达式验证特定格式，如用户ID、邮箱等。对特殊字符进行转义或过滤，防止注入攻击。

```rust
// 用户ID格式验证
fn validate_user_id(user_id: &str) -> Result<(), ApiError> {
    let pattern = r"^@[a-zA-Z0-9_-]+:[a-zA-Z0-9.-]+$";
    if !regex::Regex::new(pattern)
        .unwrap()
        .is_match(user_id)
    {
        return Err(ApiError::bad_request(format!(
            "Invalid user ID format: {}",
            user_id
        )));
    }
    Ok(())
}

// SQL注入防护：使用参数化查询
// 错误示例（不要使用）：
// let query = format!("SELECT * FROM users WHERE username = '{}'", username);
// 正确示例：
sqlx::query!("SELECT * FROM users WHERE username = $1", username)
    .fetch_one(&pool)
    .await?;

// XSS防护：对输出进行HTML转义
use ammonia::Builder;
let cleaner = Builder::default();
let safe_content = cleaner.clean(content).to_string();
```

### 5.2 加密操作

敏感数据必须加密存储，如密码哈希、会话密钥等。加密操作使用成熟的密码学库，如ring、RustCrypto、orion等。密钥管理遵循最小权限原则，密钥不硬编码在代码中。

```rust
// 密码哈希示例
use argon2::{hash_encoded, verify_encoded, Config};

pub fn hash_password(password: &str) -> Result<String, CryptoError> {
    let config = Config::default();
    let salt = generate_salt();
    hash_encoded(password.as_bytes(), &salt, &config)
        .map_err(|e| CryptoError::HashFailed(e.to_string()))
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, CryptoError> {
    verify_encoded(hash, password.as_bytes())
        .map_err(|e| CryptoError::VerifyFailed(e.to_string()))
}

// 端到端加密示例
use x25519_dalek::{EphemeralSecret, PublicKey, StaticSecret};
use rand::rngs::OsRng;

pub struct E2EEncryptor {
    private_key: StaticSecret,
}

impl E2EEncryptor {
    pub fn new() -> Self {
        let private_key = StaticSecret::new(OsRng);
        Self { private_key }
    }

    pub fn generate_ephemeral_keypair(&self) -> (PublicKey, EphemeralSecret) {
        let ephemeral_secret = EphemeralSecret::new(OsRng);
        let public_key = PublicKey::from(&ephemeral_secret);
        (public_key, ephemeral_secret)
    }

    pub fn encrypt(
        &self,
        message: &[u8],
        recipient_public: &PublicKey,
    ) -> Vec<u8> {
        // 加密实现
        let shared_secret = self.private_key.diffie_hellman(recipient_public);
        // 使用shared_secret加密消息
        message.to_vec() // 简化示例，实际应使用NaCl或libsodium
    }
}
```

### 5.3 审计日志

安全相关操作必须记录审计日志，包括操作时间、操作用户、操作类型、操作结果等。审计日志应独立存储，防止被篡改或删除。

```rust
pub struct AuditLogger {
    repository: SecurityEventRepository,
}

impl AuditLogger {
    pub async fn log(
        &self,
        event_type: &str,
        user_id: Option<&str>,
        ip_address: Option<&str>,
        severity: &str,
        description: &str,
        metadata: Option<serde_json::Value>,
    ) {
        let event = SecurityEvent::new(
            event_type,
            user_id.unwrap_or("system"),
            ip_address.unwrap_or("unknown"),
            severity,
            description,
            metadata,
        );

        if let Err(e) = self.repository.create(&event).await {
            tracing::error!("Failed to write audit log: {}", e);
        }
    }

    pub async fn log_friend_request(
        &self,
        requester_id: &str,
        target_id: &str,
        action: &str,
        ip_address: &str,
    ) {
        self.log(
            "friend_request",
            Some(requester_id),
            Some(ip_address),
            "info",
            &format!("Friend request {}: {} -> {}", action, requester_id, target_id),
            Some(json!({
                "target_id": target_id,
                "action": action
            })),
        ).await;
    }

    pub async fn log_security_event(
        &self,
        user_id: &str,
        ip_address: &str,
        threat_type: &str,
        severity: &str,
        details: &str,
    ) {
        self.log(
            "security_threat",
            Some(user_id),
            Some(ip_address),
            severity,
            &format!("Security threat detected: {}", threat_type),
            Some(json!({
                "threat_type": threat_type,
                "details": details
            })),
        ).await;
    }
}
```

---

## 六、性能优化参考

### 6.1 数据库查询优化

使用索引优化查询性能，避免全表扫描。优化复合查询，使用适当的连接和子查询。使用分页查询处理大量数据，避免一次性加载所有数据。

```rust
// 优化分页查询
pub async fn list_friends_paginated(
    &self,
    user_id: &str,
    cursor: Option<&str>,
    limit: i64,
) -> Result<(Vec<UserFriend>, Option<String>), FriendError> {
    // 使用游标分页，避免OFFSET的性能问题
    let query = if let Some(cursor) = cursor {
        sqlx::query!(
            r#"
            SELECT * FROM user_friends
            WHERE user_id = $1 AND created_at < $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
            user_id,
            cursor,
            limit
        )
    } else {
        sqlx::query!(
            r#"
            SELECT * FROM user_friends
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
            user_id,
            limit
        )
    };

    let friends = query
        .fetch_all(&self.pool)
        .await
        .map_err(FriendError::from)?;

    let next_cursor = friends.last().map(|f| f.created_at.to_string());

    Ok((friends.into_iter().map(|r| r.into()).collect(), next_cursor))
}

// 批量操作优化
pub async fn batch_create_friends(
    &self,
    friends: &[UserFriend],
) -> Result<Vec<UserFriend>, FriendError> {
    // 使用事务和批量插入
    let mut tx = self.pool.begin().await?;

    let mut results = Vec::new();
    for friend in friends {
        let result = sqlx::query_as!(
            UserFriend,
            r#"
            INSERT INTO user_friends (user_id, friend_id, remark)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, friend_id) DO NOTHING
            RETURNING *
            "#,
            friend.user_id,
            friend.friend_id,
            friend.remark
        )
        .fetch_one(&mut *tx)
        .await;

        if let Ok(f) = result {
            results.push(f.into());
        }
    }

    tx.commit().await?;
    Ok(results)
}
```

### 6.2 缓存策略

使用缓存减少数据库查询压力，提高响应速度。合理设置缓存过期时间，平衡数据新鲜度和缓存效率。使用多级缓存提高缓存命中率。

```rust
pub struct CachedFriendRepository {
    repository: FriendRepositoryImpl,
    cache: Arc<CacheManager>,
    cache_ttl: Duration,
}

#[async_trait::async_trait]
impl FriendRepository for CachedFriendRepository {
    async fn get(&self, user_id: &str, friend_id: &str) -> Result<Option<UserFriend>, FriendError> {
        let cache_key = format!("friend:{}:{}", user_id, friend_id);

        // 先查缓存
        if let Some(cached) = self.cache.get::<UserFriend>(&cache_key).await? {
            return Ok(Some(cached));
        }

        // 缓存未命中，查数据库
        let result = self.repository.get(user_id, friend_id).await?;

        // 写入缓存
        if let Some(ref friend) = result {
            self.cache.set(&cache_key, friend, Some(self.cache_ttl)).await?;
        }

        Ok(result)
    }

    async fn delete(&self, user_id: &str, friend_id: &str) -> Result<bool, FriendError> {
        let result = self.repository.delete(user_id, friend_id).await?;

        // 删除缓存
        if result {
            let cache_key = format!("friend:{}:{}", user_id, friend_id);
            self.cache.delete(&cache_key).await?;

            // 清除相关列表缓存
            let list_key = format!("friends:{}:list", user_id);
            self.cache.invalidate_pattern(&list_key).await?;
        }

        Ok(result)
    }
}
```

### 6.3 并发处理

使用异步编程提高并发处理能力。使用连接池管理数据库连接，避免频繁创建和销毁连接。使用信号量控制并发数量，防止系统过载。

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

pub struct ConcurrentFriendService {
    repository: Arc<CachedFriendRepository>,
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
}

impl ConcurrentFriendService {
    pub fn new(repository: Arc<CachedFriendRepository>, max_concurrent: usize) -> Self {
        Self {
            repository,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
        }
    }

    pub async fn batch_add_friends(
        &self,
        user_id: &str,
        friend_ids: Vec<String>,
    ) -> Vec<Result<String, FriendError>> {
        let mut handles = Vec::new();

        for friend_id in friend_ids {
            let permit = self.semaphore.clone().acquire_owned();
            let repository = self.repository.clone();
            let user_id = user_id.to_string();
            let friend_id = friend_id.clone();

            handles.push(tokio::spawn(async move {
                let _permit = permit.await;
                repository.create(&UserFriend::new(&user_id, &friend_id, None)).await
            }));
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(friend)) => results(Ok(friend.friend_id)),
                Ok(Err(e)) => results(Err(e)),
                Err(e) => results(Err(FriendError::from(e))),
            }
        }

        results
    }
}
```

---

## 附录

### A. 开发环境检查清单

开发环境配置完成后，应验证以下各项：

Rust工具链版本不低于1.75，cargo、rustc、rustfmt、clippy等工具已安装。PostgreSQL 15或更高版本已安装运行，数据库连接正常。Redis 7或更高版本已安装运行，连接正常。IDE已配置rust-analyzer扩展，代码补全和错误提示正常。代码格式化工具已配置，保存时自动格式化。Git已配置，提交前自动运行格式化检查。

### B. 代码审查清单

代码合并前，审查者应检查以下各项：

代码功能符合需求规格描述。代码格式符合rustfmt规范，无clippy警告。单元测试覆盖率达标，测试用例合理。API响应格式符合规范。错误处理完整，无未处理的错误边界。代码注释清晰，公共API有文档注释。性能考虑合理，无明显的性能问题。安全考虑周全，无安全漏洞。

### C. 发布检查清单

功能发布前，应检查以下各项：

所有功能测试用例通过。性能测试达标。安全测试通过。文档已更新。部署脚本已测试。回滚方案已准备。监控告警已配置。

---

**编制人**：[待填写]  
**审核人**：[待填写]  
**批准人**：[待填写]
