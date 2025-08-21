-- 好友功能数据库结构
-- 基于之前创建的迁移脚本

-- 用户好友关系表
CREATE TABLE IF NOT EXISTS user_friendships (
    user_id TEXT NOT NULL,
    friend_user_id TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    PRIMARY KEY (user_id, friend_user_id)
);

-- 好友请求表
CREATE TABLE IF NOT EXISTS friend_requests (
    id BIGSERIAL PRIMARY KEY,
    requester_user_id TEXT NOT NULL,
    requested_user_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    message TEXT,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    UNIQUE(requester_user_id, requested_user_id)
);

-- 用户屏蔽关系表
CREATE TABLE IF NOT EXISTS user_blocks (
    user_id TEXT NOT NULL,
    blocked_user_id TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    PRIMARY KEY (user_id, blocked_user_id)
);

-- 好友搜索历史表
CREATE TABLE IF NOT EXISTS friend_search_history (
    id BIGSERIAL PRIMARY KEY,
    user_id TEXT NOT NULL,
    search_term TEXT NOT NULL,
    searched_at BIGINT NOT NULL
);

-- 创建索引以提高查询性能

-- 好友关系索引
CREATE INDEX IF NOT EXISTS idx_user_friendships_user_id ON user_friendships(user_id);
CREATE INDEX IF NOT EXISTS idx_user_friendships_friend_user_id ON user_friendships(friend_user_id);
CREATE INDEX IF NOT EXISTS idx_user_friendships_created_at ON user_friendships(created_at);

-- 好友请求索引
CREATE INDEX IF NOT EXISTS idx_friend_requests_requester ON friend_requests(requester_user_id);
CREATE INDEX IF NOT EXISTS idx_friend_requests_requested ON friend_requests(requested_user_id);
CREATE INDEX IF NOT EXISTS idx_friend_requests_status ON friend_requests(status);
CREATE INDEX IF NOT EXISTS idx_friend_requests_created_at ON friend_requests(created_at);
CREATE INDEX IF NOT EXISTS idx_friend_requests_updated_at ON friend_requests(updated_at);

-- 用户屏蔽索引
CREATE INDEX IF NOT EXISTS idx_user_blocks_user_id ON user_blocks(user_id);
CREATE INDEX IF NOT EXISTS idx_user_blocks_blocked_user_id ON user_blocks(blocked_user_id);
CREATE INDEX IF NOT EXISTS idx_user_blocks_created_at ON user_blocks(created_at);

-- 搜索历史索引
CREATE INDEX IF NOT EXISTS idx_friend_search_history_user_id ON friend_search_history(user_id);
CREATE INDEX IF NOT EXISTS idx_friend_search_history_searched_at ON friend_search_history(searched_at);
CREATE INDEX IF NOT EXISTS idx_friend_search_history_search_term ON friend_search_history(search_term);

-- 创建复合索引以优化常见查询
CREATE INDEX IF NOT EXISTS idx_friend_requests_user_status ON friend_requests(requester_user_id, status);
CREATE INDEX IF NOT EXISTS idx_friend_requests_requested_status ON friend_requests(requested_user_id, status);

-- 为搜索功能创建全文搜索索引
CREATE INDEX IF NOT EXISTS idx_friend_search_history_search_term_gin ON friend_search_history USING gin(search_term gin_trgm_ops);

-- 创建触发器以自动更新时间戳
CREATE OR REPLACE FUNCTION update_friend_request_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = extract(epoch from now()) * 1000;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_friend_request_timestamp
    BEFORE UPDATE ON friend_requests
    FOR EACH ROW
    EXECUTE FUNCTION update_friend_request_timestamp();

-- 创建约束以确保数据完整性
ALTER TABLE friend_requests ADD CONSTRAINT check_friend_request_status 
    CHECK (status IN ('pending', 'accepted', 'rejected', 'cancelled'));

ALTER TABLE friend_requests ADD CONSTRAINT check_different_users 
    CHECK (requester_user_id != requested_user_id);

ALTER TABLE user_friendships ADD CONSTRAINT check_different_friends 
    CHECK (user_id != friend_user_id);

ALTER TABLE user_blocks ADD CONSTRAINT check_different_block_users 
    CHECK (user_id != blocked_user_id);

-- 创建统计视图
CREATE OR REPLACE VIEW friend_statistics AS
SELECT 
    'total_friendships' as metric,
    COUNT(*) as value
FROM user_friendships
UNION ALL
SELECT 
    'pending_requests' as metric,
    COUNT(*) as value
FROM friend_requests WHERE status = 'pending'
UNION ALL
SELECT 
    'total_blocks' as metric,
    COUNT(*) as value
FROM user_blocks
UNION ALL
SELECT 
    'searches_today' as metric,
    COUNT(*) as value
FROM friend_search_history 
WHERE searched_at > extract(epoch from date_trunc('day', now())) * 1000;

-- 授予synapse用户权限
GRANT ALL PRIVILEGES ON user_friendships TO synapse;
GRANT ALL PRIVILEGES ON friend_requests TO synapse;
GRANT ALL PRIVILEGES ON user_blocks TO synapse;
GRANT ALL PRIVILEGES ON friend_search_history TO synapse;
GRANT ALL PRIVILEGES ON friend_statistics TO synapse;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO synapse;

COMMIT;