-- Add user_filters table for filter API support
CREATE TABLE IF NOT EXISTS user_filters (
    filter_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    filter_json TEXT NOT NULL,
    created_ts BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_user_filters_user_id ON user_filters(user_id);
