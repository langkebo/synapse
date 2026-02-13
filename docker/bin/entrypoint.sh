#!/bin/sh
set -e

echo "=== Synapse Rust Startup Script ==="

# 等待数据库就绪
echo "Waiting for database to be ready..."
until nc -z ${DB_HOST:-db} ${DB_PORT:-5432}; do
  echo "Database is unavailable - sleeping"
  sleep 2
done
echo "Database is up!"

# 执行数据库迁移
echo "Running database migrations..."
if [ -d "/app/migrations" ]; then
    # 使用sqlx-cli或内置迁移
    if command -v sqlx &> /dev/null; then
        export DATABASE_URL="postgres://${DB_USER:-synapse}:${DB_PASSWORD:-synapse}@${DB_HOST:-db}:${DB_PORT:-5432}/${DB_NAME:-synapse}"
        sqlx migrate run --source /app/migrations
        echo "Migrations completed via sqlx-cli"
    else
        echo "sqlx-cli not found, migrations will be handled by application"
    fi
else
    echo "No migrations directory found, skipping"
fi

# 启动应用
echo "Starting Synapse Rust server..."
exec /usr/local/bin/synapse-rust
