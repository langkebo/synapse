-- PostgreSQL初始化脚本
-- 为Synapse创建数据库和用户

-- 创建pg_stat_statements扩展
CREATE EXTENSION IF NOT EXISTS pg_stat_statements;

-- 创建Synapse数据库（如果不存在）
-- 注意：数据库已通过环境变量创建，这里只是确保配置正确

-- 设置数据库参数
ALTER DATABASE synapse SET default_text_search_config = 'pg_catalog.english';
ALTER DATABASE synapse SET timezone = 'UTC';

-- 为Synapse用户设置权限
GRANT ALL PRIVILEGES ON DATABASE synapse TO synapse;

-- 连接到synapse数据库
\c synapse;

-- 创建必要的扩展
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";
CREATE EXTENSION IF NOT EXISTS "btree_gin";

-- 为好友功能创建索引优化
-- 这些索引将在好友功能表创建后自动应用

-- 设置搜索路径
ALTER USER synapse SET search_path = public;

-- 创建性能监控视图
CREATE OR REPLACE VIEW synapse_performance AS
SELECT 
    query,
    calls,
    total_time,
    mean_time,
    rows,
    100.0 * shared_blks_hit / nullif(shared_blks_hit + shared_blks_read, 0) AS hit_percent
FROM pg_stat_statements 
ORDER BY total_time DESC;

-- 授予synapse用户查看性能统计的权限
GRANT SELECT ON pg_stat_statements TO synapse;
GRANT SELECT ON synapse_performance TO synapse;

-- 创建清理函数
CREATE OR REPLACE FUNCTION cleanup_old_data()
RETURNS void AS $$
BEGIN
    -- 清理超过30天的好友搜索历史
    DELETE FROM friend_search_history 
    WHERE searched_at < NOW() - INTERVAL '30 days';
    
    -- 清理超过90天的已拒绝好友请求
    DELETE FROM friend_requests 
    WHERE status = 'rejected' 
    AND updated_at < NOW() - INTERVAL '90 days';
    
    -- 重置统计信息
    SELECT pg_stat_statements_reset();
END;
$$ LANGUAGE plpgsql;

-- 创建定期清理任务（需要pg_cron扩展，可选）
-- SELECT cron.schedule('cleanup-synapse', '0 2 * * *', 'SELECT cleanup_old_data();');

COMMIT;