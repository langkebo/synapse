-- 用户外部服务配置表
-- 用于存储用户的外部服务 API 密钥和配置（如 TrendRadar、OpenClaw、OpenAI 等）

CREATE TABLE IF NOT EXISTS user_external_services (
    id SERIAL PRIMARY KEY,
    user_id VARCHAR(255) NOT NULL,
    service_type VARCHAR(50) NOT NULL,
    
    -- 服务端点（用户可自定义）
    endpoint VARCHAR(500) NOT NULL,
    
    -- 加密的 API 密钥
    api_key_encrypted TEXT,
    
    -- 用户配置（JSON 格式）
    config JSONB DEFAULT '{}',
    
    -- 状态
    status VARCHAR(20) DEFAULT 'active',
    last_used_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    
    CONSTRAINT unique_user_service UNIQUE(user_id, service_type)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_user_external_services_user ON user_external_services(user_id);
CREATE INDEX IF NOT EXISTS idx_user_external_services_type ON user_external_services(service_type);
CREATE INDEX IF NOT EXISTS idx_user_external_services_status ON user_external_services(status);

-- 注释
COMMENT ON TABLE user_external_services IS '用户外部服务配置表，存储用户的 API 密钥和服务配置';
COMMENT ON COLUMN user_external_services.user_id IS '用户ID';
COMMENT ON COLUMN user_external_services.service_type IS '服务类型：trendradar, openclaw, openai, claude, deepseek, custom 等';
COMMENT ON COLUMN user_external_services.endpoint IS '服务端点 URL';
COMMENT ON COLUMN user_external_services.api_key_encrypted IS '加密存储的 API 密钥';
COMMENT ON COLUMN user_external_services.config IS '用户自定义配置，如模型选择、温度等';
COMMENT ON COLUMN user_external_services.status IS '状态：active, inactive';
