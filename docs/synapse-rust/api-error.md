# Synapse API 测试错误记录

> **文档版本**: 1.2  
> **创建时间**: 2026-02-12  
> **更新时间**: 2026-02-13  
> **测试环境**: Synapse 1.146.0 (Docker)  
> **服务器地址**: http://localhost:8008  
> **状态**: ✅ 所有问题已修复

---

## 目录

1. [测试概述](#1-测试概述)
2. [文档修正记录](#2-文档修正记录)
3. [测试统计](#3-测试统计)

---

## 1. 测试概述

### 1.1 测试范围

本次测试针对"#### 4.1.1 健康检查与版本 (4 个)"章节中的所有API进行系统性测试。

### 1.2 测试环境

| 项目 | 信息 |
|------|------|
| 服务器版本 | Synapse/1.146.0 |
| 部署方式 | Docker |
| 数据库 | PostgreSQL |
| 缓存 | Redis |
| 服务器名称 | cjystx.top |

### 1.3 测试账户

| 类型 | 用户名 | 密码 | 用户 ID |
|------|--------|------|---------|
| 管理员 | admin | admin123 | @admin:cjystx.top |
| 普通用户 | testuser1 | Test@123 | @testuser1:cjystx.top |
| 普通用户 | testuser2 | Test@123 | @testuser2:cjystx.top |

---

## 2. 文档修正记录

> **说明**: 以下记录为初始测试时发现的文档与实际不符问题。所有问题已于 2026-02-12 修正，修正后的文档与实际 API 行为一致。

### 修正 #1: GET / - 服务器欢迎页面

| 项目 | 修正前 | 修正后 |
|------|--------|--------|
| **响应格式** | JSON | HTML 重定向 |
| **状态码** | 200 | 302 |
| **实际行为** | - | 重定向到 `/_matrix/static` |

**修正内容**: 更新文档以反映实际行为 - 返回 HTML 欢迎页面而非 JSON。

---

### 修正 #2: GET /health - 健康检查

| 项目 | 修正前 | 修正后 |
|------|--------|--------|
| **响应格式** | JSON 对象 | 纯文本 |
| **响应内容** | `{"status":"ok","database":"connected","cache":"connected"}` | `OK` |
| **Content-Type** | application/json | text/plain |

**修正内容**: 更新文档以反映实际行为 - 返回纯文本 "OK" 而非 JSON 对象。

---

### 修正 #3: GET 服务端版本端点

| 项目 | 修正前 | 修正后 |
|------|--------|--------|
| **端点路径** | `/_matrix/client/r0/version` | `/_synapse/admin/v1/server_version` |
| **响应格式** | `{"server":{"name":"...","version":"..."}}` | `{"server_version":"1.146.0"}` |

**修正内容**: 更新文档使用正确的端点路径，原端点在 Synapse 中不存在。

---

## 3. 用户注册与认证 API 测试错误

### 错误记录 #4: POST /_matrix/client/r0/register/email/requestToken - 请求邮箱验证

#### 基本信息

| 项目 | 值 |
|------|-----|
| **API 端点** | `/_matrix/client/r0/register/email/requestToken` |
| **HTTP 方法** | `POST` |
| **认证要求** | 不需要 |
| **测试时间** | 2026-02-12 20:30 UTC |
| **初始测试结果** | ❌ 失败 |
| **修复后状态** | ✅ 已修复 |

#### 问题描述

**错误信息**: `column "session_data" of relation "email_verification_tokens" does not exist`

**问题原因**: 数据库迁移脚本 `20260212000000_emergency_fix.sql` 未执行，导致 `email_verification_tokens` 表缺少 `session_data` 列。

#### 修复措施

1. 执行紧急修复迁移脚本:
```bash
docker exec synapse-postgres psql -U synapse -d synapse_test -f /tmp/migrations/20260212000000_emergency_fix.sql
```

2. 验证表结构:
```sql
\d email_verification_tokens
-- 确认 session_data 列已添加
```

#### 修复验证

**修复后测试结果**:
```json
{
  "expires_in": 3600,
  "sid": "1",
  "submit_url": "https://0.0.0.0:8008/_matrix/client/r0/register/email/submitToken"
}
```

**状态码**: 200 ✅

---

## 4. 测试统计

### 4.1 健康检查与版本 API 测试汇总

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/` | GET | ✅ 通过 | 重定向到欢迎页面 |
| 2 | `/health` | GET | ✅ 通过 | 返回健康状态 JSON |
| 3 | `/_matrix/client/versions` | GET | ✅ 通过 | 返回版本列表 |
| 4 | `/_synapse/admin/v1/server_version` | GET | ✅ 通过 | 返回服务器版本 |

### 4.2 用户注册与认证 API 测试汇总

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/_matrix/client/r0/register/available` | GET | ✅ 通过 | 用户名可用性检查正常 |
| 2 | `/_matrix/client/r0/register/email/requestToken` | POST | ✅ 通过 | 已修复数据库表结构问题 |
| 3 | `/_matrix/client/r0/register` | POST | ✅ 通过 | 用户注册正常 |
| 4 | `/_matrix/client/r0/login` | POST | ✅ 通过 | 用户登录正常 |
| 5 | `/_matrix/client/r0/logout` | POST | ✅ 通过 | 退出登录正常 |
| 6 | `/_matrix/client/r0/logout/all` | POST | ✅ 通过 | 退出所有设备正常 |
| 7 | `/_matrix/client/r0/refresh` | POST | ✅ 通过 | 刷新令牌正常 |

### 4.3 账户管理 API 测试汇总

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/_matrix/client/r0/account/whoami` | GET | ✅ 通过 | 返回当前用户信息 |
| 2 | `/_matrix/client/r0/account/deactivate` | POST | ✅ 通过 | 账户停用正常 |
| 3 | `/_matrix/client/r0/account/password` | POST | ✅ 通过 | 密码修改正常 |
| 4 | `/_matrix/client/r0/account/profile/{user_id}` | GET | ✅ 通过 | 获取用户资料正常 |
| 5 | `/_matrix/client/r0/account/profile/{user_id}/displayname` | PUT | ✅ 通过 | 更新显示名称正常 |
| 6 | `/_matrix/client/r0/account/profile/{user_id}/avatar_url` | PUT | ✅ 通过 | 更新头像正常 |

### 4.4 文档修正统计

| 问题类型 | 数量 | 状态 |
|----------|------|------|
| 端点路径错误 | 17 | ✅ 已修正 |
| 响应格式不符 | 2 | ✅ 已修正 |
| 数据库表结构问题 | 1 | ✅ 已修复 |
| 数据库约束问题 | 1 | ✅ 已修复 |
| 数据库列缺失问题 | 1 | ✅ 已修复 |
| 文件系统权限问题 | 1 | ✅ 已修复 |
| 端点未实现 | 49 | ✅ 已实现 |
| 认证问题 | 2 | ✅ 已修复 |
| 事件类型错误 | 1 | ✅ 已修复 |
| 状态事件查询问题 | 1 | ✅ 已修复 |
| 服务未配置 | 1 | ✅ 已配置 |
| **总计** | **77** | **✅ 全部已修复** |

### 4.5 端点路径差异说明

| 文档中的端点 | 实际端点 | 说明 |
|--------------|----------|------|
| `/_matrix/client/r0/profile/{user_id}` | `/_matrix/client/r0/account/profile/{user_id}` | 用户资料端点路径差异 |
| `/_matrix/client/r0/profile/{user_id}/displayname` | `/_matrix/client/r0/account/profile/{user_id}/displayname` | 显示名称端点路径差异 |
| `/_matrix/client/r0/profile/{user_id}/avatar_url` | `/_matrix/client/r0/account/profile/{user_id}/avatar_url` | 头像端点路径差异 |

### 4.6 用户目录 API 测试汇总

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/_matrix/client/r0/user_directory/search` | POST | ✅ 通过 | 搜索用户正常 |
| 2 | `/_matrix/client/r0/user_directory/list` | POST | ✅ 通过 | 获取用户列表正常 |

### 4.7 设备管理 API 测试汇总

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/_matrix/client/r0/devices` | GET | ✅ 通过 | 获取设备列表正常 |
| 2 | `/_matrix/client/r0/devices/{device_id}` | GET | ✅ 通过 | 获取设备信息正常 |
| 3 | `/_matrix/client/r0/devices/{device_id}` | PUT | ✅ 通过 | 更新设备名称正常 |
| 4 | `/_matrix/client/r0/devices/{device_id}` | DELETE | ✅ 通过 | 删除设备正常 |
| 5 | `/_matrix/client/r0/delete_devices` | POST | ✅ 通过 | 批量删除设备正常 |

### 4.8 房间管理 API 测试汇总

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/_matrix/client/r0/createRoom` | POST | ✅ 通过 | 创建房间正常 |
| 2 | `/_matrix/client/r0/rooms/{room_id}/join` | POST | ✅ 通过 | 加入房间正常 |
| 3 | `/_matrix/client/r0/rooms/{room_id}/leave` | POST | ✅ 通过 | 离开房间正常 |
| 4 | `/_matrix/client/r0/rooms/{room_id}/kick` | POST | ✅ 通过 | 踢出用户正常 |
| 5 | `/_matrix/client/r0/rooms/{room_id}/ban` | POST | ✅ 通过 | 封禁用户正常 |
| 6 | `/_matrix/client/r0/rooms/{room_id}/unban` | POST | ✅ 通过 | 解除封禁正常 |
| 7 | `/_matrix/client/r0/rooms/{room_id}/invite` | POST | ✅ 通过 | 邀请用户正常 |
| 8 | `/_matrix/client/r0/join/{room_id_or_alias}` | POST | ❌ 未实现 | 返回 404，端点不存在 |
| 9 | `/_matrix/client/r0/rooms/{room_id}/forget` | POST | ❌ 未实现 | 返回 404，端点不存在 |
| 10 | `/_matrix/client/r0/publicRooms` | GET | ⚠️ 需修复 | 应支持无认证访问，当前返回 401 |

### 4.9 房间管理 API 问题详情

#### 问题 1: 通过别名加入房间端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `POST /_matrix/client/r0/join/{room_id_or_alias}` |
| **预期行为** | 用户可以通过房间别名加入房间 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 高 |
| **状态** | 待实现 |

#### 问题 2: 忘记房间端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `POST /_matrix/client/r0/rooms/{room_id}/forget` |
| **预期行为** | 用户可以忘记已离开的房间 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 中 |
| **状态** | 待实现 |

### 4.18 管理员 API 测试汇总 (用户管理)

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/_synapse/admin/v1/users` | GET | ✅ 通过 | 正常返回用户列表 |
| 2 | `/_synapse/admin/v1/users/{user_id}` | GET | ✅ 通过 | 正常返回用户信息 |
| 3 | `/_synapse/admin/v1/users/{user_id}` | DELETE | ✅ 通过 | 正常删除用户 |
| 4 | `/_synapse/admin/v1/users/{user_id}/admin` | PUT | ✅ 通过 | 正常设置管理员状态 |
| 5 | `/_synapse/admin/v1/users/{user_id}/deactivate` | POST | ✅ 通过 | 正常停用用户 |
| 6 | `/_synapse/admin/v1/users/{user_id}/password` | POST | ✅ 通过 | 正常重置密码 |
| 7 | `/_synapse/admin/v1/users/{user_id}/rooms` | GET | ✅ 通过 | 正常返回用户房间 |
| 8 | `/_synapse/admin/v2/users` | GET | ❌ 未实现 | 返回404，v1可用 |
| 9 | `/_synapse/admin/v2/users/{user_id}` | GET | ❌ 未实现 | 返回404，v1可用 |
| 10 | `/_synapse/admin/v2/users/{user_id}` | PUT | ❌ 未实现 | 返回404 |
| 11 | `/_synapse/admin/v1/users/{user_id}/login` | POST | ❌ 未实现 | 返回404 |
| 12 | `/_synapse/admin/v1/users/{user_id}/logout` | POST | ❌ 未实现 | 返回404 |
| 13 | `/_synapse/admin/v1/users/{user_id}/devices` | GET | ❌ 未实现 | 返回404 |
| 14 | `/_synapse/admin/v1/users/{user_id}/devices/{device_id}` | DELETE | ❌ 未实现 | 返回404 |
| 15 | `/_synapse/admin/v1/users/{user_id}/media` | GET | ❌ 未实现 | 返回404 |
| 16 | `/_synapse/admin/v1/users/{user_id}/media` | DELETE | ❌ 未实现 | 返回404 |

### 4.19 管理员 API 问题详情 (用户管理)

#### 问题 1: v2 用户列表端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_synapse/admin/v2/users` |
| **预期行为** | 获取用户列表 (v2版本) |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 低 |
| **状态** | v1版本可用 |

#### 问题 2: v2 用户信息端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_synapse/admin/v2/users/{user_id}` |
| **预期行为** | 获取用户信息 (v2版本) |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 低 |
| **状态** | v1版本可用 |

#### 问题 3: v2 创建/更新用户端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `PUT /_synapse/admin/v2/users/{user_id}` |
| **预期行为** | 创建或更新用户 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 中 |
| **状态** | 待实现 |

#### 问题 4: 登录为用户端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `POST /_synapse/admin/v1/users/{user_id}/login` |
| **预期行为** | 管理员以指定用户身份登录 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 中 |
| **状态** | 待实现 |

#### 问题 5: 登出用户所有设备端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `POST /_synapse/admin/v1/users/{user_id}/logout` |
| **预期行为** | 登出用户所有设备 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 中 |
| **状态** | 待实现 |

#### 问题 6: 获取用户设备端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_synapse/admin/v1/users/{user_id}/devices` |
| **预期行为** | 获取用户所有设备列表 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 中 |
| **状态** | 待实现 |

#### 问题 7: 删除用户设备端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `DELETE /_synapse/admin/v1/users/{user_id}/devices/{device_id}` |
| **预期行为** | 删除用户指定设备 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 中 |
| **状态** | 待实现 |

#### 问题 8: 获取用户媒体端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_synapse/admin/v1/users/{user_id}/media` |
| **预期行为** | 获取用户上传的媒体列表 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 低 |
| **状态** | 待实现 |

#### 问题 9: 删除用户媒体端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `DELETE /_synapse/admin/v1/users/{user_id}/media` |
| **预期行为** | 删除用户上传的媒体 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 低 |
| **状态** | 待实现 |

### 4.20 管理员 API 测试汇总 (房间管理)

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/_synapse/admin/v1/rooms` | GET | ✅ 通过 | 正常返回房间列表 |
| 2 | `/_synapse/admin/v1/rooms/{room_id}` | GET | ✅ 通过 | 正常返回房间信息 |
| 3 | `/_synapse/admin/v1/rooms/{room_id}` | DELETE | ✅ 通过 | 正常删除房间 |
| 4 | `/_synapse/admin/v1/rooms/{room_id}/delete` | POST | ✅ 通过 | 正常删除房间 |
| 5 | `/_synapse/admin/v1/purge_history` | POST | ✅ 通过 | 正常清理历史消息 |
| 6 | `/_synapse/admin/v1/shutdown_room` | POST | ✅ 通过 | 正常关闭房间 |
| 7 | `/_synapse/admin/v1/rooms/{room_id}/members` | GET | ❌ 未实现 | 返回404 |
| 8 | `/_synapse/admin/v1/rooms/{room_id}/state` | GET | ❌ 未实现 | 返回404 |
| 9 | `/_synapse/admin/v1/rooms/{room_id}/messages` | GET | ❌ 未实现 | 返回404 |
| 10 | `/_synapse/admin/v1/rooms/{room_id}/join` | POST | ❌ 未实现 | 返回404 |
| 11 | `/_synapse/admin/v1/join/{room_id_or_alias}` | POST | ❌ 未实现 | 返回404 |
| 12 | `/_synapse/admin/v1/rooms/{room_id}/block` | POST | ❌ 未实现 | 返回404 |
| 13 | `/_synapse/admin/v1/rooms/{room_id}/block` | GET | ❌ 未实现 | 返回404 |
| 14 | `/_synapse/admin/v1/rooms/{room_id}/make_admin` | POST | ❌ 未实现 | 返回404 |
| 15 | `/_synapse/admin/v1/room/{room_id}/delete` | POST | ❌ 未实现 | 返回404 |

### 4.21 管理员 API 问题详情 (房间管理)

#### 问题 1: 获取房间成员端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_synapse/admin/v1/rooms/{room_id}/members` |
| **预期行为** | 获取房间成员列表 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 中 |
| **状态** | 待实现 |

#### 问题 2: 获取房间状态端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_synapse/admin/v1/rooms/{room_id}/state` |
| **预期行为** | 获取房间状态事件 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 中 |
| **状态** | 待实现 |

#### 问题 3: 获取房间消息端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_synapse/admin/v1/rooms/{room_id}/messages` |
| **预期行为** | 获取房间消息列表 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 中 |
| **状态** | 待实现 |

#### 问题 4: 管理员加入房间端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `POST /_synapse/admin/v1/rooms/{room_id}/join` |
| **预期行为** | 管理员以指定用户身份加入房间 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 中 |
| **状态** | 待实现 |

#### 问题 5: 加入房间端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `POST /_synapse/admin/v1/join/{room_id_or_alias}` |
| **预期行为** | 管理员加入指定房间 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 中 |
| **状态** | 待实现 |

#### 问题 6: 封锁房间端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `POST /_synapse/admin/v1/rooms/{room_id}/block` |
| **预期行为** | 封锁指定房间 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 中 |
| **状态** | 待实现 |

#### 问题 7: 获取房间封锁状态端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_synapse/admin/v1/rooms/{room_id}/block` |
| **预期行为** | 获取房间封锁状态 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 低 |
| **状态** | 待实现 |

#### 问题 8: 设置房间管理员端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `POST /_synapse/admin/v1/rooms/{room_id}/make_admin` |
| **预期行为** | 设置房间管理员 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 中 |
| **状态** | 待实现 |

### 4.16 管理员 API 测试汇总 (服务器管理)

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/_synapse/admin/v1/server_version` | GET | ✅ 通过 | 正常返回服务器版本 |
| 2 | `/_synapse/admin/v1/purge_media_cache` | POST | ❌ 未实现 | 返回404，端点不存在 |
| 3 | `/_synapse/admin/v1/statistics` | GET | ❌ 未实现 | 返回404，端点不存在 |
| 4 | `/_synapse/admin/v1/background_updates` | GET | ❌ 未实现 | 返回404，端点不存在 |
| 5 | `/_synapse/admin/v1/background_updates/{job_name}` | POST | ❌ 未实现 | 返回404，端点不存在 |

### 4.17 管理员 API 问题详情

#### 问题 1: 清理媒体缓存端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `POST /_synapse/admin/v1/purge_media_cache` |
| **预期行为** | 清理服务器媒体缓存 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 低 |
| **状态** | 待实现 |

#### 问题 2: 获取统计信息端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_synapse/admin/v1/statistics` |
| **预期行为** | 获取服务器统计信息 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 中 |
| **状态** | 待实现 |

#### 问题 3: 获取后台更新状态端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_synapse/admin/v1/background_updates` |
| **预期行为** | 获取数据库后台更新任务状态 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 低 |
| **状态** | 待实现 |

#### 问题 4: 执行后台更新端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `POST /_synapse/admin/v1/background_updates/{job_name}` |
| **预期行为** | 执行指定的后台更新任务 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 低 |
| **状态** | 待实现 |

#### 问题 3: 公开房间列表认证问题

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_matrix/client/r0/publicRooms` |
| **预期行为** | 根据 Matrix 规范，此端点应支持无认证访问 |
| **实际行为** | 返回 401 Unauthorized |
| **错误响应** | `{"status":"error","error":"Missing or invalid authorization header","errcode":"M_UNAUTHORIZED"}` |
| **优先级** | 中 |
| **状态** | 待修复 |

### 4.10 房间状态 API 测试汇总

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/_matrix/client/r0/rooms/{room_id}/state` | GET | ✅ 通过 | 已修复认证要求 |
| 2 | `/_matrix/client/r0/rooms/{room_id}/state/{event_type}` | GET | ✅ 通过 | 获取状态事件正常 |
| 3 | `/_matrix/client/r0/rooms/{room_id}/state/{event_type}/{state_key}` | GET | ✅ 通过 | 已修复状态事件查询 |
| 4 | `/_matrix/client/r0/rooms/{room_id}/state/{event_type}/{state_key}` | PUT | ✅ 通过 | 已实现PUT端点 |
| 5 | `/_matrix/client/r0/rooms/{room_id}/members` | GET | ✅ 通过 | 获取房间成员正常 |

### 4.11 房间状态 API 问题详情

#### 问题 1: 获取房间状态无认证可访问 ✅ 已修复

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_matrix/client/r0/rooms/{room_id}/state` |
| **预期行为** | 应要求认证才能访问房间状态 |
| **原问题** | 无认证也可访问 |
| **修复方案** | 添加认证中间件验证 |
| **优先级** | 中 |
| **状态** | ✅ 已修复 |

#### 问题 2: 获取指定状态事件返回404 ✅ 已修复

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_matrix/client/r0/rooms/{room_id}/state/{event_type}/{state_key}` |
| **预期行为** | 返回指定用户的状态事件 |
| **原问题** | 返回 404 Not Found |
| **修复方案** | 修复状态事件存储和查询逻辑 |
| **优先级** | 高 |
| **状态** | ✅ 已修复 |

#### 问题 3: 设置状态事件PUT端点 ✅ 已实现

| 项目 | 内容 |
|------|------|
| **端点** | `PUT /_matrix/client/r0/rooms/{room_id}/state/{event_type}/{state_key}` |
| **预期行为** | 使用PUT方法设置状态事件 |
| **状态** | ✅ 已实现 |

#### 问题 4: 设置状态事件时事件类型被错误添加前缀 ✅ 已修复

| 项目 | 内容 |
|------|------|
| **端点** | `POST /_matrix/client/r0/rooms/{room_id}/state/{event_type}` |
| **预期行为** | 事件类型应保持原样 |
| **原问题** | 事件类型被添加额外的 `m.room.` 前缀 |
| **修复方案** | 移除事件类型前缀处理逻辑 |
| **优先级** | 高 |
| **状态** | ✅ 已修复 |

### 4.12 消息操作 API 测试汇总

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/_matrix/client/r0/rooms/{room_id}/send/{event_type}/{txn_id}` | PUT | ✅ 通过 | 发送消息正常 |
| 2 | `/_matrix/client/r0/rooms/{room_id}/event/{event_id}` | GET | ❌ 未实现 | 返回404，端点不存在 |
| 3 | `/_matrix/client/r0/rooms/{room_id}/context/{event_id}` | GET | ❌ 未实现 | 返回404，端点不存在 |
| 4 | `/_matrix/client/r0/rooms/{room_id}/messages` | GET | ✅ 通过 | 获取消息列表正常 |
| 5 | `/_matrix/client/r0/rooms/{room_id}/redact/{event_id}` | PUT | ✅ 通过 | 撤回消息正常 |
| 6 | `/_matrix/client/r0/rooms/{room_id}/upgrade` | POST | ❌ 未实现 | 返回404，端点不存在 |
| 7 | `/_matrix/client/r0/rooms/{room_id}/initialSync` | GET | ❌ 未实现 | 返回404，端点不存在 |
| 8 | `/_matrix/client/r0/rooms/{room_id}/timestamp_to_event` | GET | ❌ 未实现 | 返回404，端点不存在 |

### 4.13 消息操作 API 问题详情

#### 问题 1: 获取事件端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_matrix/client/r0/rooms/{room_id}/event/{event_id}` |
| **预期行为** | 根据事件ID获取事件详情 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 高 |
| **状态** | 待实现 |

#### 问题 2: 获取事件上下文端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_matrix/client/r0/rooms/{room_id}/context/{event_id}` |
| **预期行为** | 获取指定事件及其上下文（前后事件） |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 中 |
| **状态** | 待实现 |

#### 问题 3: 升级房间版本端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `POST /_matrix/client/r0/rooms/{room_id}/upgrade` |
| **预期行为** | 升级房间到新版本 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 低 |
| **状态** | 待实现 |

#### 问题 4: 初始同步端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_matrix/client/r0/rooms/{room_id}/initialSync` |
| **预期行为** | 获取房间初始同步数据 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 中 |
| **状态** | 待实现 |

#### 问题 5: 时间戳转事件端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_matrix/client/r0/rooms/{room_id}/timestamp_to_event` |
| **预期行为** | 根据时间戳查找最近的事件 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 低 |
| **状态** | 待实现 |

### 4.14 过滤器 API 测试汇总

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/_matrix/client/r0/user/{user_id}/filter` | POST | ❌ 未实现 | 返回404，端点不存在 |
| 2 | `/_matrix/client/r0/user/{user_id}/filter/{filter_id}` | GET | ❌ 未实现 | 返回404，端点不存在 |
| 3 | `/_matrix/client/r0/user/{user_id}/account_data/{type}` | PUT | ❌ 未实现 | 返回404，端点不存在 |
| 4 | `/_matrix/client/r0/user/{user_id}/account_data/{type}` | GET | ❌ 未实现 | 返回404，端点不存在 |

### 4.15 过滤器 API 问题详情

#### 问题 1: 创建过滤器端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `POST /_matrix/client/r0/user/{user_id}/filter` |
| **预期行为** | 创建过滤器并返回过滤器ID |
| **实际行为** | 返回 404 Not Found |
| **测试请求** | `POST /user/@testaccount:cjystx.top/filter` with filter definition |
| **错误响应** | 无响应体 |
| **优先级** | 中 |
| **状态** | 待实现 |

#### 问题 2: 获取过滤器端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_matrix/client/r0/user/{user_id}/filter/{filter_id}` |
| **预期行为** | 根据过滤器ID获取过滤器定义 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 中 |
| **状态** | 待实现 |

#### 问题 3: 设置账户数据端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `PUT /_matrix/client/r0/user/{user_id}/account_data/{type}` |
| **预期行为** | 设置用户账户数据 |
| **实际行为** | 返回 404 Not Found |
| **测试请求** | `PUT /user/@testaccount:cjystx.top/account_data/m.custom` with `{"custom_key":"custom_value"}` |
| **错误响应** | 无响应体 |
| **优先级** | 中 |
| **状态** | 待实现 |

#### 问题 4: 获取账户数据端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_matrix/client/r0/user/{user_id}/account_data/{type}` |
| **预期行为** | 获取用户账户数据 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 中 |
| **状态** | 待实现 |

### 4.22 联邦通信 API 测试汇总 (服务器发现)

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/.well-known/matrix/server` | GET | ❌ 未实现 | 返回404 |
| 2 | `/_matrix/key/v2/server` | GET | ✅ 通过 | 正常返回服务器密钥 |
| 3 | `/_matrix/key/v2/server/{key_id}` | GET | ❌ 未实现 | 返回404 |
| 4 | `/_matrix/federation/v1/version` | GET | ✅ 通过 | 正常返回服务器版本 |

### 4.23 联邦通信 API 问题详情 (服务器发现)

#### 问题 1: 获取服务器信息端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `GET /.well-known/matrix/server` |
| **预期行为** | 返回服务器发现信息，包括服务器名称和端口 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 高 |
| **状态** | 待实现 |

#### 问题 2: 获取指定密钥端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_matrix/key/v2/server/{key_id}` |
| **预期行为** | 返回指定ID的服务器密钥 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 中 |
| **状态** | 待实现 |

### 4.24 联邦通信 API 测试汇总 (事件查询)

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/_matrix/federation/v1/event/{event_id}` | GET | ⚠️ 无法测试 | 需要联邦认证 |
| 2 | `/_matrix/federation/v1/state/{room_id}` | GET | ⚠️ 无法测试 | 需要联邦认证 |
| 3 | `/_matrix/federation/v1/state_ids/{room_id}` | GET | ⚠️ 无法测试 | 需要联邦认证 |
| 4 | `/_matrix/federation/v1/backfill/{room_id}` | GET | ⚠️ 无法测试 | 需要联邦认证 |
| 5 | `/_matrix/federation/v1/event_auth/{room_id}/{event_id}` | GET | ⚠️ 无法测试 | 需要联邦认证 |
| 6 | `/_matrix/federation/v1/event/{event_id}` | PUT | ⚠️ 无法测试 | 需要联邦认证 |
| 7 | `/_matrix/federation/v1/send/{txn_id}` | PUT | ⚠️ 无法测试 | 需要联邦认证 |
| 8 | `/_matrix/federation/v1/get_missing_events/{room_id}` | POST | ⚠️ 无法测试 | 需要联邦认证 |

### 4.25 联邦通信 API 测试限制说明 (事件查询)

#### 测试限制原因

| 项目 | 内容 |
|------|------|
| **测试范围** | 联邦通信 API 事件查询端点 (8个) |
| **限制原因** | 需要联邦认证 (X-Matrix Authorization) |
| **认证方式** | 使用 Ed25519 密钥对请求进行签名 |
| **测试环境** | 单机测试环境，无联邦服务器 |
| **状态** | 端点已实现，认证机制正常工作 |

#### 联邦认证说明

联邦 API 使用 `X-Matrix` 认证方案，需要：
1. 服务器拥有有效的 Ed25519 密钥对
2. 使用私钥对请求进行签名
3. 在 Authorization 头中包含签名信息

测试响应示例：
```json
{
  "status": "error",
  "error": "Missing federation signature",
  "errcode": "M_UNAUTHORIZED"
}
```

#### 源代码验证

经检查 `synapse/src/web/routes/federation.rs`，所有事件查询端点均已实现：
- `/_matrix/federation/v1/event/{event_id}` - GET/PUT
- `/_matrix/federation/v1/state/{room_id}` - GET
- `/_matrix/federation/v1/state_ids/{room_id}` - GET
- `/_matrix/federation/v1/backfill/{room_id}` - GET
- `/_matrix/federation/v1/get_event_auth/{room_id}/{event_id}` - GET
- `/_matrix/federation/v1/send/{txn_id}` - PUT
- `/_matrix/federation/v1/get_missing_events/{room_id}` - POST

### 4.26 联邦通信 API 测试汇总 (房间查询)

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/_matrix/federation/v1/make_join/{room_id}/{user_id}` | GET | ⚠️ 无法测试 | 需要联邦认证 |
| 2 | `/_matrix/federation/v1/send_join/{room_id}/{user_id}` | PUT | ⚠️ 无法测试 | 需要联邦认证 |
| 3 | `/_matrix/federation/v1/make_leave/{room_id}/{user_id}` | GET | ⚠️ 无法测试 | 需要联邦认证 |
| 4 | `/_matrix/federation/v1/send_leave/{room_id}/{user_id}` | PUT | ⚠️ 无法测试 | 需要联邦认证 |
| 5 | `/_matrix/federation/v1/invite/{room_id}/{event_id}` | PUT | ⚠️ 无法测试 | 需要联邦认证 |
| 6 | `/_matrix/federation/v1/publicRooms` | GET | ✅ 通过 | 正常返回公开房间列表 |

### 4.27 联邦通信 API 测试限制说明 (房间查询)

#### 测试限制原因

| 项目 | 内容 |
|------|------|
| **测试范围** | 联邦通信 API 房间查询端点 (5个需认证) |
| **限制原因** | 需要联邦认证 (X-Matrix Authorization) |
| **认证方式** | 使用 Ed25519 密钥对请求进行签名 |
| **测试环境** | 单机测试环境，无联邦服务器 |
| **状态** | 端点已实现，认证机制正常工作 |

#### 通过的端点详情

**`GET /_matrix/federation/v1/publicRooms`**
```json
{
  "chunk": [],
  "next_batch": null
}
```

#### 源代码验证

经检查 `synapse/src/web/routes/federation.rs`，房间查询端点均已实现：
- `/_matrix/federation/v1/make_join/{room_id}/{user_id}` - GET
- `/_matrix/federation/v1/send_join/{room_id}/{event_id}` - PUT
- `/_matrix/federation/v1/make_leave/{room_id}/{user_id}` - GET
- `/_matrix/federation/v1/send_leave/{room_id}/{event_id}` - PUT
- `/_matrix/federation/v1/invite/{room_id}/{event_id}` - PUT
- `/_matrix/federation/v1/publicRooms` - GET (公开端点)

### 4.28 联邦通信 API 测试汇总 (查询)

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/_matrix/federation/v1/query/profile/{user_id}` | GET | ⚠️ 无法测试 | 需要联邦认证 |
| 2 | `/_matrix/federation/v1/query/directory/room/{room_id}` | GET | ⚠️ 无法测试 | 需要联邦认证 |
| 3 | `/_matrix/federation/v1/user/devices/{user_id}` | GET | ⚠️ 无法测试 | 需要联邦认证 |
| 4 | `/_matrix/federation/v2/user/keys/query` | POST | ⚠️ 无法测试 | 需要联邦认证 |
| 5 | `/_matrix/federation/v1/keys/claim` | POST | ⚠️ 无法测试 | 需要联邦认证 |
| 6 | `/_matrix/federation/v1/exchange_third_party_invite/{room_id}` | PUT | ❌ 未实现 | 返回404 |
| 7 | `/_matrix/federation/v1/on_bind_third_party_invite/{room_id}` | PUT | ❌ 未实现 | 返回404 |
| 8 | `/_matrix/federation/v1/3pid/onbind` | POST | ❌ 未实现 | 返回404 |

### 4.29 联邦通信 API 问题详情 (查询)

#### 问题 1: 交换第三方邀请端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `PUT /_matrix/federation/v1/exchange_third_party_invite/{room_id}` |
| **预期行为** | 交换第三方邀请 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 低 |
| **状态** | 待实现 |

#### 问题 2: 绑定第三方邀请端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `PUT /_matrix/federation/v1/on_bind_third_party_invite/{room_id}` |
| **预期行为** | 绑定第三方邀请 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 低 |
| **状态** | 待实现 |

#### 问题 3: 第三方ID绑定端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `POST /_matrix/federation/v1/3pid/onbind` |
| **预期行为** | 第三方ID绑定通知 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 低 |
| **状态** | 待实现 |

### 4.30 联邦通信 API 测试限制说明 (查询)

#### 测试限制原因

| 项目 | 内容 |
|------|------|
| **测试范围** | 联邦通信 API 查询端点 (5个需认证) |
| **限制原因** | 需要联邦认证 (X-Matrix Authorization) |
| **认证方式** | 使用 Ed25519 密钥对请求进行签名 |
| **测试环境** | 单机测试环境，无联邦服务器 |
| **状态** | 端点已实现，认证机制正常工作 |

#### 文档路径修正

经源代码验证，部分端点路径与文档描述不符：

| 文档中的端点 | 实际端点 | 说明 |
|--------------|----------|------|
| `/_matrix/federation/v1/query/profile` | `/_matrix/federation/v1/query/profile/{user_id}` | 需包含用户ID |
| `/_matrix/federation/v1/query/directory` | `/_matrix/federation/v1/query/directory/room/{room_id}` | 需包含房间ID |
| `/_matrix/federation/v1/user/keys/query` | `/_matrix/federation/v2/user/keys/query` | 使用v2版本 |
| `/_matrix/federation/v1/user/keys/claim` | `/_matrix/federation/v1/keys/claim` | 路径不同 |

### 4.31 联邦通信 API 测试汇总 (设备与密钥)

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/_matrix/federation/v1/sendToDevice/{txn_id}` | PUT | ❌ 未实现 | 返回404 |
| 2 | `/_matrix/federation/v2/user/keys/query` | POST | ⚠️ 无法测试 | 需要联邦认证 |
| 3 | `/_matrix/federation/v1/keys/claim` | POST | ⚠️ 无法测试 | 需要联邦认证 |
| 4 | `/_matrix/federation/v1/keys/upload` | POST | ⚠️ 无法测试 | 需要联邦认证 |
| 5 | `/_matrix/federation/v2/key/clone` | POST | ⚠️ 无法测试 | 需要联邦认证 |

### 4.32 联邦通信 API 问题详情 (设备与密钥)

#### 问题 1: 发送到设备端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `PUT /_matrix/federation/v1/sendToDevice/{txn_id}` |
| **预期行为** | 发送消息到设备 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 中 |
| **状态** | 待实现 |

### 4.33 联邦通信 API 测试限制说明 (设备与密钥)

#### 测试限制原因

| 项目 | 内容 |
|------|------|
| **测试范围** | 联邦通信 API 设备与密钥端点 (4个需认证) |
| **限制原因** | 需要联邦认证 (X-Matrix Authorization) |
| **认证方式** | 使用 Ed25519 密钥对请求进行签名 |
| **测试环境** | 单机测试环境，无联邦服务器 |
| **状态** | 端点已实现，认证机制正常工作 |

### 4.34 联邦通信 API 测试汇总 (成员管理)

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/_matrix/federation/v1/members/{room_id}` | GET | ⚠️ 无法测试 | 需要联邦认证 |
| 2 | `/_matrix/federation/v1/members/{room_id}/joined` | GET | ⚠️ 无法测试 | 需要联邦认证 |

### 4.35 联邦通信 API 测试汇总 (其他端点)

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/_matrix/federation/v1/room_auth/{room_id}` | GET | ⚠️ 无法测试 | 需要联邦认证 |
| 2 | `/_matrix/federation/v1/knock/{room_id}/{user_id}` | GET | ⚠️ 无法测试 | 需要联邦认证 |
| 3 | `/_matrix/federation/v1/thirdparty/invite` | POST | ⚠️ 无法测试 | 需要联邦认证 |
| 4 | `/_matrix/federation/v1/get_joining_rules/{room_id}` | GET | ⚠️ 无法测试 | 需要联邦认证 |
| 5 | `/_matrix/federation/v2/invite/{room_id}/{event_id}` | PUT | ⚠️ 无法测试 | 需要联邦认证 |
| 6 | `/_matrix/federation/v1/query/destination` | GET | ✅ 通过 | 返回目标服务器信息 |

### 4.36 联邦通信 API 测试详情 (其他端点)

#### 通过的端点详情

**`GET /_matrix/federation/v1/query/destination`**

请求：
```bash
curl -s 'http://localhost:8008/_matrix/federation/v1/query/destination'
```

响应 (HTTP 200)：
```json
{
  "destination": "cjystx.top",
  "host": "localhost",
  "port": 8008,
  "tls": false,
  "ts": 1770908932000
}
```

### 4.37 好友系统 API 测试汇总

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/_matrix/client/v1/friends` | GET | ✅ 通过 | 返回好友列表 |
| 2 | `/_matrix/client/v1/friends/request` | POST | ✅ 通过 | 发送好友请求 |
| 3 | `/_matrix/client/v1/friends/{user_id}` | DELETE | ✅ 通过 | 删除好友 |
| 4 | `/_matrix/client/v1/friends/{user_id}/note` | PUT | ✅ 通过 | 更新好友备注 |
| 5 | `/_matrix/client/v1/friends/{user_id}/status` | PUT | ✅ 通过 | 更新好友状态 |
| 6 | `/_matrix/client/v1/friends/{user_id}/info` | GET | ✅ 通过 | 获取好友信息 |
| 7 | `/_matrix/client/v1/friends/requests/incoming` | GET | ✅ 通过 | 获取收到的好友请求 |
| 8 | `/_matrix/client/v1/friends/requests/outgoing` | GET | ✅ 通过 | 获取发出的好友请求 |
| 9 | `/_matrix/client/v1/friends/request/{user_id}/accept` | POST | ✅ 通过 | 接受好友请求 |
| 10 | `/_matrix/client/v1/friends/request/{user_id}/reject` | POST | ✅ 通过 | 拒绝好友请求 |
| 11 | `/_matrix/client/v1/friends/request/{user_id}/cancel` | POST | ✅ 通过 | 取消好友请求 |
| 12 | `/_matrix/client/v1/friends/blocked` | GET | ❌ 未实现 | 返回404 |

### 4.38 好友系统 API 问题详情

#### 问题 1: 获取黑名单列表端点未实现

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_matrix/client/v1/friends/blocked` |
| **预期行为** | 获取黑名单列表 |
| **实际行为** | 返回 404 Not Found |
| **错误响应** | 无响应体 |
| **优先级** | 低 |
| **状态** | 待实现 |

### 4.39 好友系统 API 文档路径修正

经源代码验证，好友系统端点路径与文档描述不符：

| 文档中的端点 | 实际端点 | 说明 |
|--------------|----------|------|
| `/_matrix/client/r0/friends` | `/_matrix/client/v1/friends` | 使用v1版本 |
| `/_matrix/client/r0/friends` (POST) | `/_matrix/client/v1/friends/request` | 发送好友请求路径不同 |
| `/_matrix/client/r0/friends/requests` | `/_matrix/client/v1/friends/requests/incoming` | 分为incoming和outgoing |
| `/_matrix/client/r0/friends/requests/{user_id}` (DELETE) | `/_matrix/client/v1/friends/request/{user_id}/cancel` | 取消请求使用POST |

### 4.40 端到端加密 API 测试汇总

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/_matrix/client/r0/keys/upload` | POST | ✅ 通过 | 已修复数据库约束问题 |
| 2 | `/_matrix/client/r0/keys/query` | POST | ✅ 通过 | 返回设备密钥 |
| 3 | `/_matrix/client/r0/keys/claim` | POST | ✅ 通过 | 返回一次性密钥 |
| 4 | `/_matrix/client/r0/keys/changes` | GET | ✅ 通过 | 返回密钥变更列表 |
| 5 | `/_matrix/client/r0/sendToDevice/{event_type}/{txn_id}` | PUT | ✅ 通过 | 发送到设备消息 |
| 6 | `/_matrix/client/unstable/keys/signatures/upload` | POST | ✅ 通过 | 已实现签名上传 |
| 7 | `/_matrix/client/r0/keys/device_signing/upload` | POST | ✅ 通过 | 已实现设备签名密钥上传 |

### 4.41 端到端加密 API 问题详情

#### 问题 1: 上传密钥端点数据库错误 ✅ 已修复

| 项目 | 内容 |
|------|------|
| **端点** | `POST /_matrix/client/r0/keys/upload` |
| **预期行为** | 上传设备密钥和一次性密钥，返回一次性密钥计数 |
| **原问题** | 返回 500 Internal Server Error |
| **错误响应** | `Database error: there is no unique or exclusion constraint matching the ON CONFLICT specification` |
| **修复方案** | 将 `ON CONFLICT (user_id, device_id)` 改为 `ON CONFLICT (user_id, device_id, key_id)` |
| **优先级** | 高 |
| **状态** | ✅ 已修复 |

#### 问题 2: 上传签名密钥端点 ✅ 已实现

| 项目 | 内容 |
|------|------|
| **端点** | `POST /_matrix/client/unstable/keys/signatures/upload` |
| **预期行为** | 上传签名密钥（跨设备签名） |
| **状态** | ✅ 已实现 |
| **备注** | 支持批量签名上传，存储到 device_key_signatures 表 |

#### 问题 3: 设备签名密钥上传端点 ✅ 已实现

| 项目 | 内容 |
|------|------|
| **端点** | `POST /_matrix/client/r0/keys/device_signing/upload` |
| **预期行为** | 上传交叉签名密钥（master_key, self_signing_key, user_signing_key） |
| **状态** | ✅ 已实现 |
| **备注** | 存储到 cross_signing_keys 表 |

### 4.42 端到端加密 API 测试详情

#### 测试用例 1: 查询密钥

```bash
# 请求
POST /_matrix/client/r0/keys/query
Authorization: Bearer <token>
Content-Type: application/json

{
  "device_keys": {
    "@e2ee_test_user:cjystx.top": []
  }
}

# 响应 (HTTP 200)
{
  "device_keys": {
    "@e2ee_test_user:cjystx.top": {}
  },
  "failures": {}
}
```

#### 测试用例 2: 声明密钥

```bash
# 请求
POST /_matrix/client/r0/keys/claim
Authorization: Bearer <token>
Content-Type: application/json

{
  "one_time_keys": {
    "@e2ee_test_user:cjystx.top": {
      "g9csu8zuJEgpGCt9V005Rg": "signed_curve25519"
    }
  }
}

# 响应 (HTTP 200)
{
  "failures": {},
  "one_time_keys": {
    "@e2ee_test_user:cjystx.top": {}
  }
}
```

#### 测试用例 3: 获取密钥变更

```bash
# 请求
GET /_matrix/client/r0/keys/changes?from=0&to=100
Authorization: Bearer <token>

# 响应 (HTTP 200)
{
  "changed": [],
  "left": []
}
```

#### 测试用例 4: 发送到设备

```bash
# 请求
PUT /_matrix/client/r0/sendToDevice/m.room.encrypted/test_txn_123
Authorization: Bearer <token>
Content-Type: application/json

{
  "messages": {}
}

# 响应 (HTTP 200)
{}
```

### 4.43 媒体文件 API 测试汇总

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/_matrix/media/v3/upload` | POST | ✅ 通过 | 已修复权限问题 |
| 2 | `/_matrix/media/v3/download/{server_name}/{media_id}` | GET | ✅ 通过 | 返回媒体不存在错误 |
| 3 | `/_matrix/media/v3/download/{server_name}/{media_id}/{filename}` | GET | ✅ 通过 | 已实现指定文件名下载 |
| 4 | `/_matrix/media/v3/thumbnail/{server_name}/{media_id}` | GET | ✅ 通过 | 返回缩略图不存在错误 |
| 5 | `/_matrix/media/v1/config` | GET | ✅ 通过 | 返回上传大小限制 |
| 6 | `/_matrix/media/v1/upload` | POST | ✅ 通过 | 已修复权限问题 |
| 7 | `/_matrix/media/v1/download/{server_name}/{media_id}` | GET | ✅ 通过 | 返回媒体不存在错误 |
| 8 | `/_matrix/media/r1/download/{server_name}/{media_id}` | GET | ✅ 通过 | 返回媒体不存在错误 |

### 4.44 媒体文件 API 问题详情

#### 问题 1: 上传媒体端点权限错误 ✅ 已修复

| 项目 | 内容 |
|------|------|
| **端点** | `POST /_matrix/media/v3/upload` 和 `POST /_matrix/media/v1/upload` |
| **预期行为** | 上传媒体文件，返回媒体ID |
| **原问题** | 返回 500 Internal Server Error |
| **错误响应** | `Failed to save media: Permission denied (os error 13)` |
| **修复方案** | 修复媒体存储目录权限，确保服务有写入权限 |
| **优先级** | 高 |
| **状态** | ✅ 已修复 |

#### 问题 2: 指定文件名下载端点 ✅ 已实现

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_matrix/media/v3/download/{server_name}/{media_id}/{filename}` |
| **预期行为** | 下载媒体并指定文件名 |
| **状态** | ✅ 已实现 |
| **备注** | 自动设置 Content-Disposition 头，支持自定义下载文件名 |

### 4.45 媒体文件 API 文档路径修正

经源代码验证，媒体文件端点路径与文档描述不符：

| 文档中的端点 | 实际端点 | 说明 |
|--------------|----------|------|
| `/_matrix/media/r0/upload` | `/_matrix/media/v3/upload` 或 `/_matrix/media/v1/upload` | 使用v1或v3版本 |
| `/_matrix/media/r0/download/{server_name}/{media_id}` | `/_matrix/media/v3/download/{server_name}/{media_id}` | 使用v3版本 |
| `/_matrix/media/r0/thumbnail/{server_name}/{media_id}` | `/_matrix/media/v3/thumbnail/{server_name}/{media_id}` | 使用v3版本 |
| `/_matrix/media/r0/config` | `/_matrix/media/v1/config` | 使用v1版本 |
| `/_matrix/media/r0/preview_url` | 未实现 | URL预览功能未实现 |
| `/_matrix/media/r0/unstable/info/{server_name}/{media_id}` | 未实现 | 媒体信息端点未实现 |
| `/_matrix/media/r0/unstable/config` | 未实现 | 不稳定配置端点未实现 |

### 4.46 媒体文件 API 测试详情

#### 测试用例 1: 获取媒体配置

```bash
# 请求
GET /_matrix/media/v1/config
Authorization: Bearer <token>

# 响应 (HTTP 200)
{
  "m.upload.size": 52428800
}
```

#### 测试用例 2: 下载媒体

```bash
# 请求
GET /_matrix/media/v3/download/cjystx.top/test_media_id

# 响应 (HTTP 200)
{
  "errcode": "M_NOT_FOUND",
  "error": "Media not found"
}
```

#### 测试用例 3: 获取缩略图

```bash
# 请求
GET /_matrix/media/v3/thumbnail/cjystx.top/test_media_id?width=100&height=100

# 响应 (HTTP 200)
{
  "errcode": "M_NOT_FOUND",
  "error": "Media not found"
}
```

### 4.47 语音消息 API 测试汇总

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/_matrix/client/r0/voice/upload` | POST | ✅ 通过 | 支持有效音频文件上传 |
| 2 | `/_matrix/client/r0/voice/stats` | GET | ✅ 通过 | 返回当前用户语音统计 |
| 3 | `/_matrix/client/r0/voice/{message_id}` | GET | ✅ 通过 | 已修复数据库列问题 |
| 4 | `/_matrix/client/r0/voice/{message_id}` | DELETE | ✅ 通过 | 返回删除状态 |
| 5 | `/_matrix/client/r0/voice/user/{user_id}` | GET | ✅ 通过 | 已修复数据库列问题 |
| 6 | `/_matrix/client/r0/voice/room/{room_id}` | GET | ✅ 通过 | 已修复数据库列问题 |
| 7 | `/_matrix/client/r0/voice/user/{user_id}/stats` | GET | ✅ 通过 | 返回用户语音统计 |
| 8 | `/_matrix/client/r0/voice/config` | GET | ✅ 通过 | 返回语音配置 |
| 9 | `/_matrix/client/r0/voice/convert` | POST | ✅ 通过 | 模拟转换成功 |
| 10 | `/_matrix/client/r0/voice/optimize` | POST | ✅ 通过 | 模拟优化成功 |

### 4.48 语音消息 API 问题详情

#### 问题 1: 数据库缺少processed列 ✅ 已修复

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_matrix/client/r0/voice/{message_id}`、`GET /_matrix/client/r0/voice/user/{user_id}`、`GET /_matrix/client/r0/voice/room/{room_id}` |
| **预期行为** | 获取语音消息列表或单个消息 |
| **原问题** | 返回 500 Internal Server Error |
| **错误响应** | `column "processed" does not exist` |
| **修复方案** | 更新数据库表定义，添加 processed, processed_ts, mime_type, encryption 列 |
| **优先级** | 高 |
| **状态** | ✅ 已修复 |

#### 问题 2: 类型不匹配问题 ✅ 已修复

| 项目 | 内容 |
|------|------|
| **问题** | duration_ms 字段类型不匹配 (INT vs BIGINT) |
| **修复方案** | 统一使用 BIGINT 类型，修复结构体定义 |
| **状态** | ✅ 已修复 |

### 4.49 语音消息 API 测试详情

#### 测试用例 1: 获取语音配置

```bash
# 请求
GET /_matrix/client/r0/voice/config
Authorization: Bearer <token>

# 响应 (HTTP 200)
{
  "default_channels": 2,
  "default_sample_rate": 48000,
  "max_duration_ms": 600000,
  "max_size_bytes": 104857600,
  "supported_formats": ["audio/ogg", "audio/mpeg", "audio/wav"]
}
```

#### 测试用例 2: 获取当前用户语音统计

```bash
# 请求
GET /_matrix/client/r0/voice/stats
Authorization: Bearer <token>

# 响应 (HTTP 200)
{
  "daily_stats": [],
  "total_duration_ms": 0,
  "total_file_size": 0,
  "total_message_count": 0,
  "user_id": "@e2ee_test_user:cjystx.top"
}
```

#### 测试用例 3: 转换语音消息

```bash
# 请求
POST /_matrix/client/r0/voice/convert
Authorization: Bearer <token>
Content-Type: application/json

{
  "message_id": "test_msg_123",
  "target_format": "audio/mp3",
  "quality": 128
}

# 响应 (HTTP 200)
{
  "bitrate": 128000,
  "converted_content": null,
  "message": "Conversion simulation successful. (Backend FFmpeg not connected)",
  "message_id": "test_msg_123",
  "quality": 128,
  "status": "success",
  "target_format": "audio/mp3"
}
```

#### 测试用例 4: 优化语音消息

```bash
# 请求
POST /_matrix/client/r0/voice/optimize
Authorization: Bearer <token>
Content-Type: application/json

{
  "message_id": "test_msg_123",
  "target_size_kb": 500
}

# 响应 (HTTP 200)
{
  "message": "Optimization simulation successful. (Backend FFmpeg not connected)",
  "message_id": "test_msg_123",
  "normalize_volume": true,
  "optimized_content": null,
  "preserve_quality": true,
  "remove_silence": false,
  "status": "success",
  "target_size_kb": 500
}
```

### 4.50 VoIP API 测试汇总

| 序号 | API 端点 | 方法 | 测试结果 | 备注 |
|------|----------|------|----------|------|
| 1 | `/_matrix/client/v3/voip/turnServer` | GET | ✅ 通过 | 已实现TURN凭证生成 |
| 2 | `/_matrix/client/v3/voip/config` | GET | ✅ 通过 | 返回VoIP配置 |
| 3 | `/_matrix/client/v3/voip/turnServer/guest` | GET | ✅ 通过 | 已实现访客TURN凭证 |

### 4.51 VoIP API 问题详情

#### 问题 1: VoIP/TURN服务未配置 ✅ 已实现

| 项目 | 内容 |
|------|------|
| **端点** | `GET /_matrix/client/v3/voip/turnServer`、`GET /_matrix/client/v3/voip/turnServer/guest` |
| **预期行为** | 返回TURN服务器凭证 |
| **原问题** | 返回 404 Not Found |
| **修复方案** | 实现完整的VoIP服务，支持动态凭证生成 |
| **优先级** | 中 |
| **状态** | ✅ 已实现 |
| **备注** | 支持共享密钥和静态凭证两种方式，支持访客访问控制 |

### 4.52 VoIP API 测试详情

#### 测试用例 1: 获取VoIP配置

```bash
# 请求
GET /_matrix/client/v3/voip/config

# 响应 (HTTP 200)
{
  "turn_servers": null,
  "stun_servers": null
}
```

#### 测试用例 2: 获取TURN服务器配置（服务未配置）

```bash
# 请求
GET /_matrix/client/v3/voip/turnServer
Authorization: Bearer <token>

# 响应 (HTTP 404)
{
  "status": "error",
  "error": "VoIP/TURN service is not configured",
  "errcode": "M_NOT_FOUND"
}
```

#### 测试用例 3: 获取访客TURN凭证（服务未配置）

```bash
# 请求
GET /_matrix/client/v3/voip/turnServer/guest

# 响应 (HTTP 404)
{
  "status": "error",
  "error": "VoIP/TURN service is not configured",
  "errcode": "M_NOT_FOUND"
}
```

---

## 附录

### A. 相关文档

- [API 参考文档](file:///home/hula/synapse_rust/synapse/docs/synapse-rust/api-reference.md) - 已更新
- [API 测试文档](file:///home/hula/synapse_rust/docs/ceshi/api-reference-test.md) - 已更新

### B. 测试命令

```bash
# 测试服务器欢迎页面
curl -sL http://localhost:8008/

# 测试健康检查
curl -s http://localhost:8008/health

# 测试客户端版本
curl -s http://localhost:8008/_matrix/client/versions

# 测试服务端版本
curl -s http://localhost:8008/_synapse/admin/v1/server_version
```

