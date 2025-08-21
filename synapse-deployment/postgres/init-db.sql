-- PostgreSQL 初始化脚本
-- 用于Synapse Matrix服务器和好友功能
-- 创建数据库、用户和权限配置

-- ============================================================================
-- 数据库和用户创建
-- ============================================================================

-- 创建Synapse主用户
CREATE USER synapse_user WITH
    PASSWORD 'synapse_secure_password_2024'
    CREATEDB
    NOSUPERUSER
    NOCREATEROLE
    NOINHERIT
    LOGIN
    NOREPLICATION
    NOBYPASSRLS
    CONNECTION LIMIT 20;

-- 创建好友功能用户
CREATE USER friends_user WITH
    PASSWORD 'friends_secure_password_2024'
    NOCREATEDB
    NOSUPERUSER
    NOCREATEROLE
    NOINHERIT
    LOGIN
    NOREPLICATION
    NOBYPASSRLS
    CONNECTION LIMIT 10;

-- 创建监控用户
CREATE USER monitor_user WITH
    PASSWORD 'monitor_secure_password_2024'
    NOCREATEDB
    NOSUPERUSER
    NOCREATEROLE
    NOINHERIT
    LOGIN
    NOREPLICATION
    NOBYPASSRLS
    CONNECTION LIMIT 5;

-- 创建备份用户
CREATE USER backup_user WITH
    PASSWORD 'backup_secure_password_2024'
    NOCREATEDB
    NOSUPERUSER
    NOCREATEROLE
    NOINHERIT
    LOGIN
    NOREPLICATION
    NOBYPASSRLS
    CONNECTION LIMIT 2;

-- 创建只读用户
CREATE USER readonly_user WITH
    PASSWORD 'readonly_secure_password_2024'
    NOCREATEDB
    NOSUPERUSER
    NOCREATEROLE
    NOINHERIT
    LOGIN
    NOREPLICATION
    NOBYPASSRLS
    CONNECTION LIMIT 5;

-- 创建应用程序用户
CREATE USER app_user WITH
    PASSWORD 'app_secure_password_2024'
    NOCREATEDB
    NOSUPERUSER
    NOCREATEROLE
    NOINHERIT
    LOGIN
    NOREPLICATION
    NOBYPASSRLS
    CONNECTION LIMIT 15;

-- ============================================================================
-- 数据库创建
-- ============================================================================

-- 创建Synapse主数据库
CREATE DATABASE synapse WITH
    OWNER = synapse_user
    ENCODING = 'UTF8'
    LC_COLLATE = 'C'
    LC_CTYPE = 'C'
    TABLESPACE = pg_default
    CONNECTION LIMIT = -1
    TEMPLATE = template0;

-- 添加数据库注释
COMMENT ON DATABASE synapse IS 'Synapse Matrix服务器主数据库，包含用户数据、房间信息和好友功能';

-- ============================================================================
-- 权限配置
-- ============================================================================

-- 连接到synapse数据库进行权限配置
\c synapse;

-- 为synapse_user授予完整权限
GRANT ALL PRIVILEGES ON DATABASE synapse TO synapse_user;
GRANT ALL PRIVILEGES ON SCHEMA public TO synapse_user;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO synapse_user;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO synapse_user;
GRANT ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public TO synapse_user;

-- 为friends_user授予必要权限
GRANT CONNECT ON DATABASE synapse TO friends_user;
GRANT USAGE ON SCHEMA public TO friends_user;

-- 为monitor_user授予监控权限
GRANT CONNECT ON DATABASE synapse TO monitor_user;
GRANT USAGE ON SCHEMA public TO monitor_user;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO monitor_user;

-- 为backup_user授予备份权限
GRANT CONNECT ON DATABASE synapse TO backup_user;
GRANT USAGE ON SCHEMA public TO backup_user;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO backup_user;

-- 为readonly_user授予只读权限
GRANT CONNECT ON DATABASE synapse TO readonly_user;
GRANT USAGE ON SCHEMA public TO readonly_user;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO readonly_user;

-- 为app_user授予应用程序权限
GRANT CONNECT ON DATABASE synapse TO app_user;
GRANT USAGE ON SCHEMA public TO app_user;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO app_user;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO app_user;

-- ============================================================================
-- 好友功能专用表和权限
-- ============================================================================

-- 创建好友功能相关表（如果不存在）
-- 注意：这些表通常由Synapse迁移脚本创建，这里仅作为备用

-- 好友关系表
CREATE TABLE IF NOT EXISTS friends_relationships (
    id BIGSERIAL PRIMARY KEY,
    user_id TEXT NOT NULL,
    friend_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    UNIQUE(user_id, friend_id)
);

-- 好友请求表
CREATE TABLE IF NOT EXISTS friends_requests (
    id BIGSERIAL PRIMARY KEY,
    from_user_id TEXT NOT NULL,
    to_user_id TEXT NOT NULL,
    message TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    UNIQUE(from_user_id, to_user_id)
);

-- 好友设置表
CREATE TABLE IF NOT EXISTS friends_settings (
    user_id TEXT PRIMARY KEY,
    allow_friend_requests BOOLEAN DEFAULT TRUE,
    auto_accept_friends BOOLEAN DEFAULT FALSE,
    show_online_status BOOLEAN DEFAULT TRUE,
    privacy_level TEXT DEFAULT 'normal',
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);

-- 好友推荐表
CREATE TABLE IF NOT EXISTS friends_recommendations (
    id BIGSERIAL PRIMARY KEY,
    user_id TEXT NOT NULL,
    recommended_user_id TEXT NOT NULL,
    reason TEXT,
    score FLOAT DEFAULT 0.0,
    created_at BIGINT NOT NULL,
    UNIQUE(user_id, recommended_user_id)
);

-- 好友活动日志表
CREATE TABLE IF NOT EXISTS friends_activity_log (
    id BIGSERIAL PRIMARY KEY,
    user_id TEXT NOT NULL,
    action TEXT NOT NULL,
    target_user_id TEXT,
    details JSONB,
    created_at BIGINT NOT NULL
);

-- ============================================================================
-- 索引创建
-- ============================================================================

-- 好友关系表索引
CREATE INDEX IF NOT EXISTS idx_friends_relationships_user_id ON friends_relationships(user_id);
CREATE INDEX IF NOT EXISTS idx_friends_relationships_friend_id ON friends_relationships(friend_id);
CREATE INDEX IF NOT EXISTS idx_friends_relationships_status ON friends_relationships(status);
CREATE INDEX IF NOT EXISTS idx_friends_relationships_created_at ON friends_relationships(created_at);

-- 好友请求表索引
CREATE INDEX IF NOT EXISTS idx_friends_requests_from_user_id ON friends_requests(from_user_id);
CREATE INDEX IF NOT EXISTS idx_friends_requests_to_user_id ON friends_requests(to_user_id);
CREATE INDEX IF NOT EXISTS idx_friends_requests_status ON friends_requests(status);
CREATE INDEX IF NOT EXISTS idx_friends_requests_created_at ON friends_requests(created_at);

-- 好友推荐表索引
CREATE INDEX IF NOT EXISTS idx_friends_recommendations_user_id ON friends_recommendations(user_id);
CREATE INDEX IF NOT EXISTS idx_friends_recommendations_score ON friends_recommendations(score DESC);
CREATE INDEX IF NOT EXISTS idx_friends_recommendations_created_at ON friends_recommendations(created_at);

-- 好友活动日志表索引
CREATE INDEX IF NOT EXISTS idx_friends_activity_log_user_id ON friends_activity_log(user_id);
CREATE INDEX IF NOT EXISTS idx_friends_activity_log_action ON friends_activity_log(action);
CREATE INDEX IF NOT EXISTS idx_friends_activity_log_created_at ON friends_activity_log(created_at);

-- ============================================================================
-- 好友功能权限配置
-- ============================================================================

-- 为friends_user授予好友功能表的权限
GRANT SELECT, INSERT, UPDATE, DELETE ON friends_relationships TO friends_user;
GRANT SELECT, INSERT, UPDATE, DELETE ON friends_requests TO friends_user;
GRANT SELECT, INSERT, UPDATE, DELETE ON friends_settings TO friends_user;
GRANT SELECT, INSERT, UPDATE, DELETE ON friends_recommendations TO friends_user;
GRANT SELECT, INSERT, UPDATE, DELETE ON friends_activity_log TO friends_user;

-- 授予序列权限
GRANT USAGE, SELECT ON SEQUENCE friends_relationships_id_seq TO friends_user;
GRANT USAGE, SELECT ON SEQUENCE friends_requests_id_seq TO friends_user;
GRANT USAGE, SELECT ON SEQUENCE friends_recommendations_id_seq TO friends_user;
GRANT USAGE, SELECT ON SEQUENCE friends_activity_log_id_seq TO friends_user;

-- 为app_user授予好友功能表的权限
GRANT SELECT, INSERT, UPDATE, DELETE ON friends_relationships TO app_user;
GRANT SELECT, INSERT, UPDATE, DELETE ON friends_requests TO app_user;
GRANT SELECT, INSERT, UPDATE, DELETE ON friends_settings TO app_user;
GRANT SELECT, INSERT, UPDATE, DELETE ON friends_recommendations TO app_user;
GRANT SELECT, INSERT, UPDATE, DELETE ON friends_activity_log TO app_user;

-- 授予序列权限
GRANT USAGE, SELECT ON SEQUENCE friends_relationships_id_seq TO app_user;
GRANT USAGE, SELECT ON SEQUENCE friends_requests_id_seq TO app_user;
GRANT USAGE, SELECT ON SEQUENCE friends_recommendations_id_seq TO app_user;
GRANT USAGE, SELECT ON SEQUENCE friends_activity_log_id_seq TO app_user;

-- ============================================================================
-- 扩展安装
-- ============================================================================

-- 安装必要的PostgreSQL扩展
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";
CREATE EXTENSION IF NOT EXISTS "btree_gin";
CREATE EXTENSION IF NOT EXISTS "btree_gist";

-- 如果需要统计扩展
-- CREATE EXTENSION IF NOT EXISTS "pg_stat_statements";

-- ============================================================================
-- 性能优化配置
-- ============================================================================

-- 设置表的存储参数
ALTER TABLE friends_relationships SET (
    fillfactor = 90,
    autovacuum_vacuum_scale_factor = 0.1,
    autovacuum_analyze_scale_factor = 0.05
);

ALTER TABLE friends_requests SET (
    fillfactor = 90,
    autovacuum_vacuum_scale_factor = 0.1,
    autovacuum_analyze_scale_factor = 0.05
);

ALTER TABLE friends_activity_log SET (
    fillfactor = 95,
    autovacuum_vacuum_scale_factor = 0.2,
    autovacuum_analyze_scale_factor = 0.1
);

-- ============================================================================
-- 安全配置
-- ============================================================================

-- 撤销public schema的默认权限
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
REVOKE ALL ON DATABASE synapse FROM PUBLIC;

-- 设置行级安全策略（如果需要）
-- ALTER TABLE friends_relationships ENABLE ROW LEVEL SECURITY;
-- CREATE POLICY friends_policy ON friends_relationships
--     FOR ALL TO friends_user
--     USING (user_id = current_setting('app.current_user_id'));

-- ============================================================================
-- 监控和统计
-- ============================================================================

-- 创建监控视图
CREATE OR REPLACE VIEW friends_stats AS
SELECT 
    'relationships' as table_name,
    COUNT(*) as total_count,
    COUNT(CASE WHEN status = 'accepted' THEN 1 END) as accepted_count,
    COUNT(CASE WHEN status = 'pending' THEN 1 END) as pending_count,
    COUNT(CASE WHEN status = 'blocked' THEN 1 END) as blocked_count
FROM friends_relationships
UNION ALL
SELECT 
    'requests' as table_name,
    COUNT(*) as total_count,
    COUNT(CASE WHEN status = 'pending' THEN 1 END) as pending_count,
    COUNT(CASE WHEN status = 'accepted' THEN 1 END) as accepted_count,
    COUNT(CASE WHEN status = 'rejected' THEN 1 END) as rejected_count
FROM friends_requests;

-- 授予监控用户查看统计视图的权限
GRANT SELECT ON friends_stats TO monitor_user;
GRANT SELECT ON friends_stats TO readonly_user;

-- ============================================================================
-- 数据清理和维护
-- ============================================================================

-- 创建清理过期数据的函数
CREATE OR REPLACE FUNCTION cleanup_old_friend_data()
RETURNS void AS $$
BEGIN
    -- 清理90天前的已拒绝好友请求
    DELETE FROM friends_requests 
    WHERE status = 'rejected' 
    AND created_at < EXTRACT(EPOCH FROM NOW() - INTERVAL '90 days') * 1000;
    
    -- 清理180天前的活动日志
    DELETE FROM friends_activity_log 
    WHERE created_at < EXTRACT(EPOCH FROM NOW() - INTERVAL '180 days') * 1000;
    
    -- 清理过期的好友推荐（30天）
    DELETE FROM friends_recommendations 
    WHERE created_at < EXTRACT(EPOCH FROM NOW() - INTERVAL '30 days') * 1000;
    
    RAISE NOTICE '好友数据清理完成';
END;
$$ LANGUAGE plpgsql;

-- 授予执行清理函数的权限
GRANT EXECUTE ON FUNCTION cleanup_old_friend_data() TO synapse_user;
GRANT EXECUTE ON FUNCTION cleanup_old_friend_data() TO app_user;

-- ============================================================================
-- 触发器和约束
-- ============================================================================

-- 创建更新时间戳的触发器函数
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = EXTRACT(EPOCH FROM NOW()) * 1000;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 为相关表添加更新时间戳触发器
CREATE TRIGGER update_friends_relationships_updated_at
    BEFORE UPDATE ON friends_relationships
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_friends_requests_updated_at
    BEFORE UPDATE ON friends_requests
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_friends_settings_updated_at
    BEFORE UPDATE ON friends_settings
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 默认数据插入
-- ============================================================================

-- 插入一些默认的好友设置
-- 注意：实际用户设置应该在用户注册时创建

-- ============================================================================
-- 权限最终确认
-- ============================================================================

-- 确保所有新创建的对象都有正确的权限
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO synapse_user;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO synapse_user;
GRANT ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public TO synapse_user;

-- 为未来创建的对象设置默认权限
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO synapse_user;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO synapse_user;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON FUNCTIONS TO synapse_user;

ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO monitor_user;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO readonly_user;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO backup_user;

-- ============================================================================
-- 完成消息
-- ============================================================================

-- 显示初始化完成信息
DO $$
BEGIN
    RAISE NOTICE '===================================================';
    RAISE NOTICE 'PostgreSQL 数据库初始化完成！';
    RAISE NOTICE '===================================================';
    RAISE NOTICE '创建的用户：';
    RAISE NOTICE '  - synapse_user: Synapse主用户';
    RAISE NOTICE '  - friends_user: 好友功能用户';
    RAISE NOTICE '  - monitor_user: 监控用户';
    RAISE NOTICE '  - backup_user: 备份用户';
    RAISE NOTICE '  - readonly_user: 只读用户';
    RAISE NOTICE '  - app_user: 应用程序用户';
    RAISE NOTICE '===================================================';
    RAISE NOTICE '创建的数据库：';
    RAISE NOTICE '  - synapse: 主数据库';
    RAISE NOTICE '===================================================';
    RAISE NOTICE '安全提醒：';
    RAISE NOTICE '  1. 请立即修改所有用户的默认密码';
    RAISE NOTICE '  2. 检查pg_hba.conf配置';
    RAISE NOTICE '  3. 启用SSL连接（生产环境）';
    RAISE NOTICE '  4. 定期备份数据库';
    RAISE NOTICE '  5. 监控数据库性能和安全日志';
    RAISE NOTICE '===================================================';
END $$;

-- ============================================================================
-- 脚本结束
-- ============================================================================