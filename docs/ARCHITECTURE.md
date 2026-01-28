# Synapse Rust 架构设计文档

> **版本**：1.1.0  
> **编制日期**：2026-01-28  
> **状态**：正式发布  
> **参考标准**：[Synapse 官方文档](https://element-hq.github.io/synapse/latest/)、[Matrix 规范](https://spec.matrix.org/)

---

## 一、架构概述

### 1.1 设计原则

本项目的架构设计遵循以下核心原则，确保系统具备高性能、高可靠性和良好的可维护性。

**分层架构原则**：系统采用清晰的分层架构，从下到上依次为数据持久层、缓存层、业务逻辑层、Web 表现层。各层之间通过明确定义的接口进行通信，层与层之间的依赖关系严格遵循自上而下的方向，避免循环依赖。这种分层设计使得各层可以独立开发和测试，降低了模块间的耦合度。

**异步优先原则**：Rust 的异步编程模型允许在单线程内并发处理大量 I/O 操作。本项目所有 I/O 操作（数据库访问、网络请求、文件读写）均采用异步方式实现，充分利用 Tokio 运行时的高效调度能力。异步代码使用 async/await 语法糖，配合合适的 Future 组合器，避免回调地狱，提高代码可读性。

**错误处理原则**：项目采用 Result 类型进行错误传播，所有可能失败的函数都返回 Result 类型。上层调用者可以选择使用 ? 操作符向上传播错误，或者使用 match 表达式进行错误处理。定义统一的 ApiError 类型，包含错误码、错误消息和 HTTP 状态码，便于上层转换为标准化的错误响应。

**配置分离原则**：将代码与配置分离，所有可配置的参数都提取到配置文件中。配置支持多级优先级：默认值 < 配置文件 < 环境变量 < 命令行参数。这种设计使得同一套代码可以部署到不同环境，无需重新编译。

### 1.2 系统架构图

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Presentation Layer                           │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐  │
│  │  Client API     │  │  Admin API      │  │  Media API          │  │
│  │  (Axum Router)  │  │  (Axum Router)  │  │  (Axum Router)      │  │
│  └────────┬────────┘  └────────┬────────┘  └──────────┬──────────┘  │
│           │                    │                      │              │
│           └────────────────────┼──────────────────────┘              │
│                                │                                     │
│                    ┌──────────┴──────────┐                          │
│                    │   Middleware Layer  │                          │
│                    │  ┌────────────────┐ │                          │
│                    │  │ Authentication │ │                          │
│                    │  │ Logging        │ │                          │
│                    │  │ CORS           │ │                          │
│                    │  │ Rate Limiting  │ │                          │
│                    │  └────────────────┘ │                          │
│                    └─────────────────────┘                          │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         Service Layer                                │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌────────────┐ │
│  │ Registration │ │    Room      │ │    Sync      │ │   Media    │ │
│  │   Service    │ │   Service    │ │   Service    │ │  Service   │ │
│  └──────────────┘ └──────────────┘ └──────────────┘ └────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         Cache Layer                                  │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    Cache Manager                             │   │
│  │  ┌─────────────────┐           ┌─────────────────────────┐  │   │
│  │  │  Local Cache    │           │    Redis Cache          │  │   │
│  │  │  (Moka)         │           │    (Redis + R2D2)       │  │   │
│  │  │  LRU, In-Memory │           │    Distributed          │  │   │
│  │  └─────────────────┘           └─────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       Storage Layer                                  │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌────────────┐ │
│  │  User        │ │  Device      │ │   Room       │ │   Event    │ │
│  │  Storage     │ │  Storage     │ │   Storage    │ │   Storage  │ │
│  └──────────────┘ └──────────────┘ └──────────────┘ └────────────┘ │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐               │
│  │  Token       │ │ Membership   │ │ Presence     │               │
│  │  Storage     │ │  Storage     │ │  Storage     │               │
│  └──────────────┘ └──────────────┘ └──────────────┘               │
│                              │                                       │
│                    ┌─────────┴─────────┐                            │
│                    │   SQLx (PostgreSQL)│                           │
│                    └────────────────────┘                           │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.3 技术选型理由

本项目的技术选型基于以下考量，每个选择都经过仔细评估以确保最佳的开发体验和运行时性能。

**Web 框架选择 Axum**：Axum 是 Rust 生态中最成熟、功能最完善的 Web 框架之一。它基于 Tokio 和 Hyper 构建，提供类型安全的路由定义和中间件组合机制。Axum 的 Extractor 机制使得从请求中提取数据变得简单直观，同时保持编译时类型检查。社区活跃，文档完善，有大量开源项目参考。

**数据库访问选择 SQLx**：SQLx 是一个异步 SQL 工具库，支持 PostgreSQL、MySQL、SQLite 等多种数据库。它最大的特点是支持编译时 SQL 检查，在编译阶段就能发现 SQL 语法错误和列名不匹配等问题。SQLx 的异步 API 设计合理，与 Tokio 运行时完美配合。

**连接池选择 deadpool**：deadpool 是一个高性能的连接池实现，支持同步和异步两种使用模式。它采用无锁设计，在高并发场景下表现优异。deadpool 提供了预热功能，可以在启动时预先建立连接，避免运行时建立连接的开销。

**缓存选择 Moka 和 Redis**：采用两级缓存架构，本地缓存使用 Moka，这是一个高性能的并发 LRU 缓存库，支持异步 API 和多种过期策略。分布式缓存使用 Redis，提供数据持久化和多节点共享能力。Redis 的 Pub/Sub 功能还可以用于实现缓存失效的广播机制。

---

## 二、模块详细设计

### 2.1 common 模块设计

common 模块是整个项目的基础设施层，提供所有其他模块共享的功能和数据类型。该模块的设计目标是高内聚、低耦合，只包含真正具有普遍性的内容。

#### 2.1.1 error 子模块设计

error 子模块定义了项目统一的错误处理体系。核心类型是 ApiError 结构体，它实现了 std::error::Error trait，可以被任何错误处理框架兼容处理。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,      // 错误码，如 "BAD_REQUEST"
    pub message: String,   // 人类可读的错误消息
    pub status: u16,       // HTTP 状态码
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self
    pub fn unauthorized(message: impl Into<String>) -> Self
    pub fn forbidden(message: impl Into<String>) -> Self
    pub fn not_found(message: impl Into<String>) -> Self
    pub fn conflict(message: impl Into<String>) -> Self
    pub fn internal(message: impl Into<String>) -> Self
}

pub type ApiResult<T> = Result<T, ApiError>;
```

这种设计提供了几个好处：首先，调用者可以通过链式调用快速创建特定类型的错误；其次，错误包含足够的上下文信息用于日志记录和调试；最后，统一的类型使得错误处理代码更加简洁。

#### 2.1.2 config 子模块设计

config 子模块负责从配置文件和环境变量中读取配置参数。设计使用了 config crate 的层级配置机制，支持从多个来源合并配置。

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub name: String,      // 服务器名称
    pub host: String,      // 监听地址
    pub port: u16,         // 监听端口
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,       // 数据库连接 URL
    pub pool_size: u32,    // 连接池大小
    pub max_size: u32,     // 最大连接数
}

#[derive(Debug, Clone, Deserialize)]
pub struct CacheConfig {
    pub redis_url: String, // Redis 连接地址
    pub local_max_capacity: u64,  // 本地缓存容量
    pub redis_ttl: u64,    // Redis 缓存 TTL（秒）
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub secret: String,    // JWT 签名密钥
    pub expiry: u64,       // 访问令牌有效期（秒）
    pub refresh_expiry: u64, // 刷新令牌有效期（秒）
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub cache: CacheConfig,
    pub jwt: JwtConfig,
}
```

配置加载时，首先加载默认配置，然后从配置文件覆盖，最后从环境变量覆盖。这种层级设计使得在不同环境部署时可以通过简单设置环境变量来覆盖配置，无需修改配置文件。

#### 2.1.3 crypto 子模块设计

crypto 子模块提供密码学相关的工具函数。主要功能包括用户密码的安全存储和验证，以及通用数据的加密解密。

密码存储使用 argon2 算法，这是目前推荐的密码哈希算法，具有抗暴力破解和内存-hard 特性。verify_password 函数使用恒定时间比较算法，防止时序攻击。

```rust
pub fn hash_password(password: &str) -> Result<String, CryptoError>
pub fn verify_password(password: &str, hash: &str) -> Result<bool, CryptoError>
pub fn generate_token(length: usize) -> String
pub fn hash_data(data: &str, key: &str) -> Result<String, CryptoError>
pub fn verify_signature(data: &str, signature: &str, key: &str) -> Result<bool, CryptoError>
```

### 2.2 storage 模块设计

storage 模块负责所有数据持久化操作，采用数据访问对象（DAO）模式封装数据库操作。每个实体对应一个 Storage 结构体，提供该实体的 CRUD 操作。

#### 2.2.1 UserStorage 设计

UserStorage 封装用户实体的所有数据库操作。核心方法包括 create_user 创建新用户，get_user_by_id 通过用户 ID 获取用户信息，get_user_by_username 通过用户名获取用户信息，update_user 更新用户信息，delete_user 删除用户。

```rust
pub struct UserStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> UserStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self
    pub async fn create_user(&self, username: &str, password_hash: &str, is_admin: bool) -> Result<User, sqlx::Error>
    pub async fn get_user_by_id(&self, user_id: &str) -> Result<Option<User>, sqlx::Error>
    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, sqlx::Error>
    pub async fn update_user(&self, user_id: &str, updates: UserUpdates) -> Result<(), sqlx::Error>
    pub async fn delete_user(&self, user_id: &str) -> Result<(), sqlx::Error>
    pub async fn list_users(&self, limit: u64, offset: u64) -> Result<Vec<User>, sqlx::Error>
    pub async fn count_users(&self) -> Result<i64, sqlx::Error>
}
```

User 结构体定义如下：

```rust
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub user_id: String,
    pub username: String,
    pub password_hash: Option<String>,
    pub displayname: Option<String>,
    pub avatar_url: Option<String>,
    pub admin: bool,
    pub deactivated: bool,
    pub is_guest: bool,
    pub consent_version: Option<String>,
    pub appservice_id: Option<String>,
    pub user_type: Option<String>,
    pub shadow_banned: bool,
    pub generation: i64,
    pub invalid_update_ts: Option<i64>,
    pub migration_state: Option<String>,
    pub creation_ts: DateTime<Utc>,
}
```

#### 2.2.2 DeviceStorage 设计

DeviceStorage 封装设备实体的所有数据库操作。设备与用户之间存在一对多关系，删除用户时需要级联删除其所有设备。

```rust
pub struct DeviceStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> DeviceStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self
    pub async fn create_device(&self, device_id: &str, user_id: &str, display_name: Option<&str>) -> Result<Device, sqlx::Error>
    pub async fn get_device(&self, device_id: &str) -> Result<Option<Device>, sqlx::Error>
    pub async fn get_user_devices(&self, user_id: &str) -> Result<Vec<Device>, sqlx::Error>
    pub async fn update_device_display_name(&self, device_id: &str, display_name: &str) -> Result<(), sqlx::Error>
    pub async fn update_device_last_seen(&self, device_id: &str) -> Result<(), sqlx::Error>
    pub async fn delete_device(&self, device_id: &str) -> Result<(), sqlx::Error>
    pub async fn delete_user_devices(&self, user_id: &str) -> Result<(), sqlx::Error>
}
```

#### 2.2.3 EventStorage 设计

EventStorage 封装事件实体的所有数据库操作。事件是 Matrix 协议的核心概念，所有房间操作都表示为事件。EventStorage 需要支持高效的房间事件查询和分页。

```rust
pub struct EventStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> EventStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self
    pub async fn create_event(&self, event: &Event) -> Result<Event, sqlx::Error>
    pub async fn get_event(&self, event_id: &str) -> Result<Option<Event>, sqlx::Error>
    pub async fn get_room_events(&self, room_id: &str, from: &str, to: &str, limit: u64) -> Result<Vec<Event>, sqlx::Error>
    pub async fn get_state_events(&self, room_id: &str, event_type: Option<&str>, state_key: Option<&str>) -> Result<Vec<Event>, sqlx::Error>
    pub async fn get_missing_events(&self, room_id: &str, earliest_events: &[&str], latest_events: &[&str]) -> Result<Vec<Event>, sqlx::Error>
}
```

### 2.3 cache 模块设计

cache 模块实现两级缓存架构，平衡访问延迟和内存占用。本地缓存提供最快的访问速度，分布式缓存支持多实例共享。

#### 2.3.1 CacheManager 设计

CacheManager 是缓存层的统一入口，提供简洁的缓存操作接口。它封装了本地缓存和 Redis 缓存的实现细节，对外提供统一的 API。

```rust
pub struct CacheManager {
    local: Arc<LocalCache>,
    redis: Option<Arc<RedisCache>>,
    config: CacheConfig,
}

impl CacheManager {
    pub fn new(config: CacheConfig) -> Self
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, CacheError>
    pub async fn set<T: Serialize>(&self, key: &str, value: &T, ttl: Option<Duration>) -> Result<(), CacheError>
    pub async fn delete(&self, key: &str) -> Result<(), CacheError>
    pub async fn exists(&self, key: &str) -> Result<bool, CacheError>
    pub async fn invalidate_pattern(&self, pattern: &str) -> Result<(), CacheError>
}
```

CacheManager 的 get 操作首先查询本地缓存，命中则直接返回；未命中则查询 Redis 缓存，命中则写入本地缓存后返回；仍未命中返回 None。set 操作同时写入本地缓存和 Redis 缓存，确保多实例一致性。

#### 2.3.2 缓存键设计规范

为避免缓存键冲突，采用统一的键命名规范：{prefix}:{entity}:{id}。前缀用于区分不同的缓存类型，例如 "user" 表示用户缓存，"room" 表示房间缓存。

示例键名：user:@alice:localhost 表示用户 alice 的缓存，room:!room123:localhost 表示房间 !room123 的缓存。

### 2.4 auth 模块设计

auth 模块负责身份认证和访问控制，是系统安全的核心模块。该模块基于 JWT 实现无状态认证，结合缓存实现高性能的令牌验证。

#### 2.4.1 AuthService 设计

AuthService 提供用户注册、登录、登出等核心认证功能。

```rust
pub struct AuthService {
    user_storage: UserStorage<'static>,
    device_storage: DeviceStorage<'static>,
    token_storage: TokenStorage<'static>,
    cache: Arc<CacheManager>,
    jwt_secret: Vec<u8>,
    token_expiry: i64,
    refresh_token_expiry: i64,
    server_name: String,
}

impl AuthService {
    pub fn new(
        pool: &'static PgPool,
        cache: Arc<CacheManager>,
        jwt_secret: &str,
        server_name: &str,
    ) -> Self
    
    pub async fn register(
        &self,
        username: &str,
        password: &str,
        admin: bool,
        displayname: Option<&str>,
    ) -> ApiResult<(User, String, String, String)>
    
    pub async fn login(
        &self,
        username: &str,
        password: &str,
        device_id: Option<&str>,
        initial_display_name: Option<&str>,
    ) -> ApiResult<(User, String, String, String)>
    
    pub async fn logout(&self, user_id: &str, device_id: &str, access_token: &str) -> ApiResult<()>
    
    pub async fn validate_token(&self, token: &str) -> ApiResult<TokenClaims>
    
    pub async fn refresh_access_token(&self, refresh_token: &str) -> ApiResult<(String, String)>
}
```

register 方法的流程：首先验证用户名格式和唯一性，然后使用 crypto 模块的 hash_password 函数对密码进行哈希处理，接着创建用户记录、设备记录和访问令牌，最后返回用户信息和令牌。

login 方法的流程：首先根据用户名查询用户，然后使用 crypto 模块的 verify_password 函数验证密码，接着更新设备活跃时间，最后生成新的访问令牌和刷新令牌。

### 2.5 web 模块设计

web 模块负责 HTTP 请求的处理，是系统的入口层。采用 Axum 框架构建，路由清晰，中间件可组合。

#### 2.5.1 路由组织

每个 API 模块对应一个路由文件，定义该模块的所有端点。路由使用链式 API 定义，清晰直观。

```rust
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(|| async { json!({"msg": "Synapse Rust Matrix Server"}) }))
        .route("/_matrix/client/versions", get(get_client_versions))
        .route("/_matrix/client/r0/register", post(register))
        .route("/_matrix/client/r0/login", post(login))
        // ... 更多端点
        .with_state(state)
}
```

#### 2.5.2 Handler 函数设计

Handler 函数是请求处理的最终执行单元。每个 handler 接收请求，调用服务层执行业务逻辑，返回响应。

```rust
async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, ApiError> {
    // 参数验证
    if body.username.is_empty() {
        return Err(ApiError::bad_request("Username cannot be empty".to_string()));
    }
    
    // 调用服务层
    let (user, access_token, refresh_token, device_id) = 
        state.services.auth_service.register(
            &body.username,
            &body.password,
            body.admin.unwrap_or(false),
            body.displayname.as_deref(),
        ).await?;
    
    // 返回响应
    Ok(Json(RegisterResponse {
        user_id: user.user_id,
        access_token,
        refresh_token,
        device_id,
    }))
}
```

---

## 三、数据流设计

### 3.1 注册流程数据流

用户注册的完整数据流如下：客户端发送注册请求到服务器，请求包含用户名和密码；中间件首先处理请求，记录日志并执行认证检查；路由层将请求分发到 register handler；handler 调用 auth 模块的 register 方法；auth 模块首先调用 user_storage 检查用户名唯一性；通过后调用 crypto 模块对密码进行哈希；然后调用 user_storage 创建用户记录；调用 device_storage 创建设备记录；调用 token_storage 创建访问令牌和刷新令牌；最后将结果返回给客户端。

整个流程中，每一步都可能返回错误，错误会沿着调用栈向上传播，最终由顶层的错误处理中间件转换为标准化的错误响应。

### 3.2 消息发送流程数据流

用户在房间中发送消息的数据流如下：客户端发送消息请求，请求包含 room_id、消息类型和消息内容；认证中间件验证请求头中的 Bearer Token；验证通过后提取用户信息，附加到请求扩展中；路由层将请求分发到 send_message handler；handler 调用 room_service 的 send_message 方法；room_service 首先验证用户是否为房间成员；然后调用 event_storage 创建消息事件；更新房间的 member_count；最后返回事件 ID 给客户端。

### 3.3 同步流程数据流

客户端同步的数据流如下：客户端发送同步请求，请求包含 since 参数指定上次同步的位置；认证中间件验证访问令牌；路由层将请求分发到 sync handler；sync handler 调用 sync_service 的 sync 方法；sync_service 根据 since 参数查询新产生的事件；查询时首先检查缓存，缓存未命中则查询数据库；将事件组织成同步响应格式返回给客户端。

---

## 四、接口设计

### 4.1 内部接口规范

模块之间通过 trait 定义接口，实现依赖倒置。定义 Storage trait 规范存储操作，定义 Service trait 规范业务操作。

```rust
pub trait UserStorageTrait {
    fn create_user(&self, username: &str, password_hash: &str, is_admin: bool) -> Pin<Box<dyn Future<Output = Result<User, sqlx::Error>> + Send>>;
    fn get_user_by_id(&self, user_id: &str) -> Pin<Box<dyn Future<Output = Result<Option<User>, sqlx::Error>> + Send>>;
    // ... 其他方法
}
```

这种设计允许在测试时使用 mock 实现替换真实实现，便于单元测试。

### 4.2 外部接口规范

所有对外提供的 HTTP API 必须遵循 Matrix 规范。请求和响应均使用 JSON 格式，字段命名使用 snake_case。错误响应包含 errcode 和 error 两个字段，errcode 采用 Matrix 规范定义的错误码。

---

## 五、安全设计

### 5.1 认证安全

用户密码使用 argon2 算法哈希存储，算法参数设置为安全等级 3。JWT 使用 HS256 算法签名，密钥长度不少于 256 位。访问令牌有效期为 24 小时，刷新令牌有效期为 7 天。令牌验证结果缓存 5 分钟，平衡安全性和性能。

### 5.2 传输安全

所有 API 强制使用 HTTPS 连接，禁止 HTTP 传输。敏感数据（如密码、令牌）在客户端使用 TLS 1.3 加密传输。服务器配置支持 HSTS 响应头，强制浏览器使用 HTTPS。

### 5.3 数据安全

数据库连接使用 SSL，连接凭证从环境变量读取。敏感数据（如密码哈希）不记录日志。用户密码永不以明文形式存储或传输。实现防重放攻击机制，请求包含时间戳和随机数。

---

## 六、增强功能模块设计

### 6.1 模块概述

增强功能模块是本项目的扩展功能，包括好友系统、私聊管理、语音消息和内部安全控制。这些模块遵循项目统一架构，共享公共基础设施，提供更丰富的用户体验。

**公开发布策略：**

| 模块 | 发布策略 | API 前缀 | 说明 |
|------|----------|----------|------|
| 好友系统 | 对外发布 | `/_synapse/enhanced/friend` | 核心社交功能 |
| 私聊管理 | 对外发布 | `/_synapse/enhanced/private` | 端到端加密通信 |
| 语音消息 | 对外发布 | `/_synapse/enhanced/voice` | 语音消息处理 |
| 安全控制 | 内部管理 | `/_synapse/admin/v1/security` | 仅 Admin API 开放 |

### 6.2 好友系统模块（Friend）

好友系统模块提供完整的用户关系管理能力，包括好友关系维护、请求处理、分组管理和用户屏蔽功能。

#### 6.2.1 FriendStorage 设计

```rust
pub struct FriendStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> FriendStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self
    
    pub async fn create_friend_relation(
        &self,
        user_id: &str,
        friend_id: &str,
        remark: Option<&str>,
        category_id: Option<&str>,
    ) -> Result<FriendRelation, sqlx::Error>
    
    pub async fn remove_friend(&self, user_id: &str, friend_id: &str) -> Result<(), sqlx::Error>
    
    pub async fn get_friends(
        &self,
        user_id: &str,
        category_id: Option<&str>,
        limit: u64,
        offset: u64,
    ) -> Result<Vec<FriendInfo>, sqlx::Error>
    
    pub async fn get_friend_by_id(&self, user_id: &str, friend_id: &str) -> Result<Option<FriendInfo>, sqlx::Error>
    
    pub async fn count_friends(&self, user_id: &str, category_id: Option<&str>) -> Result<i64, sqlx::Error>
    
    pub async fn is_friend(&self, user_id: &str, friend_id: &str) -> Result<bool, sqlx::Error>
}

pub struct FriendRequestStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> FriendRequestStorage<'a> {
    pub async fn create_request(
        &self,
        requester_id: &str,
        target_id: &str,
        message: Option<&str>,
        category_id: Option<&str>,
    ) -> Result<FriendRequest, sqlx::Error>
    
    pub async fn respond_to_request(
        &self,
        request_id: &str,
        action: &str,
    ) -> Result<(), sqlx::Error>
    
    pub async fn get_pending_requests(&self, user_id: &str, limit: u64, offset: u64) -> Result<Vec<FriendRequest>, sqlx::Error>
    
    pub async fn expire_requests(&self) -> Result<u64, sqlx::Error>
}

pub struct FriendCategoryStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> FriendCategoryStorage<'a> {
    pub async fn create_category(&self, user_id: &str, name: &str, sort_order: i32) -> Result<FriendCategory, sqlx::Error>
    
    pub async fn update_category(&self, category_id: &str, name: Option<&str>, sort_order: Option<i32>) -> Result<(), sqlx::Error>
    
    pub async fn delete_category(&self, category_id: &str, move_friends: bool) -> Result<(), sqlx::Error>
    
    pub async fn get_categories(&self, user_id: &str) -> Result<Vec<FriendCategory>, sqlx::Error>
}

pub struct BlockedUserStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> BlockedUserStorage<'a> {
    pub async fn block_user(&self, user_id: &str, blocked_id: &str, reason: Option<&str>) -> Result<(), sqlx::Error>
    
    pub async fn unblock_user(&self, user_id: &str, blocked_id: &str) -> Result<(), sqlx::Error>
    
    pub async fn get_blocked_users(&self, user_id: &str) -> Result<Vec<BlockedUser>, sqlx::Error>
    
    pub async fn is_blocked(&self, user_id: &str, target_id: &str) -> Result<bool, sqlx::Error>
}
```

#### 6.2.2 FriendService 设计

```rust
pub struct FriendService {
    friend_storage: FriendStorage<'static>,
    request_storage: FriendRequestStorage<'static>,
    category_storage: FriendCategoryStorage<'static>,
    block_storage: BlockedUserStorage<'static>,
    cache: Arc<CacheManager>,
}

impl FriendService {
    pub async fn add_friend(&self, user_id: &str, target_id: &str, message: Option<&str>) -> ApiResult<FriendRequest>
    
    pub async fn accept_friend_request(&self, user_id: &str, request_id: &str) -> ApiResult<()>
    
    pub async fn reject_friend_request(&self, user_id: &str, request_id: &str) -> ApiResult<()>
    
    pub async fn remove_friend(&self, user_id: &str, friend_id: &str) -> ApiResult<()>
    
    pub async fn get_friends(&self, user_id: &str, category_id: Option<&str>, limit: u64, cursor: Option<&str>) -> ApiResult<FriendListResponse>
    
    pub async fn create_category(&self, user_id: &str, name: &str) -> ApiResult<FriendCategory>
    
    pub async fn block_user(&self, user_id: &str, blocked_id: &str, reason: Option<&str>) -> ApiResult<()>
    
    pub async fn unblock_user(&self, user_id: &str, blocked_id: &str) -> ApiResult<()>
    
    pub async fn get_recommendations(&self, user_id: &str, limit: u64) -> ApiResult<Vec<UserRecommendation>>
}
```

### 6.3 私聊管理模块（PrivateChat）

私聊管理模块提供端到端加密的私密通信能力，包括会话管理、消息传递和密钥分发功能。

#### 6.3.1 PrivateSessionStorage 设计

```rust
pub struct PrivateSessionStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> PrivateSessionStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self
    
    pub async fn create_session(
        &self,
        creator_id: &str,
        participants: &[&str],
        session_name: Option<&str>,
        ttl_seconds: Option<i32>,
    ) -> Result<PrivateSession, sqlx::Error>
    
    pub async fn get_session(&self, session_id: &str) -> Result<Option<PrivateSession>, sqlx::Error>
    
    pub async fn get_user_sessions(
        &self,
        user_id: &str,
        limit: u64,
        since: Option<i64>,
    ) -> Result<Vec<PrivateSession>, sqlx::Error>
    
    pub async fn add_participant(&self, session_id: &str, user_id: &str) -> Result<(), sqlx::Error>
    
    pub async fn remove_participant(&self, session_id: &str, user_id: &str) -> Result<(), sqlx::Error>
    
    pub async fn delete_session(&self, session_id: &str, user_id: &str) -> Result<(), sqlx::Error>
}

pub struct PrivateMessageStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> PrivateMessageStorage<'a> {
    pub async fn create_message(
        &self,
        session_id: &str,
        sender_id: &str,
        encrypted_content: &str,
        message_type: &str,
        ttl_seconds: Option<i32>,
    ) -> Result<PrivateMessage, sqlx::Error>
    
    pub async fn get_messages(
        &self,
        session_id: &str,
        user_id: &str,
        limit: u64,
        before: Option<i64>,
        after: Option<i64>,
    ) -> Result<Vec<PrivateMessage>, sqlx::Error>
    
    pub async fn mark_as_read(&self, message_id: &str, user_id: &str) -> Result<(), sqlx::Error>
    
    pub async fn get_unread_count(&self, user_id: &str) -> Result<i64, sqlx::Error>
    
    pub async fn delete_expired_messages(&self) -> Result<u64, sqlx::Error>
}

pub struct SessionKeyStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> SessionKeyStorage<'a> {
    pub async fn distribute_key(
        &self,
        session_id: &str,
        user_id: &str,
        device_id: &str,
        encrypted_key: &str,
    ) -> Result<(), sqlx::Error>
    
    pub async fn get_user_session_keys(&self, session_id: &str, user_id: &str) -> Result<Vec<SessionKey>, sqlx::Error>
}
```

#### 6.3.2 PrivateChatService 设计

```rust
pub struct PrivateChatService {
    session_storage: PrivateSessionStorage<'static>,
    message_storage: PrivateMessageStorage<'static>,
    key_storage: SessionKeyStorage<'static>,
    cache: Arc<CacheManager>,
    crypto: Arc<CryptoService>,
}

impl PrivateChatService {
    pub async fn create_session(
        &self,
        creator_id: &str,
        participants: &[&str],
        session_name: Option<&str>,
        ttl_seconds: Option<i32>,
    ) -> ApiResult<PrivateSession>
    
    pub async fn send_message(
        &self,
        session_id: &str,
        sender_id: &str,
        content: &str,
        message_type: &str,
    ) -> ApiResult<PrivateMessage>
    
    pub async fn get_messages(
        &self,
        session_id: &str,
        user_id: &str,
        limit: u64,
        before: Option<i64>,
        after: Option<i64>,
    ) -> ApiResult<Vec<PrivateMessage>>
    
    pub async fn get_sessions(&self, user_id: &str, limit: u64, since: Option<i64>) -> ApiResult<Vec<PrivateSession>>
    
    pub async fn mark_message_read(&self, message_id: &str, user_id: &str) -> ApiResult<()>
    
    pub async fn get_unread_count(&self, user_id: &str) -> ApiResult<i64>
    
    pub async fn leave_session(&self, session_id: &str, user_id: &str) -> ApiResult<()>
}
```

### 6.4 语音消息模块（Voice）

语音消息模块提供语音消息的录制、上传、处理和播放功能。

#### 6.4.1 VoiceStorage 设计

```rust
pub struct VoiceStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> VoiceStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self
    
    pub async fn create_voice_message(
        &self,
        user_id: &str,
        file_format: &str,
        file_size: i64,
        duration: i32,
        file_url: &str,
        room_id: Option<&str>,
    ) -> Result<VoiceMessage, sqlx::Error>
    
    pub async fn get_voice_message(&self, message_id: &str) -> Result<Option<VoiceMessage>, sqlx::Error>
    
    pub async fn get_user_voice_messages(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> Result<Vec<VoiceMessage>, sqlx::Error>
    
    pub async fn delete_voice_message(&self, message_id: &str, user_id: &str) -> Result<(), sqlx::Error>
    
    pub async fn get_user_stats(&self, user_id: &str) -> Result<VoiceMessageStats, sqlx::Error>
}
```

#### 6.4.2 VoiceService 设计

```rust
pub struct VoiceService {
    storage: VoiceStorage<'static>,
    cache: Arc<CacheManager>,
    media_path: String,
    max_file_size: u64,
    max_duration: u64,
}

impl VoiceService {
    pub async fn upload_voice(
        &self,
        user_id: &str,
        audio_data: &[u8],
        file_format: &str,
    ) -> ApiResult<VoiceUploadResponse>
    
    pub async fn get_voice_message(&self, message_id: &str, user_id: &str) -> ApiResult<VoiceMessage>
    
    pub async fn delete_voice_message(&self, message_id: &str, user_id: &str) -> ApiResult<()>
    
    pub async fn get_user_voices(&self, user_id: &str, limit: u64, offset: u64) -> ApiResult<Vec<VoiceMessage>>
    
    pub async fn get_user_voice_stats(&self, user_id: &str) -> ApiResult<VoiceMessageStats>
    
    fn validate_audio(&self, data: &[u8], format: &str) -> Result<AudioMetadata, ApiError>
    
    fn process_audio(&self, data: &[u8], format: &str) -> Result<Vec<u8>, ApiError>
}
```

### 6.5 安全控制模块（Security）（仅内部管理）

安全控制模块提供全面的安全防护能力，但仅通过 Admin API 对内开放，不对外发布。

#### 6.5.1 SecurityEventStorage 设计

```rust
pub struct SecurityEventStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> SecurityEventStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self
    
    pub async fn create_event(
        &self,
        event_type: &str,
        user_id: Option<&str>,
        ip_address: &str,
        severity: &str,
        description: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<SecurityEvent, sqlx::Error>
    
    pub async fn get_events(
        &self,
        user_id: Option<&str>,
        ip_address: Option<&str>,
        severity: Option<&str>,
        start_time: Option<i64>,
        end_time: Option<i64>,
        limit: u64,
    ) -> Result<Vec<SecurityEvent>, sqlx::Error>
    
    pub async fn resolve_event(&self, event_id: &str) -> Result<(), sqlx::Error>
}

pub struct BlockedIPStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> BlockedIPStorage<'a> {
    pub async fn block_ip(
        &self,
        ip_address: &str,
        reason: &str,
        duration_seconds: Option<i32>,
    ) -> Result<(), sqlx::Error>
    
    pub async fn unblock_ip(&self, ip_address: &str) -> Result<(), sqlx::Error>
    
    pub async fn get_blocked_ips(&self) -> Result<Vec<BlockedIP>, sqlx::Error>
    
    pub async fn is_blocked(&self, ip_address: &str) -> Result<bool, sqlx::Error>
    
    pub async fn cleanup_expired_blocks(&self) -> Result<u64, sqlx::Error>
}

pub struct IPReputationStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> IPReputationStorage<'a> {
    pub async fn update_reputation(&self, ip_address: &str, score: i32) -> Result<(), sqlx::Error>
    
    pub async fn get_reputation(&self, ip_address: &str) -> Result<IPReputation, sqlx::Error>
    
    pub async fn record_request(&self, ip_address: &str, success: bool) -> Result<(), sqlx::Error>
    
    pub async fn calculate_reputation(&self, ip_address: &str) -> Result<i32, sqlx::Error>
}
```

#### 6.5.2 SecurityService 设计（仅 Admin）

```rust
pub struct SecurityService {
    event_storage: SecurityEventStorage<'static>,
    ip_storage: BlockedIPStorage<'static>,
    reputation_storage: IPReputationStorage<'static>,
    cache: Arc<CacheManager>,
}

impl SecurityService {
    pub async fn analyze_threat(
        &self,
        user_id: Option<&str>,
        ip_address: &str,
        endpoint: &str,
        method: &str,
        content: Option<&str>,
    ) -> Vec<SecurityThreat>
    
    pub async fn block_ip(
        &self,
        admin_id: &str,
        ip_address: &str,
        reason: &str,
        duration_seconds: Option<i32>,
    ) -> ApiResult<()>
    
    pub async fn unblock_ip(&self, admin_id: &str, ip_address: &str) -> ApiResult<()>
    
    pub async fn get_security_events(
        &self,
        admin_id: &str,
        filters: SecurityEventFilters,
        limit: u64,
    ) -> ApiResult<Vec<SecurityEvent>>
    
    pub async fn get_blocked_ips(&self, admin_id: &str) -> ApiResult<Vec<BlockedIP>>
    
    pub async fn get_ip_reputation(&self, admin_id: &str, ip_address: &str) -> ApiResult<IPReputation>
    
    pub async fn get_system_status(&self, admin_id: &str) -> ApiResult<SystemStatus>
}
```

### 6.6 增强功能 API 路由

增强功能模块的 API 路由组织如下：

```rust
pub fn create_enhanced_router(state: AppState) -> Router {
    Router::new()
        // 好友系统
        .route("/friends", get(get_friends))
        .route("/friend/request", post(send_friend_request))
        .route("/friend/request/:request_id/respond", post(respond_friend_request))
        .route("/friend/requests", get(get_friend_requests))
        .route("/friend/categories", get(get_categories).post(create_category))
        .route("/friend/categories/:category_id", put(update_category).delete(delete_category))
        .route("/friend/blocks", get(get_blocked_users))
        .route("/friend/blocks/:user_id", post(block_user).delete(unblock_user))
        .route("/friend/recommendations", get(get_recommendations))
        .route("/friend/batch", post(batch_operation))
        
        // 私聊管理
        .route("/private/sessions", get(get_sessions).post(create_session))
        .route("/private/sessions/:session_id", delete(delete_session))
        .route("/private/sessions/:session_id/messages", get(get_messages).post(send_message))
        .route("/private/messages/:message_id/read", post(mark_read))
        .route("/private/unread-count", get(get_unread_count))
        .route("/private/search", post(search_messages))
        
        // 语音消息
        .route("/voice/upload", post(upload_voice))
        .route("/voice/messages/:message_id", get(get_voice).delete(delete_voice))
        .route("/voice/user/:user_id", get(get_user_voices))
        .route("/voice/user/:user_id/stats", get(get_voice_stats))
        
        .with_state(state)
}

pub fn create_admin_security_router(state: AppState) -> Router {
    Router::new()
        .route("/security/events", get(get_security_events))
        .route("/security/ip/blocks", get(get_blocked_ips))
        .route("/security/ip/block", post(block_ip))
        .route("/security/ip/unblock", post(unblock_ip))
        .route("/security/ip/reputation/:ip", get(get_ip_reputation))
        .route("/status", get(get_system_status))
        
        .with_state(state)
}
```

---

## 七、性能设计

### 6.1 连接池配置

数据库连接池大小根据服务器配置动态调整，默认为 CPU 核心数乘以 4。连接池支持预热功能，启动时预先建立 min_size 个连接。连接空闲超时时间为 5 分钟，超时后自动关闭。

### 6.2 缓存策略

缓存采用 Write-Through 策略，同时更新缓存和数据库。缓存键采用合理的过期策略：用户配置缓存 5 分钟，房间配置缓存 10 分钟，事件列表缓存 1 分钟。使用缓存标签实现批量失效，避免缓存不一致。

### 6.3 异步处理

所有 I/O 操作使用异步方式，不阻塞 Tokio 线程。使用合适的并发度控制，避免过多并发请求导致数据库过载。对于重量级操作（如媒体处理），使用后台任务队列异步执行。

---

## 七、附录

### 7.1 数据库 ER 图

```
┌─────────────┐       ┌─────────────┐
│    users    │──────▶│   devices   │
│─────────────│       │─────────────│
│ user_id (PK)│       │device_id(PK)│
│ username    │       │ user_id (FK)│
│ password_hash│      │ display_name│
│ admin       │       │last_seen_ts │
└─────────────┘       └─────────────┘
      │                     │
      │                     │
      ▼                     ▼
┌─────────────┐       ┌─────────────┐
│access_tokens│       │room_membership│
│─────────────│       │─────────────│
│ token (PK)  │       │room_id (FK) │
│ user_id (FK)│       │user_id (FK) │
│ device_id(FK)│      │ membership  │
└─────────────┘       └─────────────┘
      │
      │
      ▼
┌─────────────┐       ┌─────────────┐
│    rooms    │◀──────│   events    │
│─────────────│       │─────────────│
│ room_id(PK) │       │ event_id(PK)│
│ creator(FK) │       │room_id (FK) │
│ name        │       │user_id (FK) │
│ topic       │       │ event_type  │
└─────────────┘       │ content     │
                      └─────────────┘
```

### 7.2 配置项说明

| 配置项 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| server.name | String | "localhost" | 服务器名称 |
| server.host | String | "0.0.0.0" | 监听地址 |
| server.port | u16 | 8008 | 监听端口 |
| database.url | String | - | 数据库连接 URL |
| database.pool_size | u32 | 10 | 连接池大小 |
| cache.redis_url | String | "redis://localhost:6379" | Redis 连接地址 |
| jwt.secret | String | - | JWT 签名密钥 |
| jwt.expiry | u64 | 86400 | 令牌有效期（秒） |

---

**编制人**：  
**审核人**：  
**批准人**：  
