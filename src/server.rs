use axum::{routing::{get, post, put, delete, options}, Router};
use axum::body::Body;
use axum::http::{Request, StatusCode, Method, HeaderValue};
use tower_http::cors::{CorsLayer, Any};
use tower_http::trace::TraceLayer;
use std::sync::Arc;
use std::net::SocketAddr;
use tokio::signal;
use tracing::info;
use serde_json::json;

use crate::common::*;
use crate::storage::*;
use crate::cache::*;
use crate::auth::*;
use crate::services::*;
use crate::web::*;

pub struct SynapseServer {
    app_state: Arc<AppState>,
    router: Router,
    address: SocketAddr,
    media_path: std::path::PathBuf,
}

impl SynapseServer {
    pub async fn new(
        database_url: &str,
        server_name: &str,
        jwt_secret: &str,
        address: SocketAddr,
        media_path: std::path::PathBuf,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let pool = sqlx::PgPool::connect(database_url).await?;
        initialize_database(&pool).await?;

        let cache = Arc::new(CacheManager::new(cache::CacheConfig::default()));
        let services = ServiceContainer::new(&pool, cache.clone(), jwt_secret, server_name);
        let app_state = Arc::new(AppState::new(services, cache.clone()));

        let client_routes = create_router(app_state.clone());
        let admin_routes = create_admin_router(app_state.clone());
        let media_routes = create_media_router(app_state.clone(), media_path.clone());
        let federation_routes = create_federation_router(app_state.clone());

        let router = Router::new()
            .merge(client_routes)
            .merge(admin_routes)
            .merge(media_routes)
            .merge(federation_routes)
            .route("/_matrix/client/versions", get(|| async { json!({"versions": ["r0.0.1", "r0.1.0", "r0.2.0", "r0.3.0", "r0.4.0", "r0.5.0", "r0.6.0"]}) }))
            .route("/*path", get(|| async { json!({"errcode": "UNKNOWN", "error": "Unknown endpoint"}) }))
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS].into_iter().collect::<std::collections::HashSet<_>>())
                    .allow_headers(Any)
                    .allow_credentials(false)
            )
            .layer(TraceLayer::new_for_http());

        Ok(Self {
            app_state,
            router,
            address,
            media_path,
        })
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting Synapse Rust Matrix Server...");
        info!("Server name: {}", self.app_state.services.server_name);
        info!("Listening on: {}", self.address);
        info!("Media storage: {}", self.media_path.display());

        hyper::Server::bind(&self.address)
            .serve(self.router.clone().into_make_service())
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        Ok(())
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutting down server...");
}

pub async fn start_server(
    database_url: &str,
    server_name: &str,
    jwt_secret: &str,
    host: &str,
    port: u16,
    media_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let address = format!("{}:{}", host, port).parse()?;
    let media_path = std::path::PathBuf::from(media_path);

    if !media_path.exists() {
        std::fs::create_dir_all(&media_path)?;
    }

    let server = SynapseServer::new(database_url, server_name, jwt_secret, address, media_path).await?;
    server.run().await
}
