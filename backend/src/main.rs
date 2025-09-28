mod auth;
mod config;
mod database;
mod routes;
mod signing;
mod state;

use std::net::SocketAddr;

use axum::Router;
use config::AppConfig;
use dotenvy::dotenv;
use tower_http::cors::{AllowMethods, AllowOrigin, CorsLayer};

use crate::database::Database;
use crate::state::AppState;

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("Application error: {err}");
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // 尝试从当前目录和父目录加载 .env 文件
    dotenv().ok();
    dotenvy::from_path("../.env").ok();

    let config = AppConfig::from_env()?;
    let api_host = config.api_host.clone();
    let api_port = config.api_port;

    // 初始化数据库
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        let current_dir = std::env::current_dir().unwrap();
        let db_path = current_dir.join("data").join("downloads.db");
        format!("sqlite:{}", db_path.to_string_lossy())
    });

    // 确保数据库目录存在
    if let Some(db_path) = database_url.strip_prefix("sqlite:") {
        let path = std::path::Path::new(db_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let database = Database::new(&database_url).await?;

    let state = AppState::new(config, database);
    let cors = build_cors_layer(state.config.as_ref());

    let app: Router = routes::create_router(state).layer(cors);

    let addr: SocketAddr = format!("{}:{}", api_host, api_port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Server running on http://{}:{}", api_host, api_port);

    axum::serve(listener, app).await?;
    Ok(())
}

fn build_cors_layer(config: &AppConfig) -> CorsLayer {
    if config.cors_allowed_origins.len() == 1 && config.cors_allowed_origins[0] == "*" {
        CorsLayer::permissive()
    } else {
        let origins: Vec<_> = config
            .cors_allowed_origins
            .iter()
            .filter_map(|origin| origin.parse().ok())
            .collect();

        let allow_origin = AllowOrigin::list(origins);
        CorsLayer::new()
            .allow_origin(allow_origin)
            .allow_methods(AllowMethods::any())
            .allow_headers(tower_http::cors::AllowHeaders::any())
    }
}
