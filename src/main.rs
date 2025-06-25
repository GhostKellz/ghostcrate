use axum::{
    routing::{get, post},
    Router,
    response::Html,
    middleware,
};
use std::net::SocketAddr;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use ghostcrate::{
    auth::{AuthConfig, auth_middleware},
    web::{auth_handlers::*, cargo_handlers::*, admin_handlers::*},
    db::initialize_database,
    storage::Storage,
    AppState,
};

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ghostcrate=debug,tower_http=debug,axum::rejection=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:ghostcrate.db".to_string());
    
    let pool = initialize_database(&database_url).await?;
    info!("Database initialized successfully");

    // Initialize storage
    let storage = Storage::new("./data");
    storage.init().await?;
    info!("Storage initialized successfully");

    // Auth configuration
    let auth_config = AuthConfig::default();
    
    // App state
    let app_state = AppState {
        pool: pool.clone(),
        auth_config,
        storage,
    };

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    // Protected routes that require authentication
    let protected_routes = Router::new()
        .route("/api/v1/crates/new", post(publish_handler))
        .route("/api/auth/logout", post(logout_handler))
        .route("/api/auth/me", get(me_handler))
        // Admin routes
        .route("/admin", get(admin_dashboard_handler))
        .route("/admin/api/stats", get(admin_stats_handler))
        .route("/admin/api/users", get(admin_users_handler))
        .route("/admin/api/users/:user_id", post(admin_delete_user_handler))
        .layer(middleware::from_fn_with_state(app_state.clone(), auth_middleware));

    // Build our application with routes
    let app = Router::new()
        // Root route with basic HTML
        .route("/", get(home_handler))
        // Registry configuration (required by Cargo)
        .route("/config.json", get(config_handler))
        // Public Cargo Registry API v1
        .route("/api/v1/crates/:name/:version/download", get(download_handler))
        .route("/api/v1/crates", get(search_handler))
        .route("/api/v1/crates/:name", get(crate_info_handler))
        // Public Authentication API
        .route("/api/auth/login", post(login_handler))
        .route("/api/auth/register", post(register_handler))
        // Protected routes
        .merge(protected_routes)
        // Static files
        .nest_service("/static", ServeDir::new("static"))
        // State
        .with_state(app_state)
        .layer(CorsLayer::new().allow_origin(Any).allow_headers(Any).allow_methods(Any));

    info!("Starting GhostCrate server on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;

    Ok(())
}

async fn home_handler() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>👻️ GhostCrate</title>
    <style>
        body { 
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; 
            max-width: 900px; 
            margin: 0 auto; 
            padding: 2rem; 
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            min-height: 100vh;
        }
        .hero { text-align: center; margin: 4rem 0; }
        .title { font-size: 3rem; margin-bottom: 1rem; }
        .subtitle { font-size: 1.25rem; opacity: 0.9; margin-bottom: 2rem; }
        .box { 
            background: rgba(255,255,255,0.1); 
            padding: 2rem; 
            border-radius: 1rem; 
            backdrop-filter: blur(10px);
            margin: 2rem 0;
        }
        .button { 
            background: rgba(255,255,255,0.2); 
            color: white; 
            padding: 0.75rem 2rem; 
            border: none; 
            border-radius: 0.5rem; 
            text-decoration: none; 
            margin: 0.5rem;
            display: inline-block;
        }
        .button:hover { background: rgba(255,255,255,0.3); }
        .button.primary { background: rgba(34, 197, 94, 0.3); }
        .button.primary:hover { background: rgba(34, 197, 94, 0.5); }
        pre { background: rgba(0,0,0,0.3); padding: 1rem; border-radius: 0.5rem; overflow-x: auto; }
        .nav { text-align: center; margin: 2rem 0; }
        .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 1rem; }
    </style>
</head>
<body>
    <div class="hero">
        <h1 class="title">👻️ GhostCrate</h1>
        <p class="subtitle">Self-hosted Rust crate registry & package server</p>
        <div class="nav">
            <a href="/admin" class="button primary">🔧 Admin Panel</a>
            <a href="/api/v1/crates" class="button">🔍 Search API</a>
            <a href="/config.json" class="button">⚙️ Config</a>
        </div>
    </div>
    
    <div class="box">
        <h2>🚀 Getting Started</h2>
        <p>Your GhostCrate registry is now running! Configure your Cargo to use this registry:</p>
        <pre>
[registries]
ghostcrate = { index = "http://localhost:8080" }
        </pre>
        
        <div class="grid">
            <div>
                <h3>📦 Publish a crate:</h3>
                <pre>cargo publish --registry ghostcrate</pre>
            </div>
            <div>
                <h3>📥 Install from registry:</h3>
                <pre>cargo install --registry ghostcrate your_crate</pre>
            </div>
        </div>
    </div>
    
    <div class="box">
        <h2>🔧 API Endpoints</h2>
        <div class="grid">
            <div>
                <h3>Authentication</h3>
                <ul>
                    <li><strong>POST</strong> /api/auth/register</li>
                    <li><strong>POST</strong> /api/auth/login</li>
                    <li><strong>GET</strong> /api/auth/me</li>
                </ul>
            </div>
            <div>
                <h3>Cargo Registry</h3>
                <ul>
                    <li><strong>POST</strong> /api/v1/crates/new</li>
                    <li><strong>GET</strong> /api/v1/crates</li>
                    <li><strong>GET</strong> /api/v1/crates/:name/:version/download</li>
                </ul>
            </div>
        </div>
    </div>
    
    <div class="box">
        <h2>📋 Production Status</h2>
        <div class="grid">
            <div>
                <p>✅ Database: Connected</p>
                <p>✅ Authentication: Ready</p>
                <p>✅ User Management: Ready</p>
            </div>
            <div>
                <p>✅ Cargo Protocol: Implemented</p>
                <p>✅ File Storage: Ready</p>
                <p>✅ Admin Dashboard: Ready</p>
            </div>
        </div>
    </div>
    
    <div class="box">
        <h2>🔐 First Steps</h2>
        <ol>
            <li>Register your first user: <code>curl -X POST http://localhost:8080/api/auth/register -H "Content-Type: application/json" -d '{"username":"admin","email":"admin@example.com","password":"secure_password"}'</code></li>
            <li>Login to get token: <code>curl -X POST http://localhost:8080/api/auth/login -H "Content-Type: application/json" -d '{"username":"admin","password":"secure_password"}'</code></li>
            <li>Access admin panel: <a href="/admin" class="button">Open Admin Panel</a></li>
        </ol>
    </div>
</body>
</html>
    "#)
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    println!("This binary is compiled for server-side rendering only.");
}
