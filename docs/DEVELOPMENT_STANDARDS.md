# Synapse Rust 开发规范文档

> **版本**：1.0.0  
> **编制日期**：2026-01-28  
> **状态**：草稿  
> **适用范围**：Synapse Rust 项目所有开发人员

---

## 一、代码风格规范

### 1.1 格式化规范

代码格式化使用 rustfmt 工具自动执行，所有代码提交前必须通过格式化检查。项目的根目录下配置 rustfmt.toml 文件，定义项目特定的格式化规则。

缩进使用四个空格，不使用制表符。行宽限制为 120 个字符，超长行应适当换行。换行时，函数参数列表中的参数各自占一行，链式调用中的点号位于行首。空行使用应保持一致性：模块声明之间一个空行，函数定义之间两个空行，函数内部逻辑段落之间一个空行。

```rust
// 正确的格式化示例
fn complex_function(
    param1: Type1,
    param2: Type2,
    param3: Type3,
) -> Result<Type, Error> {
    let result = do_something(param1)?;

    if condition {
        handle_special_case();
    } else {
        handle_normal_case();
    }

    Ok(result)
}

// 错误的格式化示例
fn complex_function(param1:Type1,param2:Type2,param3:Type3)->Result<Type,Error>{
    let result=do_something(param1)?;
    if condition{handle_special_case();}else{handle_normal_case();}
    Ok(result)
}
```

### 1.2 命名规范

Rust 的命名约定因语言构造而异，遵循以下规范可确保代码一致性。

模块名使用蛇形小写（snake_case），例如 user_storage、room_service。结构体、枚举和特征名使用帕斯卡命名（PascalCase），例如 UserStorage、RoomEvent、StorageTrait。函数和方法名使用蛇形小写（snake_case），例如 create_user、get_by_id。常量名使用全大写蛇形（SCREAMING_SNAKE_CASE），例如 MAX_CONNECTIONS、DEFAULT_TIMEOUT。类型参数使用简短驼峰（SingleCase），例如 T、U、V。特征名通常使用形容词或名词形式，例如 Serialize、Storage、Service。

变量命名应具有描述性，避免使用无意义的单字母变量名（循环变量和泛型参数除外）。布尔变量应使用 is_、has_、can_ 等前缀，明确其语义。

```rust
// 命名示例
struct UserStorage { /* ... */ }
enum MembershipState { /* ... */ }
trait Storage { /* ... */ }

const MAX_POOL_SIZE: u32 = 100;
const DEFAULT_TIMEOUT_SECS: u64 = 30;

fn create_user() { /* ... */ }
fn get_by_id(id: &str) { /* ... */ }

let is_active = true;
let has_permission = false;
let user_count = 42;
```

### 1.3 注释规范

注释应解释「为什么」而非「是什么」，代码本身应尽可能自文档化。公共 API 必须编写文档注释，说明功能、参数和返回值。

单行注释使用 //，放置于代码上方或行尾。块注释使用 /* */，仅在注释大段代码时使用。文档注释使用 ///，支持 Markdown 格式，可被 rustdoc 工具生成文档。

```rust
/// 创建新用户
///
/// 此方法会执行以下操作：
/// 1. 检查用户名唯一性
/// 2. 对密码进行哈希处理
/// 3. 创建用户记录
/// 4. 创建设备记录
/// 5. 生成访问令牌
///
/// # 参数
///
/// * `username` - 用户名，必须唯一
/// * `password` - 原始密码，将被哈希处理
/// * `is_admin` - 是否为管理员用户
///
/// # 返回
///
/// 返回创建的用户信息和令牌元组
///
/// # 错误
///
/// 如果用户名已被占用，返回 [`ApiError::conflict`]
pub async fn create_user(
    username: &str,
    password: &str,
    is_admin: bool,
) -> Result<(User, TokenInfo), ApiError> {
    // 1. 检查用户名唯一性
    // 使用查询锁防止并发创建冲突
    if exists_by_username(username).await? {
        return Err(ApiError::conflict("Username already taken".to_string()));
    }

    // 2. 哈希密码
    let password_hash = hash_password(password)?;

    // ... 其他逻辑
    Ok((user, token_info))
}
```

---

## 二、错误处理规范

### 2.1 错误类型定义

项目定义统一的错误类型 ApiError，包含错误码、错误消息和 HTTP 状态码。所有向上层返回的错误都应转换为这种类型。

```rust
/// API 错误类型
///
/// 包含错误码、错误消息和 HTTP 状态码。
/// 实现了标准错误接口，可被任何错误处理框架处理。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    /// 错误码，如 "BAD_REQUEST"、"NOT_FOUND" 等
    pub code: String,
    /// 人类可读的错误消息
    pub message: String,
    /// HTTP 状态码
    pub status: u16,
}

impl ApiError {
    /// 创建 400 Bad Request 错误
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            code: "BAD_REQUEST".to_string(),
            message: message.into(),
            status: 400,
        }
    }

    /// 创建 401 Unauthorized 错误
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            code: "UNAUTHORIZED".to_string(),
            message: message.into(),
            status: 401,
        }
    }

    /// 创建 403 Forbidden 错误
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self {
            code: "FORBIDDEN".to_string(),
            message: message.into(),
            status: 403,
        }
    }

    /// 创建 404 Not Found 错误
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: "NOT_FOUND".to_string(),
            message: message.into(),
            status: 404,
        }
    }

    /// 创建 409 Conflict 错误
    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            code: "CONFLICT".to_string(),
            message: message.into(),
            status: 409,
        }
    }

    /// 创建 500 Internal Server Error 错误
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: "INTERNAL_ERROR".to_string(),
            message: message.into(),
            status: 500,
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

/// API 结果类型
pub type ApiResult<T> = Result<T, ApiError>;
```

### 2.2 错误传播规范

错误传播使用 ? 操作符，保持代码简洁。对于需要添加上下文的错误，使用 map_err 或 with_context 方法。

```rust
// 使用 ? 操作符传播错误
async fn create_user(&self, username: &str, password: &str) -> ApiResult<User> {
    // 验证用户名格式
    self.validate_username(username)?;  // ApiError 可直接传播

    // 检查用户名是否已存在
    let exists = self.user_storage.exists_by_username(username).await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    if exists {
        return Err(ApiError::conflict("Username already taken".to_string()));
    }

    // 创建用户
    let user = self.user_storage.create_user(username, password).await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(user)
}

// 使用 with_context 添加上下文
async fn get_user(&self, user_id: &str) -> ApiResult<User> {
    self.user_storage.get_by_id(user_id)
        .await
        .map_err(|e| {
            ApiError::internal(format!(
                "Failed to get user {}: {}",
                user_id,
                e
            ))
        })?
        .ok_or_else(|| ApiError::not_found(format!("User {} not found", user_id)))
}
```

### 2.3 不可恢复错误处理

对于不可恢复的错误（如配置错误、资源初始化失败），使用 panic 或 abort。测试代码中使用 assert!、assert_eq!、assert_ne! 进行断言。

```rust
// 配置验证，确保启动时发现配置错误
fn validate_config(config: &Config) {
    assert!(!config.database.url.is_empty(), "Database URL must be set");
    assert!(config.jwt.secret.len() >= 32, "JWT secret must be at least 32 characters");
    assert!(config.server.port > 0 && config.server.port < 65536, "Invalid port number");
}

// 测试用例
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = ApiError::not_found("User not found");
        assert_eq!(err.code, "NOT_FOUND");
        assert_eq!(err.status, 404);
        assert_eq!(err.message, "User not found");
    }
}
```

---

## 三、异步编程规范

### 3.1 异步函数定义

所有涉及 I/O 操作的函数应定义为异步函数，使用 async 关键字。异步函数返回 Future trait 对象，由 Tokio 运行时调度执行。

```rust
// 异步函数定义
async fn fetch_user(user_id: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(User, "SELECT * FROM users WHERE user_id = $1", user_id)
        .fetch_optional(&self.pool)
        .await
}

// 如果函数不涉及 I/O，应使用同步版本
fn hash_password(password: &str) -> Result<String, CryptoError> {
    // 密码学操作是 CPU 密集型，但使用 Rust 的优化实现
    // 不需要异步
    let config = argon2::Config::default();
    argon2::hash_encoded(password.as_bytes(), &[], &config)
}
```

### 3.2 Tokio 运行时使用

在程序入口点创建 Tokio 运行时，在异步代码中使用 #[tokio::main] 宏或手动创建运行时。避免在异步上下文中使用阻塞操作，所有 I/O 操作应使用异步版本。

```rust
// 使用 #[tokio::main] 宏（推荐）
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config()?;
    let server = SynapseServer::new(&config).await?;
    server.run().await?;
    Ok(())
}

// 对于需要自定义运行时的场景
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async {
        let config = load_config()?;
        let server = SynapseServer::new(&config).await?;
        server.run().await?;
        Ok(())
    })
}
```

### 3.3 Future 组合器使用

使用合适的 Future 组合器处理多个异步操作，避免嵌套回调。常用组合器包括 join!、try_join!、select!、race! 等。

```rust
// 并发执行多个异步操作
async fn create_user_with_device(
    &self,
    username: &str,
    password: &str,
    device_name: &str,
) -> Result<(User, Device, Token), sqlx::Error> {
    // 并发执行：创建用户、创建设备
    let (user, device) = try_join!(
        self.user_storage.create_user(username, password),
        self.device_storage.create_device(username, device_name)
    )?;

    // 生成令牌
    let token = self.token_storage.create(&user.user_id, &device.device_id).await?;

    Ok((user, device, token))
}

// 超时控制
async fn fetch_with_timeout(url: &str) -> Result<Response, reqwest::Error> {
    let client = reqwest::Client::new();
    tokio::time::timeout(Duration::from_secs(10), client.get(url).send()).await
        .map_err(|_| reqwest::Error::from(reqwest::error::ErrorKind::Timeout))?
}

// 竞态处理：优先使用缓存，缓存未命中时回源数据库
async fn get_cached_or_fetch(
    &self,
    key: &str,
) -> Result<Option<Value>, CacheError> {
    // 先查缓存
    if let Some(value) = self.cache.get(key).await? {
        return Ok(Some(value));
    }

    // 缓存未命中，查数据库
    let value = self.storage.get(key).await?;

    // 写入缓存
    if let Some(ref v) = value {
        self.cache.set(key, v, None).await?;
    }

    Ok(value)
}
```

### 3.4 Send 和 Sync 约束

异步代码必须满足 Send 和 Sync 约束，确保线程安全。使用 Arc 而不是 Rc 共享所有权，使用 Mutex 或 RwLock 提供互斥访问。

```rust
// 正确：使用 Arc 共享状态
pub struct AppState {
    pub services: Arc<ServiceContainer>,
    pub cache: Arc<CacheManager>,
}

// 正确：使用 Mutex 保护可变状态
pub struct CacheManager {
    local: Arc<RwLock<LocalCache>>,  // 使用读写锁
    redis: Mutex<Option<RedisClient>>,
}

// 错误：Rc 不能跨线程共享
// pub struct BadState {
//     data: Rc<User>,
// }
```

---

## 四、测试规范

### 4.1 测试文件组织

测试代码与源代码放在同一模块中，使用 #[cfg(test)] 属性标记。单元测试放在源文件末尾，集成测试放在 tests 目录中。

```
src/
├── lib.rs
├── main.rs
└── storage/
    ├── mod.rs
    ├── user.rs          # 用户存储实现
    └── tests/
        └── mod.rs       # 用户存储集成测试

tests/
├── api/
│   ├── mod.rs
│   ├── auth_test.rs
│   └── room_test.rs
└── integration_test.rs  # 端到端测试
```

### 4.2 单元测试编写

每个公开函数和复杂私有函数应编写单元测试。测试应覆盖正常路径、边界条件和错误情况。使用 #[test] 属性标记测试函数，使用 #[tokio::test] 标记异步测试。

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // 同步测试
    #[test]
    fn test_error_display() {
        let err = ApiError::not_found("User not found");
        assert_eq!(err.to_string(), "NOT_FOUND: User not found");
    }

    // 异步测试
    #[tokio::test]
    async fn test_user_creation() {
        // 设置测试数据库
        let pool = setup_test_db().await;

        // 创建存储实例
        let storage = UserStorage::new(&pool);

        // 测试正常创建
        let user = storage.create_user("testuser", "password123", false).await.unwrap();
        assert_eq!(user.username, "testuser");
        assert!(!user.admin);

        // 测试重复创建应失败
        let result = storage.create_user("testuser", "password123", false).await;
        assert!(result.is_err());
    }

    // 参数化测试
    #[test]
    fn test_username_validation() {
        let valid_names = vec!["alice", "bob123", "user_name", "@alice:localhost"];
        for name in valid_names {
            assert!(is_valid_username(name), "Expected {} to be valid", name);
        }

        let invalid_names = vec!["", "Abc", "user name", "user@name"];
        for name in invalid_names {
            assert!(!is_valid_username(name), "Expected {} to be invalid", name);
        }
    }

    // 测试夹具（Fixtures）
    #[tokio::test]
    async fn test_user_query_with_fixture(test_db: TestDb) {
        let storage = test_db.user_storage();

        // 插入测试数据
        storage.create_user("user1", "pass1", false).await.unwrap();
        storage.create_user("user2", "pass2", false).await.unwrap();

        // 执行查询
        let users = storage.list_all().await.unwrap();
        assert_eq!(users.len(), 2);
    }
}
```

### 4.3 集成测试编写

集成测试验证多个模块的协作，确保组件间的接口正确。测试环境应尽可能接近生产环境，使用真实的数据库和缓存实例（测试专用实例）。

```rust
// tests/api/auth_test.rs

use synapse_rust::prelude::*;

#[tokio::test]
async fn test_register_and_login_flow() {
    // 1. 设置测试环境
    let config = TestConfig::new();
    let server = SynapseServer::new(&config).await.unwrap();

    // 2. 注册新用户
    let register_response = reqwest::Client::new()
        .post(&format!("{}/_matrix/client/r0/register", server.url()))
        .json(&json!({
            "username": "testuser",
            "password": "testpassword"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(register_response.status(), 200);
    let register_body: RegisterResponse = register_response.json().await.unwrap();
    assert!(register_body.user_id.starts_with("@testuser:"));

    // 3. 使用凭据登录
    let login_response = reqwest::Client::new()
        .post(&format!("{}/_matrix/client/r0/login", server.url()))
        .json(&json!({
            "type": "m.login.password",
            "user": "testuser",
            "password": "testpassword"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(login_response.status(), 200);
    let login_body: LoginResponse = login_response.json().await.unwrap();
    assert!(!login_body.access_token.is_empty());

    // 4. 使用访问令牌获取用户信息
    let whoami_response = reqwest::Client::new()
        .get(&format!("{}/_matrix/client/r0/account/whoami", server.url()))
        .bearer_auth(&login_body.access_token)
        .send()
        .await
        .unwrap();

    assert_eq!(whoami_response.status(), 200);
    let whoami_body: WhoamiResponse = whoami_response.json().await.unwrap();
    assert_eq!(whoami_body.user_id, register_body.user_id);
}
```

---

## 五、Git 提交规范

### 5.1 提交信息格式

提交信息应遵循以下格式，包含类型、范围和描述：

```
<类型>(<范围>): <描述>

[可选的正文]

[可选的脚注]
```

类型标识本次提交的性质：feat 表示新功能、fix 表示修复 bug、docs 表示文档更新、style 表示代码格式调整、refactor 表示代码重构、test 表示添加测试、chore 表示构建或辅助工具更新。

```text
feat(auth): 添加用户注册功能

实现用户注册流程，包括：
- 用户名格式验证
- 密码强度检查
- 唯一性检查
- 用户记录创建
- 设备记录创建
- 初始令牌生成

Fixes #123
Ref #45
```

### 5.2 分支策略

主分支（main）始终保持可发布状态，所有开发在特性分支（feature/*）进行。功能开发完成后创建 Pull Request 进行代码审查，审查通过后合并到主分支。

分支命名规范：feature/* 表示新功能、bugfix/* 表示修复 bug、hotfix/* 表示紧急修复、refactor/* 表示代码重构。

### 5.3 代码审查要点

代码审查应关注以下方面：功能正确性（代码是否正确实现了需求）、代码质量（命名、注释、代码风格）、性能影响（是否有性能问题）、安全性（是否有安全漏洞）、测试覆盖（是否有必要的测试）。

---

## 六、文档规范

### 6.1 代码文档

所有公共 API 必须编写文档注释，说明功能、参数、返回值和可能的错误。文档使用 Markdown 格式，支持标题、列表、代码块等格式。示例代码应放在 #[doc = ""] 属性或 ```rust,ignore 代码块中。

```rust
/// 根据用户 ID 获取用户信息
///
/// 此方法会查询数据库并返回用户信息。如果用户不存在，返回 `None`。
///
/// # 参数
///
/// * `user_id` - 用户的完整 ID，格式为 `@username:servername`
///
/// # 返回
///
/// 返回包含用户信息的 `Some(User)`，如果用户不存在则返回 `None`。
///
/// # 示例
///
/// ```
/// use synapse_rust::storage::UserStorage;
///
/// let storage = UserStorage::new(&pool);
/// let user = storage.get_by_id("@alice:localhost").await?;
/// match user {
///     Some(u) => println!("Found user: {}", u.username),
///     None => println!("User not found"),
/// }
/// ```
///
/// # 错误
///
/// 如果数据库查询失败，返回 `sqlx::Error`。
pub async fn get_by_id(&self, user_id: &str) -> Result<Option<User>, sqlx::Error>
```

### 6.2 项目文档

项目文档使用 Markdown 格式编写，放在 docs 目录下。文档应保持更新，与代码实现同步。重要的设计决策应在文档中记录原因。

---

## 七、依赖管理规范

### 7.1 Cargo.toml 规范

依赖应指定精确版本或使用兼容版本范围。避免使用 * 通配符版本。开发依赖使用 dev-dependencies 节，只有编译时依赖使用 build-dependencies 节。

```toml
[package]
name = "synapse-rust"
version = "0.1.0"
edition = "2021"

[dependencies]
# 精确版本
tokio = { version = "1.35", features = ["full"] }

# 兼容版本范围（次要版本兼容）
axum = "0.7"

# 仅开发依赖
[dev-dependencies]
tokio-test = "0.4"
reqwest = { version = "0.11", features = ["json"] }

[profile.release]
opt-level = 3
lto = true
```

### 7.2 依赖更新策略

定期检查依赖更新，使用 cargo outdated 工具查看可用的更新。重大版本更新需要充分测试后再合并。所有安全漏洞修复应优先处理。

---

## 八、附录

### 8.1 常用 rustfmt 配置

```toml
# .rustfmt.toml
max_width = 120
tab_spaces = 4
edition = "2021"
fn_single_line = false
fn_params_layout = "Vertical"
where_layout = "Vertical"
force_multiline_blocks = true
group_imports = "StdExternalCrate"
reorder_modules = true
```

### 8.2 clippy 注意事项

项目应通过 clippy 的所有检查（允许的警告除外）。使用 cargo clippy --fix 自动修复可修复的问题，审查其他警告并酌情处理。

### 8.3 代码审查清单

审查代码时检查以下项目：功能是否正确实现、错误处理是否完善、并发是否安全、性能是否合理、命名是否清晰、注释是否充分、测试是否充分、文档是否同步。

---

**编制人**：  
**审核人**：  
**批准人**：  
