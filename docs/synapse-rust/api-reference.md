# Synapse Rust Matrix Server API Reference

> **服务器地址**: `http://localhost:8008`  
> **版本**: 0.1.0  
> **文档版本**: 6.0  
> **最后更新**: 2026-02-13

---

## 目录

1. [概述](#1-概述)
2. [认证方式](#2-认证方式)
3. [核心客户端 API](#3-核心客户端-api)
4. [管理员 API](#4-管理员-api)
5. [联邦通信 API](#5-联邦通信-api)
6. [好友系统 API](#6-好友系统-api)
7. [端到端加密 API](#7-端到端加密-api)
8. [媒体文件 API](#8-媒体文件-api)
9. [语音消息 API](#9-语音消息-api)
10. [VoIP API](#10-voip-api)
11. [密钥备份 API](#11-密钥备份-api)
12. [外部服务 API](#12-外部服务-api)
13. [错误码参考](#13-错误码参考)
14. [API 统计](#14-api-统计)

---

## 1. 概述

### 1.1 API 分类

| 分类 | 端点数量 | 说明 |
|------|---------|------|
| 核心客户端 API | 68 | 用户认证、房间管理、消息操作 |
| 管理员 API | 55 | 服务器管理、用户管理、房间管理 |
| 联邦通信 API | 40 | 服务器间通信 |
| 好友系统 API | 13 | 基于 Matrix 房间的好友管理 |
| 端到端加密 API | 8 | E2EE 相关功能 |
| 媒体文件 API | 9 | 媒体上传下载 |
| 语音消息 API | 10 | 语音消息处理 |
| VoIP API | 3 | VoIP 配置 |
| 密钥备份 API | 11 | 密钥备份管理 |
| 外部服务 API | 6 | 外部服务配置管理 |
| **总计** | **223** | |

### 1.2 基础 URL

```
http://localhost:8008
```

### 1.3 请求头

| 请求头 | 说明 | 示例 |
|--------|------|------|
| `Authorization` | Bearer Token 认证 | `Bearer syt_abc123...` |
| `Content-Type` | 请求体格式 | `application/json` |
| `Accept` | 响应格式 | `application/json` |

---

## 2. 认证方式

### 2.1 Bearer Token 认证

大多数 API 需要在请求头中携带 Access Token：

```http
Authorization: Bearer <access_token>
```

### 2.2 获取 Token

通过登录接口获取：

```http
POST /_matrix/client/r0/login
Content-Type: application/json

{
  "type": "m.login.password",
  "user": "alice",
  "password": "password123"
}
```

**响应**:
```json
{
  "access_token": "syt_abc123...",
  "device_id": "DEVICEID",
  "user_id": "@alice:example.com",
  "expires_in": 86400000,
  "refresh_token": "abc123..."
}
```

---

## 3. 核心客户端 API

### 3.1 健康检查与版本

#### 3.1.1 服务器欢迎信息

| 属性 | 值 |
|------|-----|
| **端点** | `/` |
| **方法** | `GET` |
| **认证** | 不需要 |

**响应示例**:
```json
{
  "msg": "Synapse Rust Matrix Server",
  "version": "0.1.0"
}
```

#### 3.1.2 健康检查

| 属性 | 值 |
|------|-----|
| **端点** | `/health` |
| **方法** | `GET` |
| **认证** | 不需要 |

**响应示例**:
```json
{
  "status": "ok",
  "database": "connected",
  "cache": "connected"
}
```

#### 3.1.3 获取客户端 API 版本

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/versions` |
| **方法** | `GET` |
| **认证** | 不需要 |

**响应示例**:
```json
{
  "versions": ["r0.0.1", "r0.1.0", "r0.2.0", "r0.3.0", "r0.4.0", "r0.5.0", "r0.6.0"],
  "unstable_features": {
    "m.lazy_load_members": true,
    "m.require_identity_server": false,
    "m.supports_login_via_phone_number": true
  }
}
```

#### 3.1.4 获取服务端版本

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/version` |
| **方法** | `GET` |
| **认证** | 不需要 |

**响应示例**:
```json
{
  "version": "0.1.0"
}
```

#### 3.1.5 获取服务器 Well-Known (Server)

| 属性 | 值 |
|------|-----|
| **端点** | `/.well-known/matrix/server` |
| **方法** | `GET` |
| **认证** | 不需要 |

**响应示例**:
```json
{
  "m.server": "example.com:8448"
}
```

#### 3.1.6 获取服务器 Well-Known (Client)

| 属性 | 值 |
|------|-----|
| **端点** | `/.well-known/matrix/client` |
| **方法** | `GET` |
| **认证** | 不需要 |

**响应示例**:
```json
{
  "m.homeserver": {
    "base_url": "https://example.com"
  }
}
```

---

### 3.2 用户注册与认证

#### 3.2.1 检查用户名可用性

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/register/available` |
| **方法** | `GET` |
| **认证** | 不需要 |

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `username` | string | 是 | 要检查的用户名 |

**响应示例**:
```json
{
  "available": true,
  "username": "alice"
}
```

#### 3.2.2 请求邮箱验证

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/register/email/requestToken` |
| **方法** | `POST` |
| **认证** | 不需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `email` | string | 是 | 邮箱地址 |
| `client_secret` | string | 否 | 客户端密钥 |

**请求示例**:
```json
{
  "email": "user@example.com",
  "client_secret": "abc123"
}
```

**响应示例**:
```json
{
  "sid": "123",
  "submit_url": "https://localhost:8008/_matrix/client/r0/register/email/submitToken",
  "expires_in": 3600
}
```

#### 3.2.3 提交邮箱验证令牌

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/register/email/submitToken` |
| **方法** | `POST` |
| **认证** | 不需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `sid` | string | 是 | 会话ID |
| `client_secret` | string | 是 | 客户端密钥 |
| `token` | string | 是 | 验证令牌 |

**响应示例**:
```json
{
  "success": true
}
```

#### 3.2.4 用户注册

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/register` |
| **方法** | `POST` |
| **认证** | 不需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `username` | string | 是 | 用户名 |
| `password` | string | 是 | 密码 |
| `device_id` | string | 否 | 设备ID |
| `initial_device_display_name` | string | 否 | 设备显示名称 |
| `displayname` | string | 否 | 显示名称 |
| `auth` | object | 否 | 认证信息 |

**请求示例**:
```json
{
  "username": "alice",
  "password": "password123",
  "device_id": "DEVICEID",
  "initial_device_display_name": "My Device"
}
```

**响应示例**:
```json
{
  "user_id": "@alice:example.com",
  "access_token": "syt_abc123...",
  "device_id": "DEVICEID",
  "expires_in": 86400000,
  "refresh_token": "abc123..."
}
```

#### 3.2.5 用户登录

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/login` |
| **方法** | `POST` |
| **认证** | 不需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `type` | string | 是 | 登录类型，如 `m.login.password` |
| `user` | string | 是 | 用户名或完整用户ID |
| `password` | string | 是 | 密码 |
| `device_id` | string | 否 | 设备ID |
| `initial_device_display_name` | string | 否 | 设备显示名称 |

**请求示例**:
```json
{
  "type": "m.login.password",
  "user": "alice",
  "password": "password123",
  "device_id": "DEVICEID"
}
```

**响应示例**:
```json
{
  "user_id": "@alice:example.com",
  "access_token": "syt_abc123...",
  "device_id": "DEVICEID",
  "expires_in": 86400000,
  "refresh_token": "abc123...",
  "well_known": {
    "m.homeserver": {
      "base_url": "http://localhost:8008"
    }
  }
}
```

#### 3.2.6 退出登录

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/logout` |
| **方法** | `POST` |
| **认证** | 需要 |

**响应示例**:
```json
{}
```

#### 3.2.7 退出所有设备

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/logout/all` |
| **方法** | `POST` |
| **认证** | 需要 |

**响应示例**:
```json
{}
```

#### 3.2.8 刷新令牌

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/refresh` |
| **方法** | `POST` |
| **认证** | 不需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `refresh_token` | string | 是 | 刷新令牌 |

**响应示例**:
```json
{
  "access_token": "syt_new_token...",
  "expires_in": 86400000,
  "refresh_token": "new_refresh_token...",
  "device_id": "DEVICEID"
}
```

---

### 3.3 账户管理

#### 3.3.1 获取当前用户信息

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/account/whoami` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "user_id": "@alice:example.com",
  "displayname": "Alice",
  "avatar_url": "mxc://example.com/avatar",
  "admin": false
}
```

#### 3.3.2 获取用户资料

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/account/profile/{user_id}` |
| **方法** | `GET` |
| **认证** | 需要 |

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `user_id` | string | 是 | 用户ID |

**响应示例**:
```json
{
  "displayname": "Alice",
  "avatar_url": "mxc://example.com/avatar"
}
```

#### 3.3.3 更新显示名称

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/account/profile/{user_id}/displayname` |
| **方法** | `PUT` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `displayname` | string | 是 | 新的显示名称（最大255字符） |

**请求示例**:
```json
{
  "displayname": "Alice Smith"
}
```

**响应示例**:
```json
{}
```

#### 3.3.4 更新头像

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/account/profile/{user_id}/avatar_url` |
| **方法** | `PUT` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `avatar_url` | string | 是 | MXC URL 格式的头像地址（最大255字符） |

**请求示例**:
```json
{
  "avatar_url": "mxc://example.com/new_avatar"
}
```

**响应示例**:
```json
{}
```

#### 3.3.5 修改密码

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/account/password` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `new_password` | string | 是 | 新密码 |

**响应示例**:
```json
{}
```

#### 3.3.6 停用账户

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/account/deactivate` |
| **方法** | `POST` |
| **认证** | 需要 |

**响应示例**:
```json
{}
```

---

### 3.4 过滤器

#### 3.4.1 创建过滤器

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/user/{user_id}/filter` |
| **方法** | `POST` |
| **认证** | 需要 |

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `user_id` | string | 是 | 用户ID |

**请求体**: 过滤器定义对象

**响应示例**:
```json
{
  "filter_id": "filter_123"
}
```

#### 3.4.2 获取过滤器

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/user/{user_id}/filter/{filter_id}` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**: 过滤器定义对象

---

### 3.5 账户数据

#### 3.5.1 设置账户数据

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/user/{user_id}/account_data/{type}` |
| **方法** | `PUT` |
| **认证** | 需要 |

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `user_id` | string | 是 | 用户ID |
| `type` | string | 是 | 数据类型 |

**请求体**: 账户数据对象

**响应示例**:
```json
{}
```

#### 3.5.2 获取账户数据

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/user/{user_id}/account_data/{type}` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**: 账户数据对象

---

### 3.6 用户目录

#### 3.6.1 搜索用户

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/user_directory/search` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `search_term` | string | 是 | 搜索关键词 |
| `limit` | integer | 否 | 返回结果数量限制（默认10） |

**请求示例**:
```json
{
  "search_term": "alice",
  "limit": 10
}
```

**响应示例**:
```json
{
  "results": [
    {
      "user_id": "@alice:example.com",
      "display_name": "Alice",
      "avatar_url": "mxc://example.com/avatar"
    }
  ],
  "limited": false
}
```

#### 3.6.2 获取用户列表

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/user_directory/list` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `limit` | integer | 否 | 返回结果数量限制（默认50） |
| `offset` | integer | 否 | 偏移量（默认0） |

**响应示例**:
```json
{
  "total": 100,
  "offset": 0,
  "users": [
    {
      "user_id": "@alice:example.com",
      "display_name": "Alice",
      "avatar_url": "mxc://example.com/avatar"
    }
  ]
}
```

---

### 3.7 设备管理

#### 3.7.1 获取设备列表

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/devices` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "devices": [
    {
      "device_id": "DEVICEID",
      "display_name": "My Device",
      "last_seen_ts": 1234567890000,
      "user_id": "@alice:example.com"
    }
  ]
}
```

#### 3.7.2 获取设备信息

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/devices/{device_id}` |
| **方法** | `GET` |
| **认证** | 需要 |

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `device_id` | string | 是 | 设备ID |

**响应示例**:
```json
{
  "device_id": "DEVICEID",
  "display_name": "My Device",
  "last_seen_ts": 1234567890000,
  "user_id": "@alice:example.com"
}
```

#### 3.7.3 更新设备

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/devices/{device_id}` |
| **方法** | `PUT` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `display_name` | string | 是 | 设备显示名称 |

**请求示例**:
```json
{
  "display_name": "My Laptop"
}
```

**响应示例**:
```json
{}
```

#### 3.7.4 删除设备

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/devices/{device_id}` |
| **方法** | `DELETE` |
| **认证** | 需要 |

**响应示例**:
```json
{}
```

#### 3.7.5 批量删除设备

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/delete_devices` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `devices` | array | 是 | 要删除的设备ID列表 |

**请求示例**:
```json
{
  "devices": ["DEVICE1", "DEVICE2"]
}
```

**响应示例**:
```json
{}
```

---

### 3.8 在线状态

#### 3.8.1 获取在线状态

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/presence/{user_id}/status` |
| **方法** | `GET` |
| **认证** | 需要 |

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `user_id` | string | 是 | 用户ID |

**响应示例**:
```json
{
  "presence": "online",
  "status_msg": "Working from home"
}
```

#### 3.8.2 设置在线状态

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/presence/{user_id}/status` |
| **方法** | `PUT` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `presence` | string | 是 | 状态：`online`、`offline`、`unavailable` |
| `status_msg` | string | 否 | 状态消息 |

**请求示例**:
```json
{
  "presence": "online",
  "status_msg": "Working from home"
}
```

**响应示例**:
```json
{}
```

---

### 3.9 同步与状态

#### 3.9.1 同步数据

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/sync` |
| **方法** | `GET` |
| **认证** | 需要 |

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `since` | string | 否 | 上次同步的令牌 |
| `timeout` | integer | 否 | 长轮询超时时间（毫秒，默认30000） |
| `full_state` | boolean | 否 | 是否返回完整状态 |
| `set_presence` | string | 否 | 设置在线状态（默认online） |

**响应示例**:
```json
{
  "next_batch": "s72594_4483_1934",
  "rooms": {
    "join": {},
    "invite": {},
    "leave": {}
  },
  "presence": {
    "events": []
  },
  "account_data": {
    "events": []
  }
}
```

#### 3.9.2 设置打字状态

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/typing/{user_id}` |
| **方法** | `PUT` |
| **认证** | 需要 |

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `room_id` | string | 是 | 房间ID |
| `user_id` | string | 是 | 用户ID |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `typing` | boolean | 是 | 是否正在输入 |

**请求示例**:
```json
{
  "typing": true
}
```

**响应示例**:
```json
{}
```

#### 3.9.3 发送已读回执

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/receipt/{receipt_type}/{event_id}` |
| **方法** | `POST` |
| **认证** | 需要 |

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `room_id` | string | 是 | 房间ID |
| `receipt_type` | string | 是 | 回执类型：`m.read`、`m.read.private` |
| `event_id` | string | 是 | 事件ID |

**响应示例**:
```json
{}
```

#### 3.9.4 设置已读标记

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/read_markers` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `event_id` | string | 否 | 完全读取的事件ID |
| `m.fully_read` | string | 否 | 完全读取的事件ID |
| `m.read` | string | 否 | 读取位置事件ID |

**请求示例**:
```json
{
  "m.fully_read": "$event_id:example.com",
  "m.read": "$event_id:example.com"
}
```

**响应示例**:
```json
{}
```

---

### 3.10 房间管理

#### 3.10.1 创建房间

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/createRoom` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `visibility` | string | 否 | 可见性：`public`、`private` |
| `room_alias_name` | string | 否 | 房间别名（最大255字符） |
| `name` | string | 否 | 房间名称（最大255字符） |
| `topic` | string | 否 | 房间主题（最大4096字符） |
| `invite` | array | 否 | 邀请的用户ID列表（最多100个） |
| `preset` | string | 否 | 预设：`private_chat`、`public_chat`、`trusted_private_chat` |

**请求示例**:
```json
{
  "name": "My Room",
  "topic": "A test room",
  "preset": "private_chat",
  "invite": ["@bob:example.com"]
}
```

**响应示例**:
```json
{
  "room_id": "!room:example.com"
}
```

#### 3.10.2 加入房间

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/join` |
| **方法** | `POST` |
| **认证** | 需要 |

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `room_id` | string | 是 | 房间ID |

**响应示例**:
```json
{}
```

#### 3.10.3 通过ID或别名加入房间

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/join/{room_id_or_alias}` |
| **方法** | `POST` |
| **认证** | 需要 |

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `room_id_or_alias` | string | 是 | 房间ID或别名 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `server_name` | array | 否 | 服务器名称列表 |

**响应示例**:
```json
{
  "room_id": "!room:example.com"
}
```

#### 3.10.4 离开房间

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/leave` |
| **方法** | `POST` |
| **认证** | 需要 |

**响应示例**:
```json
{}
```

#### 3.10.5 忘记房间

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/forget` |
| **方法** | `POST` |
| **认证** | 需要 |

**响应示例**:
```json
{}
```

#### 3.10.6 踢出用户

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/kick` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `user_id` | string | 是 | 要踢出的用户ID |
| `reason` | string | 否 | 原因（最大512字符） |

**请求示例**:
```json
{
  "user_id": "@bob:example.com",
  "reason": "Spamming"
}
```

**响应示例**:
```json
{}
```

#### 3.10.7 封禁用户

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/ban` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `user_id` | string | 是 | 要封禁的用户ID |
| `reason` | string | 否 | 原因（最大512字符） |

**请求示例**:
```json
{
  "user_id": "@bob:example.com",
  "reason": "Harassment"
}
```

**响应示例**:
```json
{}
```

#### 3.10.8 解除封禁

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/unban` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `user_id` | string | 是 | 要解除封禁的用户ID |

**请求示例**:
```json
{
  "user_id": "@bob:example.com"
}
```

**响应示例**:
```json
{}
```

#### 3.10.9 邀请用户

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/invite` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `user_id` | string | 是 | 要邀请的用户ID |

**请求示例**:
```json
{
  "user_id": "@bob:example.com"
}
```

**响应示例**:
```json
{}
```

#### 3.10.10 获取房间信息

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/summary` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "room_id": "!room:example.com",
  "name": "My Room",
  "topic": "A test room"
}
```

#### 3.10.11 获取房间成员

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/members` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "chunk": [
    {
      "type": "m.room.member",
      "state_key": "@alice:example.com",
      "content": {
        "membership": "join",
        "displayname": "Alice"
      },
      "sender": "@alice:example.com",
      "event_id": "$event_id:example.com"
    }
  ]
}
```

#### 3.10.12 获取用户房间列表

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/user/{user_id}/rooms` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "joined_rooms": ["!room1:example.com", "!room2:example.com"]
}
```

#### 3.10.13 升级房间

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/upgrade` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `new_version` | string | 否 | 新房间版本（默认6） |

**请求示例**:
```json
{
  "new_version": "6"
}
```

**响应示例**:
```json
{
  "replacement_room": "!new_room:example.com"
}
```

#### 3.10.14 房间初始同步

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/initialSync` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "room_id": "!room:example.com",
  "info": {
    "name": "My Room",
    "topic": "A test room"
  },
  "state": [],
  "messages": {
    "chunk": []
  },
  "members": []
}
```

#### 3.10.15 时间戳转事件

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/timestamp_to_event` |
| **方法** | `GET` |
| **认证** | 需要 |

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `ts` | integer | 是 | 时间戳 |
| `dir` | string | 否 | 方向：`f`（向前）、`b`（向后） |

**响应示例**:
```json
{
  "event_id": "$event_id:example.com",
  "origin_server_ts": 1234567890000
}
```

---

### 3.11 房间状态与消息

#### 3.11.1 获取房间状态

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/state` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "state": [
    {
      "type": "m.room.name",
      "state_key": "",
      "content": {
        "name": "My Room"
      },
      "sender": "@alice:example.com",
      "event_id": "$event_id:example.com"
    }
  ]
}
```

#### 3.11.2 获取特定类型状态事件

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/state/{event_type}` |
| **方法** | `GET` |
| **认证** | 需要 |

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `room_id` | string | 是 | 房间ID |
| `event_type` | string | 是 | 事件类型 |

**响应示例**:
```json
{
  "events": []
}
```

#### 3.11.3 获取状态事件（带状态键）

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/state/{event_type}/{state_key}` |
| **方法** | `GET` |
| **认证** | 需要 |

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `room_id` | string | 是 | 房间ID |
| `event_type` | string | 是 | 事件类型 |
| `state_key` | string | 是 | 状态键 |

**响应示例**:
```json
{
  "type": "m.room.member",
  "event_id": "$event_id:example.com",
  "sender": "@alice:example.com",
  "content": {
    "displayname": "Alice",
    "membership": "join"
  },
  "state_key": "@alice:example.com"
}
```

#### 3.11.4 设置房间状态

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/state/{event_type}/{state_key}` |
| **方法** | `PUT` |
| **认证** | 需要 |

**请求体**: 根据事件类型而定

**响应示例**:
```json
{
  "event_id": "$event_id:example.com",
  "type": "m.room.name",
  "state_key": ""
}
```

#### 3.11.5 发送状态事件

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/state/{event_type}` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**: 根据事件类型而定

**响应示例**:
```json
{
  "event_id": "$event_id:example.com",
  "type": "m.room.name",
  "state_key": "@alice:example.com"
}
```

#### 3.11.6 发送事件/消息

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/send/{event_type}/{txn_id}` |
| **方法** | `PUT` |
| **认证** | 需要 |

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `room_id` | string | 是 | 房间ID |
| `event_type` | string | 是 | 事件类型 |
| `txn_id` | string | 是 | 事务ID |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `msgtype` | string | 否 | 消息类型（默认m.room.message） |
| `body` | any | 是 | 消息内容（最大64KB） |

**请求示例**:
```json
{
  "msgtype": "m.text",
  "body": "Hello, World!"
}
```

**响应示例**:
```json
{
  "event_id": "$event_id:example.com"
}
```

#### 3.11.7 获取房间消息

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/messages` |
| **方法** | `GET` |
| **认证** | 需要 |

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `from` | integer | 否 | 起始位置（默认0） |
| `limit` | integer | 否 | 数量限制（默认10） |
| `dir` | string | 否 | 方向：`f`（向前）、`b`（向后，默认） |

**响应示例**:
```json
{
  "chunk": [
    {
      "type": "m.room.message",
      "content": {
        "msgtype": "m.text",
        "body": "Hello!"
      },
      "sender": "@alice:example.com",
      "event_id": "$event_id:example.com",
      "origin_server_ts": 1234567890000
    }
  ]
}
```

#### 3.11.8 获取事件

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/event/{event_id}` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "event_id": "$event_id:example.com",
  "room_id": "!room:example.com",
  "sender": "@alice:example.com",
  "type": "m.room.message",
  "content": {},
  "origin_server_ts": 1234567890000,
  "state_key": null
}
```

#### 3.11.9 获取事件上下文

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/context/{event_id}` |
| **方法** | `GET` |
| **认证** | 需要 |

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `limit` | integer | 否 | 数量限制（默认10） |

**响应示例**:
```json
{
  "event": {},
  "events_before": [],
  "events_after": [],
  "state": [],
  "start": "",
  "end": ""
}
```

#### 3.11.10 删除事件

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/redact/{event_id}` |
| **方法** | `PUT` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `reason` | string | 否 | 删除原因 |

**请求示例**:
```json
{
  "reason": "Inappropriate content"
}
```

**响应示例**:
```json
{
  "event_id": "$redaction_id:example.com"
}
```

#### 3.11.11 获取成员历史事件

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/get_membership_events` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `limit` | integer | 否 | 数量限制（默认100） |

**响应示例**:
```json
{
  "events": []
}
```

---

### 3.12 房间目录

#### 3.12.1 获取房间信息

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/directory/room/{room_alias}` |
| **方法** | `GET` |
| **认证** | 不需要 |

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `room_alias` | string | 是 | 房间别名 |

**响应示例**:
```json
{
  "room_id": "!room:example.com"
}
```

#### 3.12.2 设置房间目录

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/directory/room/{room_alias}` |
| **方法** | `PUT` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `room_id` | string | 是 | 房间ID |

**请求示例**:
```json
{
  "room_id": "!room:example.com"
}
```

**响应示例**:
```json
{}
```

#### 3.12.3 删除房间目录

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/directory/room/{room_alias}` |
| **方法** | `DELETE` |
| **认证** | 需要 |

**响应示例**:
```json
{}
```

#### 3.12.4 获取公共房间列表

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/directory/list/public` |
| **方法** | `GET` |
| **认证** | 不需要 |

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `limit` | integer | 否 | 数量限制（默认10） |

**响应示例**:
```json
{
  "chunk": []
}
```

#### 3.12.5 获取房间别名列表

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/directory/room/{room_id}/alias` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "aliases": ["#room:example.com"]
}
```

#### 3.12.6 设置房间别名

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/directory/room/{room_id}/alias/{room_alias}` |
| **方法** | `PUT` |
| **认证** | 需要 |

**响应示例**:
```json
{}
```

#### 3.12.7 删除房间别名

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/directory/room/{room_id}/alias/{room_alias}` |
| **方法** | `DELETE` |
| **认证** | 需要 |

**响应示例**:
```json
{}
```

#### 3.12.8 获取公共房间（v1）

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/publicRooms` |
| **方法** | `GET` |
| **认证** | 不需要 |

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `limit` | integer | 否 | 数量限制（默认10） |
| `since` | string | 否 | 分页令牌 |

**响应示例**:
```json
{
  "chunk": []
}
```

#### 3.12.9 创建公共房间

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/publicRooms` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**: 同创建房间

**响应示例**:
```json
{
  "room_id": "!room:example.com"
}
```

---

### 3.13 事件举报

#### 3.13.1 举报事件

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/report/{event_id}` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `reason` | string | 否 | 举报原因 |
| `score` | integer | 否 | 严重程度分数（-100 到 0，默认-100） |

**请求示例**:
```json
{
  "reason": "Spam",
  "score": -50
}
```

**响应示例**:
```json
{
  "report_id": 123
}
```

#### 3.13.2 更新举报分数

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/report/{event_id}/score` |
| **方法** | `PUT` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `score` | integer | 是 | 新的分数 |

**请求示例**:
```json
{
  "score": -75
}
```

**响应示例**:
```json
{}
```

---

## 4. 管理员 API

> 所有管理员 API 需要管理员认证。

### 4.1 服务器信息

#### 4.1.1 获取服务器版本

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/server_version` |
| **方法** | `GET` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "version": "1.0.0",
  "python_version": "3.9.0"
}
```

#### 4.1.2 获取服务器状态

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/status` |
| **方法** | `GET` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "status": "running",
  "version": "1.0.0",
  "users": 100,
  "rooms": 50,
  "uptime": 0
}
```

#### 4.1.3 获取服务器统计

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/server_stats` |
| **方法** | `GET` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "user_count": 100,
  "room_count": 50,
  "total_message_count": 10000,
  "database_pool_size": 20,
  "cache_enabled": true
}
```

#### 4.1.4 获取服务器配置

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/config` |
| **方法** | `GET` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "server_name": "example.com",
  "version": "1.0.0",
  "registration_enabled": true,
  "guest_registration_enabled": false,
  "password_policy": {
    "enabled": true,
    "minimum_length": 8,
    "require_digit": true,
    "require_lowercase": true,
    "require_uppercase": true,
    "require_symbol": true
  },
  "rate_limiting": {
    "enabled": true,
    "per_second": 10,
    "burst_size": 50
  }
}
```

#### 4.1.5 获取服务器日志

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/logs` |
| **方法** | `GET` |
| **认证** | 管理员 |

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `level` | string | 否 | 日志级别（默认info） |
| `limit` | integer | 否 | 日志条数限制（默认100） |

**响应示例**:
```json
{
  "logs": [
    {
      "timestamp": "2026-02-13T00:00:00Z",
      "level": "info",
      "message": "Server started successfully",
      "module": "synapse::server"
    }
  ],
  "total": 3,
  "level_filter": "info"
}
```

#### 4.1.6 获取用户统计

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/user_stats` |
| **方法** | `GET` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "total_users": 100,
  "active_users": 100,
  "admin_users": 1,
  "deactivated_users": 0,
  "guest_users": 0,
  "average_rooms_per_user": 5.0,
  "user_registration_enabled": true
}
```

#### 4.1.7 获取媒体统计

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/media_stats` |
| **方法** | `GET` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "total_storage_bytes": 1073741824,
  "total_storage_human": "1.00 GB",
  "file_count": 500,
  "media_directory": "/app/data/media",
  "thumbnail_enabled": true,
  "max_upload_size_mb": 50
}
```

#### 4.1.8 获取统计信息

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/statistics` |
| **方法** | `GET` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "users": {
    "total": 100,
    "active": 100
  },
  "rooms": {
    "total": 50
  },
  "daily_active_users": 100,
  "daily_active_rooms": 50,
  "monthly_active_users": 100
}
```

#### 4.1.9 获取后台更新任务

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/background_updates` |
| **方法** | `GET` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "updates": [
    {
      "job_name": "job_name",
      "progress": 50,
      "completed": false
    }
  ],
  "enabled": true
}
```

#### 4.1.10 执行后台更新任务

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/background_updates/{job_name}` |
| **方法** | `POST` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "success": true,
  "job_name": "job_name"
}
```

---

### 4.2 用户管理

#### 4.2.1 获取用户列表 (v1)

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/users` |
| **方法** | `GET` |
| **认证** | 管理员 |

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `limit` | integer | 否 | 数量限制（默认100，最大10000） |
| `offset` | integer | 否 | 偏移量（默认0） |

**响应示例**:
```json
{
  "users": [
    {
      "name": "alice",
      "is_guest": false,
      "admin": false,
      "deactivated": false,
      "displayname": "Alice",
      "avatar_url": "mxc://example.com/avatar",
      "creation_ts": 1234567890,
      "user_type": null
    }
  ],
  "total": 100
}
```

#### 4.2.2 获取用户列表 (v2)

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v2/users` |
| **方法** | `GET` |
| **认证** | 管理员 |

同 v1 接口

#### 4.2.3 获取用户信息 (v1)

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/users/{user_id}` |
| **方法** | `GET` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "name": "alice",
  "is_guest": false,
  "admin": false,
  "deactivated": false,
  "displayname": "Alice",
  "avatar_url": "mxc://example.com/avatar",
  "creation_ts": 1234567890,
  "user_type": null
}
```

#### 4.2.4 获取用户信息 (v2)

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v2/users/{user_id}` |
| **方法** | `GET` |
| **认证** | 管理员 |

同 v1 接口

#### 4.2.5 创建或更新用户 (v2)

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v2/users/{user_id}` |
| **方法** | `PUT` |
| **认证** | 管理员 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `password` | string | 否 | 密码 |
| `displayname` | string | 否 | 显示名称 |
| `admin` | boolean | 否 | 是否为管理员 |
| `deactivated` | boolean | 否 | 是否停用 |

**请求示例**:
```json
{
  "password": "password123",
  "displayname": "Alice",
  "admin": false
}
```

**响应示例**:
```json
{}
```

#### 4.2.6 删除用户

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/users/{user_id}` |
| **方法** | `DELETE` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "user_id": "@alice:example.com",
  "deleted": true
}
```

#### 4.2.7 设置管理员

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/users/{user_id}/admin` |
| **方法** | `PUT` |
| **认证** | 管理员 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `admin` | boolean | 是 | 是否为管理员 |

**请求示例**:
```json
{
  "admin": true
}
```

**响应示例**:
```json
{
  "success": true
}
```

#### 4.2.8 停用用户

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/users/{user_id}/deactivate` |
| **方法** | `POST` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "id_server_unbind_result": "success"
}
```

#### 4.2.9 重置用户密码

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/users/{user_id}/password` |
| **方法** | `POST` |
| **认证** | 管理员 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `new_password` | string | 是 | 新密码 |

**请求示例**:
```json
{
  "new_password": "newpassword123"
}
```

**响应示例**:
```json
{}
```

#### 4.2.10 获取用户房间

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/users/{user_id}/rooms` |
| **方法** | `GET` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "rooms": ["!room1:example.com", "!room2:example.com"]
}
```

#### 4.2.11 以用户身份登录

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/users/{user_id}/login` |
| **方法** | `POST` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "access_token": "syt_token...",
  "device_id": "admin_uuid",
  "user_id": "@alice:example.com"
}
```

#### 4.2.12 登出用户所有设备

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/users/{user_id}/logout` |
| **方法** | `POST` |
| **认证** | 管理员 |

**响应示例**:
```json
{}
```

#### 4.2.13 获取用户设备

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/users/{user_id}/devices` |
| **方法** | `GET` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "devices": [
    {
      "device_id": "DEVICEID",
      "last_seen_ts": 1234567890000
    }
  ],
  "total": 1
}
```

#### 4.2.14 删除用户设备

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/users/{user_id}/devices/{device_id}` |
| **方法** | `DELETE` |
| **认证** | 管理员 |

**响应示例**:
```json
{}
```

#### 4.2.15 获取用户媒体

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/users/{user_id}/media` |
| **方法** | `GET` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "media": [
    {
      "media_id": "media_123",
      "media_type": "image/png",
      "size": 102400,
      "created_ts": 1234567890000
    }
  ],
  "total": 1
}
```

#### 4.2.16 删除用户媒体

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/users/{user_id}/media` |
| **方法** | `DELETE` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "deleted": 5
}
```

---

### 4.3 房间管理

#### 4.3.1 获取房间列表

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/rooms` |
| **方法** | `GET` |
| **认证** | 管理员 |

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `limit` | integer | 否 | 数量限制（默认100，最大10000） |
| `offset` | integer | 否 | 偏移量（默认0） |

**响应示例**:
```json
{
  "rooms": [
    {
      "room_id": "!room:example.com",
      "name": "My Room",
      "topic": "A test room",
      "creator": "@alice:example.com",
      "joined_members": 5,
      "joined_local_members": 5,
      "is_public": true
    }
  ],
  "total": 50
}
```

#### 4.3.2 获取房间信息

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/rooms/{room_id}` |
| **方法** | `GET` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "room_id": "!room:example.com",
  "name": "My Room",
  "topic": "A test room",
  "creator": "@alice:example.com",
  "is_public": true,
  "join_rule": "public"
}
```

#### 4.3.3 删除房间

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/rooms/{room_id}` |
| **方法** | `DELETE` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "room_id": "!room:example.com",
  "deleted": true
}
```

#### 4.3.4 删除房间 (POST)

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/rooms/{room_id}/delete` |
| **方法** | `POST` |
| **认证** | 管理员 |

同 DELETE 接口

#### 4.3.5 获取房间成员

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/rooms/{room_id}/members` |
| **方法** | `GET` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "members": [
    {
      "user_id": "@alice:example.com",
      "membership": "join",
      "joined_ts": 1234567890000
    }
  ],
  "total": 1
}
```

#### 4.3.6 获取房间状态

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/rooms/{room_id}/state` |
| **方法** | `GET` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "state": [
    {
      "event_id": "$event_id:example.com",
      "type": "m.room.name",
      "state_key": "",
      "content": {}
    }
  ]
}
```

#### 4.3.7 获取房间消息

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/rooms/{room_id}/messages` |
| **方法** | `GET` |
| **认证** | 管理员 |

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `limit` | integer | 否 | 数量限制（默认100，最大1000） |

**响应示例**:
```json
{
  "chunk": [
    {
      "event_id": "$event_id:example.com",
      "type": "m.room.message",
      "sender": "@alice:example.com",
      "content": {},
      "origin_server_ts": 1234567890000
    }
  ],
  "start": "",
  "end": ""
}
```

#### 4.3.8 管理员加入房间

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/rooms/{room_id}/join` |
| **方法** | `POST` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "room_id": "!room:example.com",
  "joined": true
}
```

#### 4.3.9 通过别名加入房间

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/join/{room_id_or_alias}` |
| **方法** | `POST` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "room_id": "!room:example.com",
  "joined": true
}
```

#### 4.3.10 封禁房间

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/rooms/{room_id}/block` |
| **方法** | `POST` |
| **认证** | 管理员 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `block` | boolean | 是 | 是否封禁 |

**请求示例**:
```json
{
  "block": true
}
```

**响应示例**:
```json
{
  "block": true
}
```

#### 4.3.11 获取房间封禁状态

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/rooms/{room_id}/block` |
| **方法** | `GET` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "block": false,
  "room_id": "!room:example.com"
}
```

#### 4.3.12 设置房间管理员

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/rooms/{room_id}/make_admin` |
| **方法** | `POST` |
| **认证** | 管理员 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `user_id` | string | 是 | 用户ID |

**请求示例**:
```json
{
  "user_id": "@alice:example.com"
}
```

**响应示例**:
```json
{}
```

#### 4.3.13 清理历史

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/purge_history` |
| **方法** | `POST` |
| **认证** | 管理员 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `room_id` | string | 是 | 房间ID |
| `purge_up_to_ts` | integer | 否 | 清理此时间戳之前的事件（默认30天前） |

**请求示例**:
```json
{
  "room_id": "!room:example.com",
  "purge_up_to_ts": 1234567890000
}
```

**响应示例**:
```json
{
  "success": true,
  "deleted_events": 100
}
```

#### 4.3.14 关闭房间

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/shutdown_room` |
| **方法** | `POST` |
| **认证** | 管理员 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `room_id` | string | 是 | 房间ID |

**请求示例**:
```json
{
  "room_id": "!room:example.com"
}
```

**响应示例**:
```json
{
  "kicked_users": [],
  "failed_to_kick_users": [],
  "closed_room": true
}
```

#### 4.3.15 清理媒体缓存

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/purge_media_cache` |
| **方法** | `POST` |
| **认证** | 管理员 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `older_than` | integer | 否 | 清理此时间戳之前的媒体（默认30天前） |

**请求示例**:
```json
{
  "older_than": 1234567890
}
```

**响应示例**:
```json
{
  "deleted": 10
}
```

---

### 4.4 安全相关

#### 4.4.1 获取安全事件

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/security/events` |
| **方法** | `GET` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "events": [
    {
      "id": 1,
      "event_type": "admin_action:block_ip",
      "user_id": "@admin:example.com",
      "ip_address": "192.168.1.1",
      "user_agent": null,
      "details": null,
      "created_at": 1234567890
    }
  ],
  "total": 1
}
```

#### 4.4.2 获取 IP 阻止列表

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/security/ip/blocks` |
| **方法** | `GET` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "blocked_ips": [
    {
      "ip_address": "192.168.1.1/32",
      "reason": "Spam",
      "blocked_at": 1234567890,
      "expires_at": null
    }
  ],
  "total": 1
}
```

#### 4.4.3 阻止 IP

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/security/ip/block` |
| **方法** | `POST` |
| **认证** | 管理员 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `ip_address` | string | 是 | IP 地址或 CIDR |
| `reason` | string | 否 | 原因 |
| `expires_at` | string | 否 | 过期时间（RFC3339格式） |

**请求示例**:
```json
{
  "ip_address": "192.168.1.1",
  "reason": "Spam",
  "expires_at": "2026-03-13T00:00:00Z"
}
```

**响应示例**:
```json
{
  "success": true,
  "ip_address": "192.168.1.1"
}
```

#### 4.4.4 解除 IP 阻止

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/security/ip/unblock` |
| **方法** | `POST` |
| **认证** | 管理员 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `ip_address` | string | 是 | IP 地址或 CIDR |

**请求示例**:
```json
{
  "ip_address": "192.168.1.1"
}
```

**响应示例**:
```json
{
  "success": true,
  "ip_address": "192.168.1.1",
  "message": "IP unblocked"
}
```

#### 4.4.5 获取 IP 信誉

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/security/ip/reputation/{ip}` |
| **方法** | `GET` |
| **认证** | 管理员 |

**响应示例**:
```json
{
  "ip_address": "192.168.1.1",
  "score": 50,
  "last_seen_at": 1234567890,
  "updated_at": 1234567890,
  "details": null
}
```

---

### 4.5 管理员注册

#### 4.5.1 获取注册 nonce

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/register/nonce` |
| **方法** | `GET` |
| **认证** | 不需要（但需要有效的客户端IP） |

**响应示例**:
```json
{
  "nonce": "abc123",
  "expires_at": 1234567890
}
```

#### 4.5.2 管理员注册

| 属性 | 值 |
|------|-----|
| **端点** | `/_synapse/admin/v1/register` |
| **方法** | `POST` |
| **认证** | 不需要（需要HMAC签名） |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `username` | string | 是 | 用户名 |
| `password` | string | 是 | 密码 |
| `nonce` | string | 是 | 注册 nonce |
| `mac` | string | 是 | HMAC 签名 |
| `admin` | boolean | 否 | 是否为管理员 |

**请求示例**:
```json
{
  "username": "newuser",
  "password": "password123",
  "nonce": "abc123",
  "mac": "signature",
  "admin": false
}
```

**响应示例**:
```json
{
  "user_id": "@newuser:example.com",
  "access_token": "syt_token...",
  "device_id": "DEVICEID"
}
```

---

## 5. 联邦通信 API

### 5.1 密钥与发现（无需签名）

#### 5.1.1 获取服务器密钥

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v2/server` |
| **方法** | `GET` |
| **认证** | 不需要 |

**响应示例**:
```json
{
  "server_name": "example.com",
  "verify_keys": {
    "ed25519:1": {
      "key": "base64_encoded_key"
    }
  },
  "old_verify_keys": {},
  "valid_until_ts": 1234567890000
}
```

#### 5.1.2 获取服务器密钥 (v2)

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/key/v2/server` |
| **方法** | `GET` |
| **认证** | 不需要 |

同上

#### 5.1.3 查询服务器密钥

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v2/query/{server_name}/{key_id}` |
| **方法** | `GET` |
| **认证** | 不需要 |

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `server_name` | string | 是 | 服务器名称 |
| `key_id` | string | 是 | 密钥ID |

#### 5.1.4 查询服务器密钥 (v2)

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/key/v2/query/{server_name}/{key_id}` |
| **方法** | `GET` |
| **认证** | 不需要 |

同上

#### 5.1.5 获取联邦版本

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/version` |
| **方法** | `GET` |
| **认证** | 不需要 |

**响应示例**:
```json
{
  "version": "86400000",
  "server": {
    "name": "Synapse Rust",
    "version": "0.1.0"
  }
}
```

#### 5.1.6 联邦发现

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1` |
| **方法** | `GET` |
| **认证** | 不需要 |

**响应示例**:
```json
{
  "version": "0.1.0",
  "server_name": "example.com",
  "capabilities": {
    "m.change_password": true,
    "m.room_versions": {
      "1": {
        "status": "stable"
      }
    }
  }
}
```

#### 5.1.7 获取公共房间

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/publicRooms` |
| **方法** | `GET` |
| **认证** | 不需要 |

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `limit` | integer | 否 | 数量限制（默认10） |

**响应示例**:
```json
{
  "chunk": [],
  "next_batch": null
}
```

#### 5.1.8 查询目标

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/query/destination` |
| **方法** | `GET` |
| **认证** | 不需要 |

**响应示例**:
```json
{
  "destination": "example.com",
  "host": "localhost",
  "port": 8008,
  "tls": false,
  "ts": 1234567890000
}
```

#### 5.1.9 获取房间事件

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/room/{room_id}/{event_id}` |
| **方法** | `GET` |
| **认证** | 不需要 |

**响应示例**:
```json
{
  "event_id": "$event_id:example.com",
  "type": "m.room.message",
  "sender": "@alice:example.com",
  "content": {},
  "state_key": null,
  "origin_server_ts": 1234567890000,
  "room_id": "!room:example.com"
}
```

---

### 5.2 房间操作（需要签名）

#### 5.2.1 获取房间成员

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/members/{room_id}` |
| **方法** | `GET` |
| **认证** | 联邦签名 |

**响应示例**:
```json
{
  "members": [
    {
      "room_id": "!room:example.com",
      "user_id": "@alice:example.com",
      "membership": "join",
      "display_name": "Alice",
      "avatar_url": "mxc://example.com/avatar"
    }
  ],
  "room_id": "!room:example.com",
  "offset": 0,
  "total": 1
}
```

#### 5.2.2 获取已加入成员

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/members/{room_id}/joined` |
| **方法** | `GET` |
| **认证** | 联邦签名 |

**响应示例**:
```json
{
  "joined": [
    {
      "room_id": "!room:example.com",
      "user_id": "@alice:example.com",
      "membership": "join"
    }
  ],
  "room_id": "!room:example.com"
}
```

#### 5.2.3 获取用户设备

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/user/devices/{user_id}` |
| **方法** | `GET` |
| **认证** | 联邦签名 |

**响应示例**:
```json
{
  "user_id": "@alice:example.com",
  "devices": [
    {
      "device_id": "DEVICEID",
      "user_id": "@alice:example.com",
      "keys": {},
      "device_display_name": "My Device",
      "last_seen_ts": 1234567890000,
      "last_seen_ip": "127.0.0.1"
    }
  ]
}
```

#### 5.2.4 获取房间认证链

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/room_auth/{room_id}` |
| **方法** | `GET` |
| **认证** | 联邦签名 |

**响应示例**:
```json
{
  "room_id": "!room:example.com",
  "auth_chain": []
}
```

#### 5.2.5 敲门请求

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/knock/{room_id}/{user_id}` |
| **方法** | `GET` |
| **认证** | 联邦签名 |

**响应示例**:
```json
{
  "event_id": "$event_id:example.com",
  "room_id": "!room:example.com",
  "state": "knocking"
}
```

#### 5.2.6 第三方邀请

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/thirdparty/invite` |
| **方法** | `POST` |
| **认证** | 联邦签名 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `room_id` | string | 是 | 房间ID |
| `invitee` | string | 是 | 被邀请者 |
| `sender` | string | 是 | 邀请者 |

**响应示例**:
```json
{
  "event_id": "$event_id:example.com",
  "room_id": "!room:example.com",
  "state": "invited"
}
```

#### 5.2.7 获取加入规则

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/get_joining_rules/{room_id}` |
| **方法** | `GET` |
| **认证** | 联邦签名 |

**响应示例**:
```json
{
  "room_id": "!room:example.com",
  "join_rule": "invite",
  "allow": []
}
```

#### 5.2.8 邀请 (v2)

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v2/invite/{room_id}/{event_id}` |
| **方法** | `PUT` |
| **认证** | 联邦签名 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `origin` | string | 是 | 发送方服务器 |
| `event` | object | 是 | 邀请事件 |

**响应示例**:
```json
{
  "event_id": "$event_id:example.com"
}
```

#### 5.2.9 发送事务

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/send/{txn_id}` |
| **方法** | `PUT` |
| **认证** | 联邦签名 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `origin` | string | 是 | 发送方服务器 |
| `pdus` | array | 是 | PDU 列表 |

**响应示例**:
```json
{
  "results": [
    {
      "event_id": "$event_id:example.com",
      "success": true
    }
  ]
}
```

#### 5.2.10 生成加入模板

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/make_join/{room_id}/{user_id}` |
| **方法** | `GET` |
| **认证** | 联邦签名 |

**响应示例**:
```json
{
  "room_version": "1",
  "auth_events": [],
  "event": {
    "type": "m.room.member",
    "content": {
      "membership": "join"
    },
    "sender": "@alice:example.com",
    "state_key": "@alice:example.com"
  }
}
```

#### 5.2.11 生成离开模板

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/make_leave/{room_id}/{user_id}` |
| **方法** | `GET` |
| **认证** | 联邦签名 |

**响应示例**:
```json
{
  "room_version": "1",
  "auth_events": [],
  "event": {
    "type": "m.room.member",
    "content": {
      "membership": "leave"
    },
    "sender": "@alice:example.com",
    "state_key": "@alice:example.com"
  }
}
```

#### 5.2.12 发送加入

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/send_join/{room_id}/{event_id}` |
| **方法** | `PUT` |
| **认证** | 联邦签名 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `origin` | string | 是 | 发送方服务器 |
| `event` | object | 是 | 加入事件 |

**响应示例**:
```json
{
  "event_id": "$event_id:example.com"
}
```

#### 5.2.13 发送离开

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/send_leave/{room_id}/{event_id}` |
| **方法** | `PUT` |
| **认证** | 联邦签名 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `origin` | string | 是 | 发送方服务器 |
| `event` | object | 是 | 离开事件 |

**响应示例**:
```json
{
  "event_id": "$event_id:example.com"
}
```

#### 5.2.14 邀请 (v1)

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/invite/{room_id}/{event_id}` |
| **方法** | `PUT` |
| **认证** | 联邦签名 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `origin` | string | 是 | 发送方服务器 |

**响应示例**:
```json
{
  "event_id": "$event_id:example.com"
}
```

#### 5.2.15 获取缺失事件

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/get_missing_events/{room_id}` |
| **方法** | `POST` |
| **认证** | 联邦签名 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `earliest_events` | array | 是 | 最早事件列表 |
| `latest_events` | array | 是 | 最新事件列表 |
| `limit` | integer | 否 | 数量限制（默认10） |

**响应示例**:
```json
{
  "events": []
}
```

#### 5.2.16 获取事件认证链

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/get_event_auth/{room_id}/{event_id}` |
| **方法** | `GET` |
| **认证** | 联邦签名 |

**响应示例**:
```json
{
  "auth_chain": []
}
```

#### 5.2.17 获取房间状态

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/state/{room_id}` |
| **方法** | `GET` |
| **认证** | 联邦签名 |

**响应示例**:
```json
{
  "state": []
}
```

#### 5.2.18 获取事件

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/event/{event_id}` |
| **方法** | `GET` |
| **认证** | 联邦签名 |

**响应示例**:
```json
{
  "event_id": "$event_id:example.com",
  "type": "m.room.message",
  "sender": "@alice:example.com",
  "content": {},
  "state_key": null,
  "origin_server_ts": 1234567890000,
  "room_id": "!room:example.com"
}
```

#### 5.2.19 获取状态ID列表

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/state_ids/{room_id}` |
| **方法** | `GET` |
| **认证** | 联邦签名 |

**响应示例**:
```json
{
  "state_ids": ["$event_id1:example.com", "$event_id2:example.com"]
}
```

#### 5.2.20 查询房间目录

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/query/directory/room/{room_id}` |
| **方法** | `GET` |
| **认证** | 联邦签名 |

**响应示例**:
```json
{
  "room_id": "!room:example.com",
  "servers": ["example.com"],
  "name": "My Room",
  "topic": "A test room",
  "guest_can_join": true,
  "world_readable": true
}
```

#### 5.2.21 查询用户资料

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/query/profile/{user_id}` |
| **方法** | `GET` |
| **认证** | 联邦签名 |

**响应示例**:
```json
{
  "displayname": "Alice",
  "avatar_url": "mxc://example.com/avatar"
}
```

#### 5.2.22 回填

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/backfill/{room_id}` |
| **方法** | `GET` |
| **认证** | 联邦签名 |

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `limit` | integer | 否 | 数量限制（默认10） |
| `v` | array | 是 | 事件ID列表 |

**响应示例**:
```json
{
  "origin": "example.com",
  "pdus": [],
  "limit": 10
}
```

#### 5.2.23 声明密钥

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/keys/claim` |
| **方法** | `POST` |
| **认证** | 联邦签名 |

**请求体**: 密钥声明请求

**响应示例**:
```json
{
  "one_time_keys": {},
  "failures": {}
}
```

#### 5.2.24 上传密钥

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/keys/upload` |
| **方法** | `POST` |
| **认证** | 联邦签名 |

**请求体**: 密钥上传请求

**响应示例**:
```json
{
  "one_time_key_counts": {}
}
```

#### 5.2.25 克隆密钥

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v2/key/clone` |
| **方法** | `POST` |
| **认证** | 联邦签名 |

**响应示例**:
```json
{
  "success": true
}
```

#### 5.2.26 查询用户密钥

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v2/user/keys/query` |
| **方法** | `POST` |
| **认证** | 联邦签名 |

**响应示例**:
```json
{
  "device_keys": {}
}
```

#### 5.2.27 交换第三方邀请

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/exchange_third_party_invite/{room_id}` |
| **方法** | `PUT` |
| **认证** | 联邦签名 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `sender` | string | 是 | 发送者 |
| `state_key` | string | 是 | 状态键 |
| `content` | object | 否 | 内容 |

**响应示例**:
```json
{
  "event_id": "$event_id:example.com"
}
```

#### 5.2.28 绑定第三方邀请

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/on_bind_third_party_invite/{room_id}` |
| **方法** | `PUT` |
| **认证** | 联邦签名 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `event_id` | string | 是 | 事件ID |
| `sender` | string | 是 | 发送者 |
| `state_key` | string | 是 | 状态键 |
| `content` | object | 否 | 内容 |

**响应示例**:
```json
{}
```

#### 5.2.29 3PID 绑定

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/3pid/onbind` |
| **方法** | `POST` |
| **认证** | 联邦签名 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `mxid` | string | 是 | Matrix ID |
| `medium` | string | 否 | 媒介（默认email） |
| `address` | string | 否 | 地址 |

**响应示例**:
```json
{}
```

#### 5.2.30 发送设备到设备消息

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/federation/v1/sendToDevice/{txn_id}` |
| **方法** | `PUT` |
| **认证** | 联邦签名 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `messages` | object | 是 | 消息内容 |
| `type` | string | 否 | 事件类型（默认m.room.message） |

**响应示例**:
```json
{}
```

---

## 6. 好友系统 API

> 好友系统基于 Matrix 房间实现。

### 6.1 好友管理

#### 6.1.1 获取好友列表

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v1/friends` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "friends": [
    {
      "user_id": "@bob:example.com",
      "display_name": "Bob",
      "avatar_url": "mxc://example.com/avatar",
      "since": 1234567890,
      "status": "online",
      "note": "Best friend"
    }
  ],
  "total": 1
}
```

#### 6.1.2 发送好友请求

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v1/friends/request` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `user_id` | string | 是 | 目标用户ID |
| `message` | string | 否 | 请求消息 |

**请求示例**:
```json
{
  "user_id": "@bob:example.com",
  "message": "Hi, let's be friends!"
}
```

**响应示例**:
```json
{
  "room_id": "!dm:example.com",
  "status": "pending"
}
```

#### 6.1.3 接受好友请求

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v1/friends/request/{user_id}/accept` |
| **方法** | `POST` |
| **认证** | 需要 |

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `user_id` | string | 是 | 请求者用户ID |

**响应示例**:
```json
{
  "room_id": "!dm:example.com",
  "status": "accepted"
}
```

#### 6.1.4 拒绝好友请求

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v1/friends/request/{user_id}/reject` |
| **方法** | `POST` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "status": "rejected"
}
```

#### 6.1.5 取消好友请求

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v1/friends/request/{user_id}/cancel` |
| **方法** | `POST` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "status": "cancelled"
}
```

#### 6.1.6 获取收到的请求

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v1/friends/requests/incoming` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "requests": [
    {
      "user_id": "@bob:example.com",
      "display_name": "Bob",
      "message": "Hi!",
      "timestamp": 1234567890000,
      "status": "pending"
    }
  ]
}
```

#### 6.1.7 获取发出的请求

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v1/friends/requests/outgoing` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "requests": [
    {
      "user_id": "@charlie:example.com",
      "timestamp": 1234567890000,
      "status": "pending"
    }
  ]
}
```

#### 6.1.8 获取被阻止的好友

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v1/friends/blocked` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "blocked": [
    {
      "user_id": "@bob:example.com",
      "blocked_at": 1234567890000
    }
  ],
  "total": 1
}
```

#### 6.1.9 阻止好友

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v1/friends/{user_id}/block` |
| **方法** | `POST` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "status": "blocked"
}
```

#### 6.1.10 解除阻止好友

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v1/friends/{user_id}/unblock` |
| **方法** | `POST` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "status": "unblocked"
}
```

#### 6.1.11 删除好友

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v1/friends/{user_id}` |
| **方法** | `DELETE` |
| **认证** | 需要 |

**响应示例**:
```json
{}
```

#### 6.1.12 更新好友备注

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v1/friends/{user_id}/note` |
| **方法** | `PUT` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `note` | string | 是 | 备注内容（最大1000字符） |

**请求示例**:
```json
{
  "note": "Met at conference"
}
```

**响应示例**:
```json
{}
```

#### 6.1.13 更新好友状态

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v1/friends/{user_id}/status` |
| **方法** | `PUT` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `status` | string | 是 | 状态：`favorite`、`normal`、`blocked`、`hidden` |

**请求示例**:
```json
{
  "status": "favorite"
}
```

**响应示例**:
```json
{}
```

#### 6.1.14 获取好友信息

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v1/friends/{user_id}/info` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "user_id": "@bob:example.com",
  "display_name": "Bob",
  "avatar_url": "mxc://example.com/avatar",
  "since": 1234567890,
  "status": "normal",
  "note": "Best friend",
  "dm_room_id": "!dm:example.com"
}
```

---

## 7. 端到端加密 API

### 7.1 上传设备密钥

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/keys/upload` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `device_keys` | object | 否 | 设备密钥 |
| `one_time_keys` | object | 否 | 一次性密钥 |

**请求示例**:
```json
{
  "device_keys": {
    "user_id": "@alice:example.com",
    "device_id": "DEVICEID",
    "algorithms": ["m.olm.v1.curve25519-aes-sha2"],
    "keys": {
      "curve25519:DEVICEID": "base64_key",
      "ed25519:DEVICEID": "base64_key"
    },
    "signatures": {
      "@alice:example.com": {
        "ed25519:DEVICEID": "signature"
      }
    }
  },
  "one_time_keys": {
    "curve25519:ABCDEF": {
      "key": "base64_key"
    }
  }
}
```

**响应示例**:
```json
{
  "one_time_key_counts": {
    "curve25519": 50,
    "signed_curve25519": 50
  }
}
```

### 7.2 查询设备密钥

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/keys/query` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `device_keys` | object | 是 | 要查询的用户和设备 |

**请求示例**:
```json
{
  "device_keys": {
    "@bob:example.com": []
  }
}
```

**响应示例**:
```json
{
  "device_keys": {
    "@bob:example.com": {
      "DEVICEID": {
        "user_id": "@bob:example.com",
        "device_id": "DEVICEID",
        "algorithms": ["m.olm.v1.curve25519-aes-sha2"],
        "keys": {},
        "signatures": {}
      }
    }
  },
  "failures": {}
}
```

### 7.3 声明一次性密钥

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/keys/claim` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `one_time_keys` | object | 是 | 要声明的一次性密钥 |

**请求示例**:
```json
{
  "one_time_keys": {
    "@bob:example.com": {
      "DEVICEID": "signed_curve25519"
    }
  }
}
```

**响应示例**:
```json
{
  "one_time_keys": {
    "@bob:example.com": {
      "DEVICEID": {
        "signed_curve25519:ABCDEF": {
          "key": "base64_key",
          "signatures": {}
        }
      }
    }
  },
  "failures": {}
}
```

### 7.4 获取密钥变更通知

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/keys/changes` |
| **方法** | `GET` |
| **认证** | 需要 |

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `from` | string | 否 | 起始令牌（默认0） |
| `to` | string | 否 | 结束令牌 |

**响应示例**:
```json
{
  "changed": ["@bob:example.com"],
  "left": []
}
```

### 7.5 获取房间密钥分发

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/rooms/{room_id}/keys/distribution` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "room_id": "!room:example.com",
  "algorithm": "m.megolm.v1.aes-sha2",
  "session_id": "session_id",
  "session_key": "base64_session_key"
}
```

### 7.6 发送设备到设备消息

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/sendToDevice/{event_type}/{transaction_id}` |
| **方法** | `PUT` |
| **认证** | 需要 |

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `event_type` | string | 是 | 事件类型 |
| `transaction_id` | string | 是 | 事务ID |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `messages` | object | 是 | 消息内容 |

**请求示例**:
```json
{
  "messages": {
    "@bob:example.com": {
      "DEVICEID": {
        "algorithm": "m.megolm.v1.aes-sha2",
        "sender_key": "sender_curve25519_key",
        "session_id": "session_id",
        "session_key": "session_key"
      }
    }
  }
}
```

**响应示例**:
```json
{}
```

### 7.7 上传签名

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/unstable/keys/signatures/upload` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `signatures` | object | 是 | 签名数据 |

**响应示例**:
```json
{}
```

### 7.8 上传设备签名密钥

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/keys/device_signing/upload` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `master_key` | object | 否 | 主密钥 |
| `self_signing_key` | object | 否 | 自签名密钥 |
| `user_signing_key` | object | 否 | 用户签名密钥 |

**请求示例**:
```json
{
  "master_key": {},
  "self_signing_key": {},
  "user_signing_key": {}
}
```

**响应示例**:
```json
{}
```

---

## 8. 媒体文件 API

### 8.1 上传媒体 (v3)

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/media/v3/upload` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `content` | array/string | 是 | 文件内容（字节数组或 Base64） |
| `content_type` | string | 否 | MIME 类型（默认application/octet-stream） |
| `filename` | string | 否 | 文件名 |

**请求示例**:
```json
{
  "content": "base64_encoded_content",
  "content_type": "image/png",
  "filename": "avatar.png"
}
```

**响应示例**:
```json
{
  "content_uri": "mxc://example.com/media_id"
}
```

### 8.2 上传媒体 (v1)

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/media/v1/upload` |
| **方法** | `POST` |
| **认证** | 需要 |

同 v3 接口

### 8.3 上传媒体（指定服务器和ID）

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/media/v3/upload/{server_name}/{media_id}` |
| **方法** | `POST` |
| **认证** | 需要 |

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `server_name` | string | 是 | 服务器名称 |
| `media_id` | string | 是 | 媒体ID |

同 v3 接口请求体

### 8.4 下载媒体 (v3)

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/media/v3/download/{server_name}/{media_id}` |
| **方法** | `GET` |
| **认证** | 不需要 |

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `server_name` | string | 是 | 服务器名称 |
| `media_id` | string | 是 | 媒体ID |

**响应**: 二进制文件内容

**响应头**:

| 响应头 | 说明 |
|--------|------|
| `Content-Type` | MIME 类型 |
| `Content-Length` | 文件大小 |

### 8.5 下载媒体（带文件名）

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/media/v3/download/{server_name}/{media_id}/{filename}` |
| **方法** | `GET` |
| **认证** | 不需要 |

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `server_name` | string | 是 | 服务器名称 |
| `media_id` | string | 是 | 媒体ID |
| `filename` | string | 是 | 文件名 |

**响应**: 二进制文件内容

**响应头**: 包含 `Content-Disposition: attachment; filename="filename"`

### 8.6 下载媒体 (v1)

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/media/v1/download/{server_name}/{media_id}` |
| **方法** | `GET` |
| **认证** | 不需要 |

同 v3 接口

### 8.7 下载媒体 (r1)

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/media/r1/download/{server_name}/{media_id}` |
| **方法** | `GET` |
| **认证** | 不需要 |

同 v3 接口

### 8.8 获取缩略图

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/media/v3/thumbnail/{server_name}/{media_id}` |
| **方法** | `GET` |
| **认证** | 不需要 |

**请求参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `width` | integer | 否 | 宽度（默认800） |
| `height` | integer | 否 | 高度（默认600） |
| `method` | string | 否 | 缩放方式：`scale`、`crop`（默认scale） |

**响应**: 缩略图二进制内容

### 8.9 获取媒体配置

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/media/v1/config` |
| **方法** | `GET` |
| **认证** | 不需要 |

**响应示例**:
```json
{
  "m.upload.size": 52428800
}
```

---

## 9. 语音消息 API

### 9.1 获取语音配置

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/voice/config` |
| **方法** | `GET` |
| **认证** | 不需要 |

**响应示例**:
```json
{
  "supported_formats": ["audio/ogg", "audio/mpeg", "audio/wav"],
  "max_size_bytes": 104857600,
  "max_duration_ms": 600000,
  "default_sample_rate": 48000,
  "default_channels": 2
}
```

### 9.2 上传语音消息

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/voice/upload` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `content` | string | 是 | Base64 编码的音频内容（最大10MB） |
| `content_type` | string | 否 | MIME 类型（默认audio/ogg） |
| `duration_ms` | integer | 是 | 时长（毫秒，必须大于0） |
| `room_id` | string | 否 | 房间ID |
| `session_id` | string | 否 | 会话ID |

**请求示例**:
```json
{
  "content": "base64_encoded_audio",
  "content_type": "audio/ogg",
  "duration_ms": 5000,
  "room_id": "!room:example.com"
}
```

**响应示例**:
```json
{
  "message_id": "msg_123",
  "content_uri": "mxc://example.com/voice_123",
  "duration_ms": 5000
}
```

### 9.3 获取语音消息

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/voice/{message_id}` |
| **方法** | `GET` |
| **认证** | 不需要 |

**响应示例**:
```json
{
  "message_id": "msg_123",
  "content": "base64_encoded_audio",
  "content_type": "audio/ogg",
  "size": 102400
}
```

### 9.4 删除语音消息

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/voice/{message_id}` |
| **方法** | `DELETE` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "deleted": true,
  "message_id": "msg_123"
}
```

### 9.5 获取用户语音消息

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/voice/user/{user_id}` |
| **方法** | `GET` |
| **认证** | 不需要 |

**响应示例**:
```json
{
  "messages": [
    {
      "message_id": "msg_123",
      "duration_ms": 5000,
      "created_at": 1234567890000
    }
  ]
}
```

### 9.6 获取房间语音消息

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/voice/room/{room_id}` |
| **方法** | `GET` |
| **认证** | 不需要 |

**响应示例**:
```json
{
  "messages": [
    {
      "message_id": "msg_123",
      "user_id": "@alice:example.com",
      "duration_ms": 5000
    }
  ]
}
```

### 9.7 获取用户语音统计

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/voice/user/{user_id}/stats` |
| **方法** | `GET` |
| **认证** | 不需要 |

**响应示例**:
```json
{
  "user_id": "@alice:example.com",
  "total_messages": 10,
  "total_duration_ms": 50000,
  "total_size_bytes": 1024000
}
```

### 9.8 获取当前用户语音统计

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/voice/stats` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "user_id": "@alice:example.com",
  "total_messages": 10,
  "total_duration_ms": 50000,
  "total_size_bytes": 1024000
}
```

### 9.9 语音格式转换

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/voice/convert` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `message_id` | string | 是 | 消息ID（最大100字符） |
| `target_format` | string | 是 | 目标格式（如audio/mpeg） |
| `quality` | integer | 否 | 质量（32-320 kbps，默认128） |
| `bitrate` | integer | 否 | 比特率（64000-320000 bps，默认128000） |

**请求示例**:
```json
{
  "message_id": "msg_123",
  "target_format": "audio/mpeg",
  "quality": 128,
  "bitrate": 128000
}
```

**响应示例**:
```json
{
  "status": "success",
  "message": "Conversion simulation successful. (Backend FFmpeg not connected)",
  "message_id": "msg_123",
  "target_format": "audio/mpeg",
  "quality": 128,
  "bitrate": 128000,
  "converted_content": null
}
```

### 9.10 语音优化

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/voice/optimize` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `message_id` | string | 是 | 消息ID（最大100字符） |
| `target_size_kb` | integer | 否 | 目标大小（10-10000 KB，默认500） |
| `preserve_quality` | boolean | 否 | 是否保持质量（默认true） |
| `remove_silence` | boolean | 否 | 是否移除静音（默认false） |
| `normalize_volume` | boolean | 否 | 是否标准化音量（默认true） |

**请求示例**:
```json
{
  "message_id": "msg_123",
  "target_size_kb": 500,
  "preserve_quality": true,
  "remove_silence": false,
  "normalize_volume": true
}
```

**响应示例**:
```json
{
  "status": "success",
  "message": "Optimization simulation successful. (Backend FFmpeg not connected)",
  "message_id": "msg_123",
  "target_size_kb": 500,
  "preserve_quality": true,
  "remove_silence": false,
  "normalize_volume": true,
  "optimized_content": null
}
```

---

## 10. VoIP API

### 10.1 获取 TURN 服务器

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v3/voip/turnServer` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "username": "user:1234567890",
  "password": "credential",
  "uris": [
    "turn:turn.example.com:3478?transport=udp",
    "turn:turn.example.com:3478?transport=tcp"
  ],
  "ttl": 86400
}
```

### 10.2 获取 VoIP 配置

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v3/voip/config` |
| **方法** | `GET` |
| **认证** | 不需要 |

**响应示例**:
```json
{
  "turn_servers": [
    {
      "username": "user",
      "password": "pass",
      "uris": ["turn:turn.example.com:3478"],
      "ttl": 86400
    }
  ],
  "stun_servers": ["stun:stun.example.com:3478"]
}
```

### 10.3 获取访客 TURN 凭证

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v3/voip/turnServer/guest` |
| **方法** | `GET` |
| **认证** | 不需要 |

**响应示例**:
```json
{
  "username": "guest:1234567890",
  "password": "guest_credential",
  "uris": ["turn:turn.example.com:3478"],
  "ttl": 86400
}
```

---

## 11. 密钥备份 API

### 11.1 获取所有备份版本

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/room_keys/version` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "versions": [
    {
      "algorithm": "m.megolm.v1.aes-sha2",
      "auth_data": {},
      "version": "1"
    }
  ]
}
```

### 11.2 创建备份版本

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/room_keys/version` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `algorithm` | string | 否 | 算法（最大255字符，默认m.megolm.v1.aes-sha2） |
| `auth_data` | object | 否 | 认证数据 |

**请求示例**:
```json
{
  "algorithm": "m.megolm.v1.aes-sha2",
  "auth_data": {
    "public_key": "base64_public_key"
  }
}
```

**响应示例**:
```json
{
  "version": "1"
}
```

### 11.3 获取特定备份版本

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/room_keys/version/{version}` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "algorithm": "m.megolm.v1.aes-sha2",
  "auth_data": {},
  "version": "1"
}
```

### 11.4 更新备份版本

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/room_keys/version/{version}` |
| **方法** | `PUT` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `auth_data` | object | 否 | 认证数据 |

**请求示例**:
```json
{
  "auth_data": {
    "public_key": "new_base64_public_key"
  }
}
```

**响应示例**:
```json
{
  "version": "1"
}
```

### 11.5 删除备份版本

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/room_keys/version/{version}` |
| **方法** | `DELETE` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "deleted": true,
  "version": "1"
}
```

### 11.6 获取房间密钥

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/room_keys/{version}` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "rooms": {
    "!room:example.com": {
      "sessions": {
        "session_id": {
          "first_message_index": 0,
          "forwarded_count": 0,
          "is_verified": true,
          "session_data": {}
        }
      }
    }
  },
  "etag": "1_1234567890"
}
```

### 11.7 上传房间密钥

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/room_keys/{version}` |
| **方法** | `PUT` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `room_id` | string | 否 | 房间ID（最大255字符） |
| `sessions` | array | 否 | 会话列表 |

**请求示例**:
```json
{
  "room_id": "!room:example.com",
  "sessions": [
    {
      "session_id": "session_id",
      "session_data": {}
    }
  ]
}
```

**响应示例**:
```json
{
  "count": 1,
  "etag": "1_1234567890"
}
```

### 11.8 批量上传房间密钥

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/room_keys/{version}/keys` |
| **方法** | `POST` |
| **认证** | 需要 |

**请求体**: 按房间分组的密钥数据

**响应示例**:
```json
{
  "count": 10,
  "etag": "1_1234567890"
}
```

### 11.9 获取房间密钥（按房间ID）

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/room_keys/{version}/keys/{room_id}` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "rooms": {
    "!room:example.com": {
      "sessions": {}
    }
  }
}
```

### 11.10 获取特定房间密钥

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/r0/room_keys/{version}/keys/{room_id}/{session_id}` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "room_id": "!room:example.com",
  "session_id": "session_id",
  "first_message_index": 0,
  "forwarded_count": 0,
  "is_verified": true,
  "session_data": {}
}
```

---

## 12. 外部服务 API

### 12.1 获取外部服务列表

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v3/users/me/external-services` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "services": [
    {
      "service_type": "openai",
      "endpoint": "https://api.openai.com",
      "has_api_key": true,
      "status": "active"
    }
  ]
}
```

### 12.2 获取外部服务配置

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v3/users/me/external-services/{service_type}` |
| **方法** | `GET` |
| **认证** | 需要 |

**路径参数**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `service_type` | string | 是 | 服务类型（最大50字符） |

**有效服务类型**: `trendradar`, `openclaw`, `openai`, `claude`, `deepseek`, `anthropic`, `gemini`, `custom` 或以 `custom_` 开头

**响应示例**:
```json
{
  "endpoint": "https://api.openai.com",
  "has_api_key": true,
  "config": {},
  "status": "active"
}
```

### 12.3 保存外部服务配置

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v3/users/me/external-services/{service_type}` |
| **方法** | `PUT` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `endpoint` | string | 是 | 服务端点URL（最大500字符，必须以http://或https://开头） |
| `api_key` | string | 是 | API密钥（最大1024字符） |
| `config` | object | 否 | 配置数据 |

**请求示例**:
```json
{
  "endpoint": "https://api.openai.com",
  "api_key": "sk-xxx",
  "config": {}
}
```

**响应**: HTTP 200 OK

### 12.4 删除外部服务配置

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v3/users/me/external-services/{service_type}` |
| **方法** | `DELETE` |
| **认证** | 需要 |

**响应**: HTTP 204 No Content

### 12.5 获取外部服务凭证

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v3/users/me/external-services/{service_type}/credentials` |
| **方法** | `GET` |
| **认证** | 需要 |

**响应示例**:
```json
{
  "endpoint": "https://api.openai.com",
  "api_key": "sk-xxx",
  "config": {}
}
```

### 12.6 设置外部服务状态

| 属性 | 值 |
|------|-----|
| **端点** | `/_matrix/client/v3/users/me/external-services/{service_type}/status` |
| **方法** | `PUT` |
| **认证** | 需要 |

**请求体**:

| 参数名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| `status` | string | 是 | 状态：`active` 或 `inactive` |

**请求示例**:
```json
{
  "status": "active"
}
```

**响应**: HTTP 200 OK

---

## 13. 错误码参考

### 13.1 标准 Matrix 错误码

| 错误码 | HTTP 状态码 | 说明 |
|--------|-------------|------|
| `M_FORBIDDEN` | 403 | 禁止访问 |
| `M_UNKNOWN_TOKEN` | 401 | 无效或过期的令牌 |
| `M_MISSING_TOKEN` | 401 | 缺少令牌 |
| `M_BAD_JSON` | 400 | JSON 格式错误 |
| `M_NOT_JSON` | 400 | 不是 JSON 格式 |
| `M_NOT_FOUND` | 404 | 资源不存在 |
| `M_LIMIT_EXCEEDED` | 429 | 请求过于频繁 |
| `M_UNKNOWN` | 500 | 未知错误 |
| `M_UNRECOGNIZED` | 400 | 无法识别的请求 |
| `M_UNAUTHORIZED` | 401 | 未授权 |
| `M_USER_DEACTIVATED` | 403 | 用户已停用 |
| `M_USER_IN_USE` | 400 | 用户名已存在 |
| `M_INVALID_USERNAME` | 400 | 无效的用户名 |
| `M_ROOM_IN_USE` | 400 | 房间已存在 |
| `M_INVALID_ROOM_STATE` | 400 | 无效的房间状态 |
| `M_THREEPID_IN_USE` | 400 | 第三方ID已存在 |
| `M_THREEPID_NOT_FOUND` | 404 | 第三方ID不存在 |
| `M_SERVER_NOT_TRUSTED` | 401 | 服务器不受信任 |
| `M_UNSUPPORTED_ROOM_VERSION` | 400 | 不支持的房间版本 |
| `M_INCOMPATIBLE_ROOM_VERSION` | 400 | 不兼容的房间版本 |
| `M_EXCLUSIVE_MXID` | 400 | 独占的MXID |

### 13.2 自定义错误码

| 错误码 | HTTP 状态码 | 说明 |
|--------|-------------|------|
| `M_VOICE_MESSAGE_TOO_LARGE` | 413 | 语音消息过大 |
| `M_VOICE_DURATION_EXCEEDED` | 400 | 语音时长超限 |
| `M_FRIEND_REQUEST_EXISTS` | 409 | 好友请求已存在 |
| `M_FRIEND_NOT_FOUND` | 404 | 好友不存在 |
| `M_EXTERNAL_SERVICE_ERROR` | 502 | 外部服务错误 |
| `M_INVALID_SERVICE_TYPE` | 400 | 无效的服务类型 |
| `M_BACKUP_VERSION_MISMATCH` | 409 | 备份版本不匹配 |

### 13.3 错误响应格式

```json
{
  "errcode": "M_UNKNOWN_TOKEN",
  "error": "Invalid token",
  "soft_logout": false
}
```

---

## 14. API 统计

### 14.1 按模块统计

| 模块 | 端点数量 | GET | POST | PUT | DELETE |
|------|---------|-----|------|-----|--------|
| 核心客户端 API | 68 | 35 | 20 | 10 | 3 |
| 管理员 API | 55 | 30 | 15 | 5 | 5 |
| 联邦通信 API | 40 | 25 | 10 | 5 | 0 |
| 好友系统 API | 13 | 4 | 7 | 2 | 0 |
| 端到端加密 API | 8 | 2 | 5 | 1 | 0 |
| 媒体文件 API | 9 | 6 | 3 | 0 | 0 |
| 语音消息 API | 10 | 6 | 4 | 0 | 0 |
| VoIP API | 3 | 3 | 0 | 0 | 0 |
| 密钥备份 API | 11 | 5 | 2 | 3 | 1 |
| 外部服务 API | 6 | 3 | 0 | 2 | 1 |
| **总计** | **223** | **120** | **66** | **28** | **10** |

### 14.2 按认证需求统计

| 认证类型 | 端点数量 | 说明 |
|----------|---------|------|
| 不需要认证 | 45 | 健康检查、版本信息、媒体下载等 |
| Bearer Token | 138 | 普通用户操作 |
| 管理员认证 | 55 | 服务器管理操作 |
| 联邦签名 | 30 | 服务器间通信 |

### 14.3 按API版本统计

| 版本 | 端点数量 |
|------|---------|
| r0 (Client-Server API v0) | 95 |
| v1 (Federation API) | 35 |
| v2 (Federation API) | 5 |
| v3 (Client-Server API v3) | 12 |
| Admin API | 55 |
| 自定义扩展 | 20 |

---

## 更新日志

| 版本 | 日期 | 更新内容 |
|------|------|----------|
| 6.0 | 2026-02-13 | 修正外部服务API统计（新增DELETE方法），总计223个端点 |
| 5.0 | 2026-02-13 | 完整扫描路由文件，新增45个API端点，总计222个 |
| 4.0 | 2025-01-15 | 新增外部服务API、完善管理员API |
| 3.0 | 2024-12-01 | 新增语音消息API、好友系统API |
| 2.0 | 2024-10-15 | 新增密钥备份API、完善E2EE API |
| 1.0 | 2024-08-01 | 初始版本，核心客户端API和联邦API |

---

> **文档生成说明**: 本文档通过扫描 `src/web/routes/` 目录下的所有路由文件自动生成。最后更新时间: 2026-02-13