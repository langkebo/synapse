# Synapse Rust 重构项目规则

> **版本**：2.1.0  
> **最后更新**：2026-01-28  
> **项目状态**：源代码重建中  
> **参考文档**：[Synapse 官方文档](https://element-hq.github.io/synapse/latest/)

---

## 一、项目状态说明

### 1.1 当前状态

由于执行 `git clean -fd` 命令导致源代码目录被意外删除，项目目前处于重建阶段。数据库配置和 schema 已恢复，但核心代码需要重新实现。

| 组件 | 状态 | 说明 |
|------|------|------|
| 数据库 schema | ✅ 已恢复 | users、devices、rooms、events 等表已创建 |
| 数据库用户 | ✅ 已配置 | synapse_user 用户已创建并授权 |
| 项目配置 | ✅ 已存在 | Cargo.toml、基础目录结构存在 |
| 源代码 | 🔄 重建中 | 需要重新实现所有模块 |
| 文档 | ⚠️ 需要更新 | ruls.md 需与当前状态同步 |

### 1.2 重建优先级

| 优先级 | 模块 | 预计工时 | 依赖 |
|--------|------|----------|------|
| P0 | 基础模块（common） | 2小时 | 无 |
| P0 | 存储层（storage） | 4小时 | common |
| P0 | 认证模块（auth） | 3小时 | storage |
| P1 | 服务层（services） | 4小时 | auth、storage |
| P1 | Web 路由层（web/routes） | 4小时 | services |
| P1 | 中间件（web/middleware） | 2小时 | web/routes |
| P1 | 服务器入口（server.rs、main.rs） | 2小时 | web |
| P2 | 测试模块 | 3小时 | 所有模块 |
| P2 | 文档完善 | 2小时 | 所有模块 |

---

## 二、核心目标

### 2.1 性能目标

| 指标 | 当前值 | 目标值 | 提升幅度 |
|------|--------|--------|----------|
| 同步延迟 | 待测量 | 5ms | 基准建立 |
| 内存占用 | 待测量 | 200MB | 基准建立 |
| 并发用户 | 待测量 | 500K | 基准建立 |
| API 响应时间 | 待测量 | <10ms | 基准建立 |

### 2.2 功能目标

- **API 兼容性**：保持与 Matrix 规范完全兼容
- **E2EE 支持**：实现完整的端到端加密功能
- **联邦通信**：完整的 Federation API 支持
- **管理功能**：完善的 Admin API 支持
- **媒体处理**：媒体上传、存储、检索功能
- **增强功能**：好友系统、私聊管理、语音消息（内部管理）

---

## 三、增强功能模块评估

### 3.1 模块公开发布策略

| 模块 | 发布策略 | 说明 |
|------|----------|------|
| 好友系统 | ✅ 对外发布 | 核心社交功能，用户需求强烈 |
| 私聊管理 | ✅ 对外发布 | 端到端加密通信，核心功能 |
| 语音消息 | ✅ 对外发布 | 用户体验增强功能 |
| 安全控制 | ❌ 内部管理 | 仅 Admin API 对内开放 |

### 3.2 安全控制模块评估

**决策：不建议公开发布该模块**

**评估理由：**

1. **功能复杂度高**：包含威胁检测、IP声誉系统、GeoIP定位、异常行为分析等10+功能
2. **实现难度大**：需要集成外部威胁情报库、地理位置服务、行为分析模型
3. **维护成本高**：安全规则需持续更新，检测算法需定期调优
4. **与Matrix协议重叠**：认证、授权、加密等安全机制已有完善实现
5. **安全风险**：公开的安全功能可能被恶意用户研究绕过方法

**建议处理方式：**

- 仅作为内部管理功能，通过 Admin API 使用
- 不提供公开 API 接口
- 部署时仅限内网访问或添加额外认证

### 3.3 好友模块增强建议

**当前状态**：好友关系管理、请求处理、分组管理、用户屏蔽功能已较完善

**建议加强功能：**

| 功能 | 优先级 | 说明 |
|------|--------|------|
| 好友推荐 | P2 | 基于共同好友、互动频率推荐 |
| 好友动态 | P2 | 上线/下线/发布内容状态通知 |
| 批量操作 | P2 | 批量添加、删除、分组管理 |
| 权限控制 | P2 | 精细化的好友权限管理 |

---

## 四、技术栈规范

### 3.1 核心技术选型

| 类别 | 技术 | 版本 | 用途 |
|------|------|------|------|
| 编程语言 | Rust | 2021 Edition | 核心开发 |
| 异步运行时 | Tokio | 1.35+ | 异步处理 |
| Web 框架 | Axum | 0.7 | HTTP 服务 |
| Web 中间件 | Tower-HTTP | 0.5 | CORS、追踪等 |
| 数据库 | PostgreSQL | 15+ | 数据持久化 |
| ORM | SQLx | 0.7 | 数据库操作 |
| 连接池 | deadpool | 0.10 | 连接池管理 |
| 缓存 | Redis | 7.0+ | 分布式缓存 |
| 本地缓存 | Moka | 0.12 | LRU 缓存 |
| 序列化 | serde | 1.0 | JSON 序列化 |
| 配置管理 | config | 0.14 | 配置解析 |
| JWT 认证 | jsonwebtoken | 9.0 | Token 生成 |
| 日志追踪 | tracing | 0.1 | 结构化日志 |

### 3.2 项目结构

```
synapse_rust/
├── Cargo.toml                 # 项目配置
├── src/
│   ├── lib.rs                # 库入口
│   ├── main.rs               # 服务入口
│   ├── common/               # 公共模块
│   │   ├── mod.rs
│   │   ├── error.rs          # 错误类型
│   │   ├── types.rs          # 公共类型
│   │   ├── config.rs         # 配置解析
│   │   └── crypto.rs         # 加密工具
│   ├── storage/              # 存储层
│   │   ├── mod.rs
│   │   ├── user.rs           # 用户存储
│   │   ├── device.rs         # 设备存储
│   │   ├── token.rs          # 令牌存储
│   │   ├── room.rs           # 房间存储
│   │   ├── membership.rs     # 成员存储
│   │   ├── event.rs          # 事件存储
│   │   ├── friend.rs         # 好友关系存储
│   │   └── private.rs        # 私聊会话存储
│   ├── cache/                # 缓存层
│   │   ├── mod.rs
│   │   ├── local.rs          # 本地缓存
│   │   └── redis.rs          # Redis 缓存
│   ├── auth/                 # 认证模块
│   │   └── mod.rs            # 认证服务
│   ├── services/             # 业务服务层
│   │   ├── mod.rs
│   │   ├── registration.rs   # 注册服务
│   │   ├── room.rs           # 房间服务
│   │   ├── sync.rs           # 同步服务
│   │   ├── media.rs          # 媒体服务
│   │   ├── friend.rs         # 好友服务
│   │   ├── private_chat.rs   # 私聊服务
│   │   └── voice.rs          # 语音消息服务
│   ├── web/                  # Web 路由层
│   │   ├── mod.rs
│   │   ├── routes/
│   │   │   ├── mod.rs        # 客户端 API
│   │   │   ├── admin.rs      # 管理 API
│   │   │   ├── media.rs      # 媒体 API
│   │   │   ├── federation.rs # 联邦 API
│   │   │   ├── friend.rs     # 好友 API (增强)
│   │   │   ├── private.rs    # 私聊 API (增强)
│   │   │   └── voice.rs      # 语音消息 API (增强)
│   │   └── middleware/       # HTTP 中间件
│   │       ├── mod.rs
│   │       ├── logging.rs
│   │       ├── cors.rs
│   │       └── auth.rs
│   └── server.rs             # 服务器配置
├── schema.sql                # 数据库 schema
├── config.yaml               # 配置文件模板
└── docs/                     # 文档目录
```

---

## 四、API 实现规范

### 4.1 Client API 实现状态

| 端点 | 方法 | 状态 | 优先级 |
|------|------|------|--------|
| `/_matrix/client/versions` | GET | 待实现 | P0 |
| `/_matrix/client/r0/register` | POST | 待实现 | P0 |
| `/_matrix/client/r0/register/available` | GET | 待实现 | P0 |
| `/_matrix/client/r0/login` | POST | 待实现 | P0 |
| `/_matrix/client/r0/logout` | POST | 待实现 | P1 |
| `/_matrix/client/r0/logout/all` | POST | 待实现 | P1 |
| `/_matrix/client/r0/refresh` | POST | 待实现 | P1 |
| `/_matrix/client/r0/account/whoami` | GET | 待实现 | P1 |
| `/_matrix/client/r0/sync` | GET | 待实现 | P1 |
| `/_matrix/client/r0/rooms/:room_id/messages` | GET | 待实现 | P1 |
| `/_matrix/client/r0/rooms/:room_id/send/:event_type` | POST | 待实现 | P1 |
| `/_matrix/client/r0/createRoom` | POST | 待实现 | P1 |

### 4.2 Admin API 实现状态

| 端点 | 方法 | 状态 | 优先级 |
|------|------|------|--------|
| `/_synapse/admin/v1/server_version` | GET | 待实现 | P1 |
| `/_synapse/admin/v1/register` | POST | 待实现 | P1 |
| `/_synapse/admin/v1/users/:user_id` | GET | 待实现 | P1 |
| `/_synapse/admin/v1/users/:user_id` | PUT | 待实现 | P1 |
| `/_synapse/admin/v1/users/:user_id/admin` | POST | 待实现 | P2 |
| `/_synapse/admin/v1/rooms/:room_id` | GET | 待实现 | P1 |
| `/_synapse/admin/v1/rooms/:room_id` | DELETE | 待实现 | P2 |

### 4.3 Federation API 实现状态

| 端点 | 方法 | 状态 | 优先级 |
|------|------|------|--------|
| `/_matrix/federation/v1/version` | GET | 待实现 | P1 |
| `/_matrix/federation/v1/send/:txn_id` | PUT | 待实现 | P1 |
| `/_matrix/federation/v1/keys/claim` | POST | 待实现 | P2 |
| `/_matrix/federation/v1/keys/upload` | POST | 待实现 | P2 |
| `/_matrix/federation/v2/key/clone` | POST | 待实现 | P2 |

### 4.4 Enhanced API 实现状态（增强功能）

#### 4.4.1 好友系统 API

| 端点 | 方法 | 状态 | 优先级 |
|------|------|------|--------|
| `/_synapse/enhanced/friends` | GET | 待实现 | P1 |
| `/_synapse/enhanced/friend/request` | POST | 待实现 | P1 |
| `/_synapse/enhanced/friend/request/:request_id/respond` | POST | 待实现 | P1 |
| `/_synapse/enhanced/friend/requests` | GET | 待实现 | P1 |
| `/_synapse/enhanced/friend/categories` | GET/POST | 待实现 | P1 |
| `/_synapse/enhanced/friend/categories/:category_id` | PUT/DELETE | 待实现 | P2 |
| `/_synapse/enhanced/friend/blocks` | GET | 待实现 | P1 |
| `/_synapse/enhanced/friend/blocks/:user_id` | POST/DELETE | 待实现 | P1 |
| `/_synapse/enhanced/friend/recommendations` | GET | 待实现 | P2 |
| `/_synapse/enhanced/friend/batch` | POST | 待实现 | P2 |

#### 4.4.2 私聊管理 API

| 端点 | 方法 | 状态 | 优先级 |
|------|------|------|--------|
| `/_synapse/enhanced/private/sessions` | GET/POST | 待实现 | P1 |
| `/_synapse/enhanced/private/sessions/:session_id` | DELETE | 待实现 | P1 |
| `/_synapse/enhanced/private/sessions/:session_id/messages` | GET/POST | 待实现 | P1 |
| `/_synapse/enhanced/private/messages/:message_id/read` | POST | 待实现 | P1 |
| `/_synapse/enhanced/private/unread-count` | GET | 待实现 | P1 |
| `/_synapse/enhanced/private/search` | POST | 待实现 | P2 |

#### 4.4.3 语音消息 API

| 端点 | 方法 | 状态 | 优先级 |
|------|------|------|--------|
| `/_synapse/enhanced/voice/upload` | POST | 待实现 | P1 |
| `/_synapse/enhanced/voice/messages/:message_id` | GET | 待实现 | P1 |
| `/_synapse/enhanced/voice/messages/:message_id` | DELETE | 待实现 | P1 |
| `/_synapse/enhanced/voice/user/:user_id` | GET | 待实现 | P1 |
| `/_synapse/enhanced/voice/user/:user_id/stats` | GET | 待实现 | P2 |

#### 4.4.4 安全控制 API（仅 Admin）

| 端点 | 方法 | 状态 | 优先级 |
|------|------|------|--------|
| `/_synapse/admin/v1/security/events` | GET | 待实现 | P1 |
| `/_synapse/admin/v1/security/ip/blocks` | GET | 待实现 | P1 |
| `/_synapse/admin/v1/security/ip/block` | POST | 待实现 | P1 |
| `/_synapse/admin/v1/security/ip/unblock` | POST | 待实现 | P1 |
| `/_synapse/admin/v1/security/ip/reputation/:ip` | GET | 待实现 | P1 |
| `/_synapse/admin/v1/status` | GET | 待实现 | P1 |

**注意**：安全控制模块仅对管理员开放，不对外发布。

---

## 五、数据库设计规范

### 5.1 核心表结构

#### 5.1.1 用户表（users）

```sql
CREATE TABLE users (
    user_id TEXT NOT NULL PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT,
    admin BOOLEAN DEFAULT FALSE,
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
);
```

#### 5.1.2 设备表（devices）

```sql
CREATE TABLE devices (
    device_id TEXT NOT NULL PRIMARY KEY,
    user_id TEXT NOT NULL,
    display_name TEXT,
    last_seen_ts BIGINT NOT NULL,
    last_seen_ip TEXT,
    created_ts BIGINT NOT NULL,
    ignored_user_list TEXT,
    appservice_id TEXT,
    first_seen_ts BIGINT DEFAULT 0,
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);
```

#### 5.1.3 访问令牌表（access_tokens）

```sql
CREATE TABLE access_tokens (
    id BIGSERIAL PRIMARY KEY,
    token TEXT NOT NULL UNIQUE,
    user_id TEXT NOT NULL,
    device_id TEXT,
    expires_ts BIGINT,
    created_ts BIGINT NOT NULL,
    invalidated_ts BIGINT,
    expired_ts BIGINT,
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE,
    FOREIGN KEY (device_id) REFERENCES devices(device_id) ON DELETE CASCADE
);
```

#### 5.1.4 房间表（rooms）

```sql
CREATE TABLE rooms (
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
    deleted_ts BIGINT,
    join_rule TEXT,
    member_count INTEGER DEFAULT 0
);
```

#### 5.1.5 房间成员表（room_memberships）

```sql
CREATE TABLE room_memberships (
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
    join_reason TEXT,
    banned_by TEXT,
    PRIMARY KEY (room_id, user_id),
    FOREIGN KEY (room_id) REFERENCES rooms(room_id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);
```

#### 5.1.6 事件表（events）

```sql
CREATE TABLE events (
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
    FOREIGN KEY (room_id) REFERENCES rooms(room_id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);
```

---

## 六、错误处理规范

### 6.1 错误类型定义

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub status: u16,
}

impl ApiError {
    pub fn bad_request(message: String) -> Self
    pub fn unauthorized(message: String) -> Self
    pub fn forbidden(message: String) -> Self
    pub fn not_found(message: String) -> Self
    pub fn conflict(message: String) -> Self
    pub fn internal(message: String) -> Self
}

pub type ApiResult<T> = Result<T, ApiError>;
```

### 错误码映射

6.2 | HTTP 状态码 | 错误码 | 说明 |
|-------------|--------|------|
| 400 | BAD_REQUEST | 请求参数错误 |
| 401 | UNAUTHORIZED | 未认证或 Token 无效 |
| 403 | FORBIDDEN | 权限不足 |
| 404 | NOT_FOUND | 资源不存在 |
| 409 | CONFLICT | 资源冲突 |
| 429 | RATE_LIMITED | 请求频率超限 |
| 500 | INTERNAL_ERROR | 服务器内部错误 |
| 502 | BAD_GATEWAY | 网关错误 |
| 503 | SERVER_BUSY | 服务繁忙 |

---

## 七、认证规范

### 7.1 JWT Token 结构

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,           // 用户 ID
    pub user_id: String,       // 用户 ID
    pub admin: bool,           // 是否管理员
    pub exp: i64,              // 过期时间
    pub iat: i64,              // 签发时间
    pub device_id: Option<String>, // 设备 ID
}
```

### 7.2 认证流程

1. **注册流程**：用户名 → 密码哈希 → 创建设备 → 生成 Token
2. **登录流程**：验证密码 → 更新设备 → 生成 Token
3. **Token 验证**：解析 JWT → 验证签名 → 检查过期 → 缓存验证

---

## 八、缓存策略

### 8.1 两级缓存架构

```
┌─────────────────────────────────────┐
│           Application Layer         │
│    (Service → Cache Manager)        │
└──────────────┬──────────────────────┘
               │
    ┌──────────┴──────────┐
    │                     │
┌───┴───┐           ┌─────┴─────┐
│ Local │           │   Redis   │
│ Cache │           │   Cache   │
│ (Moka)│           │ (Redis)   │
└───────┘           └───────────┘
     │                     │
     └─────────┬───────────┘
               │
        ┌──────┴──────┐
        │  PostgreSQL │
        └─────────────┘
```

### 8.2 缓存配置

```rust
pub struct CacheConfig {
    pub local_max_capacity: u64,      // 本地缓存最大容量
    pub local_time_to_live: Duration, // 本地缓存 TTL
    pub redis_url: String,            // Redis 连接地址
    pub redis_pool_size: u32,         // Redis 连接池大小
    pub redis_ttl: Duration,          // Redis 缓存 TTL
}
```

---

## 九、代码风格规范

### 9.1 命名约定

| 类型 | 约定 | 示例 |
|------|------|------|
| 模块 | 蛇形小写 | `user_storage` |
| 结构体 | 帕斯卡命名 | `UserStorage` |
| 函数 | 蛇形小写 | `create_user` |
| 常量 | 蛇形大写 | `MAX_CONNECTIONS` |
| 类型参数 | 简短驼峰 | `T: Into<String>` |
| 特征 | 形容词或名词 | `Storage` |

### 9.2 错误处理

- 使用 `Result<T, E>` 进行错误传播
- 使用 `?` 操作符进行错误传播
- 定义有意义的错误类型
- 提供错误的上下文信息

### 9.3 异步编程

- 使用 `async/await` 语法
- 使用适当的 `Send` 和 `Sync` 约束
- 避免在异步上下文中使用阻塞操作
- 使用连接池管理数据库连接

---

## 十、测试规范

### 10.1 测试分类

| 级别 | 覆盖率目标 | 说明 |
|------|-----------|------|
| 单元测试 | 80% | 测试单个函数或模块 |
| 集成测试 | 60% | 测试模块间交互 |
| API 测试 | 100% | 测试所有 API 端点 |

### 10.2 测试配置

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    async fn setup_test_db() -> PgPool {
        // 创建测试数据库连接
    }

    #[tokio::test]
    async fn test_user_registration() {
        // 测试用户注册功能
    }
}
```

---

## 十一、部署规范

### 11.1 环境配置

```yaml
# config.yaml
server:
  name: "localhost"
  host: "0.0.0.0"
  port: 8008

database:
  url: "postgres://synapse_user:synapse_password@localhost:5432/synapse_db"
  pool_size: 10

cache:
  redis_url: "redis://localhost:6379"
  local_max_capacity: 10000

jwt:
  secret: "${JWT_SECRET}"
  expiry: 86400
```

### 11.2 Docker 部署

```dockerfile
FROM rust:1.75 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/synapse-rust /usr/local/bin/
COPY config.yaml /etc/synapse/config.yaml
EXPOSE 8008
CMD ["synapse-rust"]
```

---

## 十二、参考资料

### 12.1 官方文档

- [Synapse 官方文档](https://element-hq.github.io/synapse/latest/)
- [Matrix 规范](https://spec.matrix.org/)
- [Axum 框架文档](https://docs.rs/axum/latest/axum/)
- [SQLx 文档](https://docs.rs/sqlx/latest/sqlx/)

### 12.2 相关资源

- [项目仓库](https://github.com/langkebo/synapse)
- [问题追踪](https://github.com/langkebo/synapse/issues)
- [贡献指南](CONTRIBUTING.md)

---

## 十三、变更日志

| 版本 | 日期 | 变更说明 |
|------|------|----------|
| 2.0.0 | 2026-01-28 | 代码重建，更新项目状态 |
| 1.2.0 | 2026-01-28 | 修复编译错误，更新 API 状态 |
| 1.1.0 | 2026-01-27 | 添加 E2EE 优化计划 |
| 1.0.0 | 2026-01-26 | 初始版本 |
