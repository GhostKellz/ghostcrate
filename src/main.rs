use axum::{
    routing::{get, post, delete},
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
    config::AppConfig,
    auth::auth_middleware,
    web::{
        auth_handlers::*, 
        cargo_handlers::*, 
        admin_handlers::{admin_dashboard_handler, admin_users_handler},
        github_handlers::*,
        organization_handlers::*,
        health_handlers::{health_handler, admin_stats_handler},
        mirror_handlers::*,
    },
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

    // Load configuration
    let config = AppConfig::from_env()?;
    info!("Configuration loaded successfully");

    // Initialize database
    let pool = initialize_database(&config.database.url).await?;
    info!("Database initialized successfully");

    // Initialize storage
    let mut storage = Storage::new(config.storage.clone())?;
    storage.init().await?;
    info!("Storage initialized successfully");
    
    // App state
    let app_state = AppState {
        pool: pool.clone(),
        config: config.clone(),
        storage,
    };

    let addr = SocketAddr::from(([127, 0, 0, 1], config.server.port));

    // Protected routes that require authentication
    let protected_routes = Router::new()
        // Cargo Registry API
        .route("/api/v1/crates/new", post(publish_handler))
        // Auth routes
        .route("/api/auth/logout", post(logout_handler))
        .route("/api/auth/me", get(me_handler))
        // Organization routes
        .route("/api/organizations", post(create_organization_handler))
        .route("/api/organizations/:org_id", get(get_organization_handler))
        .route("/api/organizations/:org_id", post(update_organization_handler))
        .route("/api/organizations/:org_id", delete(delete_organization_handler))
        .route("/api/organizations/:org_id/members", get(get_organization_members_handler))
        .route("/api/organizations/:org_id/invite", post(invite_user_handler))
        .route("/api/organizations/:org_id/remove-member/:user_id", post(remove_member_handler))
        .route("/api/organizations/invites/:invite_id/accept", post(accept_invite_handler))
        // GitHub routes
        .route("/api/github/link", get(github_link_handler))
        .route("/api/github/disconnect", post(github_disconnect_handler))
        // Admin routes
        .route("/admin", get(admin_dashboard_handler))
        .route("/admin/api/stats", get(admin_stats_handler))
        .route("/admin/api/users", get(admin_users_handler))
        .layer(middleware::from_fn_with_state(app_state.clone(), auth_middleware));

    // Build our application with routes
    let app = Router::new()
        // Root route with basic HTML
        .route("/", get(home_handler))
        // Registry configuration (required by Cargo)
        .route("/config.json", get(config_handler))
        // Health and metrics routes (public)
        .route("/health", get(health_handler))
        // Public Cargo Registry API v1
        .route("/api/v1/crates/:name/:version/download", get(download_handler))
        .route("/api/v1/crates", get(search_handler))
        .route("/api/v1/crates/:name", get(crate_info_handler))
        // Public Authentication API
        .route("/api/auth/login", post(login_handler))
        .route("/api/auth/register", post(register_handler))
        // GitHub OAuth callback (public)
        .route("/api/github/callback", get(github_callback_handler))
        // Crates.io mirror routes (public)
        .route("/api/mirror/status", get(mirror_status_handler))
        .route("/api/mirror/sync", post(start_mirror_sync_handler))
        .route("/api/mirror/search", get(proxy_crates_io_search_handler))
        .route("/api/mirror/crate/:name/:version", get(proxy_crate_download_handler))
        // Protected routes
        .merge(protected_routes)
        // Static files
        .nest_service("/static", ServeDir::new("static"))
        // State
        .with_state(app_state)
        .layer(CorsLayer::new().allow_origin(Any).allow_headers(Any).allow_methods(Any));

    info!("Starting GhostCrate v0.2.0 server on {}", addr);
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
    <title>👻️ GhostCrate v0.2.0</title>
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
        .version { background: rgba(34, 197, 94, 0.3); padding: 0.25rem 0.75rem; border-radius: 1rem; font-size: 0.875rem; }
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
        .button.secondary { background: rgba(59, 130, 246, 0.3); }
        .button.secondary:hover { background: rgba(59, 130, 246, 0.5); }
        pre { background: rgba(0,0,0,0.3); padding: 1rem; border-radius: 0.5rem; overflow-x: auto; }
        .nav { text-align: center; margin: 2rem 0; }
        .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 1rem; }
        .feature-new { color: #34d399; font-weight: bold; }
    </style>
</head>
<body>
    <div class="hero">
        <h1 class="title">👻️ GhostCrate <span class="version">v0.2.0</span></h1>
        <p class="subtitle">Production-ready self-hosted Rust crate registry</p>
        <div class="nav">
            <a href="/admin" class="button primary">🔧 Admin Panel</a>
            <a href="/health" class="button secondary">💓 Health Check</a>
            <a href="/metrics" class="button">📊 Metrics</a>
            <a href="/api/v1/crates" class="button">🔍 Search API</a>
            <a href="/config.json" class="button">⚙️ Config</a>
        </div>
    </div>
    
    <div class="box">
        <h2>🚀 Getting Started</h2>
        <p>Your production-ready GhostCrate registry is now running! Configure your Cargo to use this registry:</p>
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
        <h2>🆕 What's New in v0.2.0</h2>
        <div class="grid">
            <div>
                <h3 class="feature-new">🏢 Organizations</h3>
                <ul>
                    <li>Create and manage organizations</li>
                    <li>Team-based access control</li>
                    <li>Organization invitations</li>
                </ul>
            </div>
            <div>
                <h3 class="feature-new">☁️ S3/MinIO Storage</h3>
                <ul>
                    <li>AWS S3 support</li>
                    <li>MinIO compatibility</li>
                    <li>Scalable cloud storage</li>
                </ul>
            </div>
            <div>
                <h3 class="feature-new">🔗 GitHub Integration</h3>
                <ul>
                    <li>GitHub OAuth login</li>
                    <li>Link GitHub accounts</li>
                    <li>Repository metadata</li>
                </ul>
            </div>
            <div>
                <h3 class="feature-new">🪞 Crates.io Mirror</h3>
                <ul>
                    <li>Mirror crates.io packages</li>
                    <li>Offline package access</li>
                    <li>Hybrid registry support</li>
                </ul>
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
                    <li class="feature-new"><strong>GET</strong> /api/github/link</li>
                </ul>
            </div>
            <div>
                <h3>Organizations <span class="feature-new">NEW</span></h3>
                <ul>
                    <li><strong>GET</strong> /api/organizations</li>
                    <li><strong>POST</strong> /api/organizations</li>
                    <li><strong>GET</strong> /api/organizations/:id</li>
                    <li><strong>POST</strong> /api/organizations/:id/invite</li>
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
            <div>
                <h3>Health & Monitoring <span class="feature-new">NEW</span></h3>
                <ul>
                    <li><strong>GET</strong> /health</li>
                    <li><strong>GET</strong> /health/ready</li>
                    <li><strong>GET</strong> /metrics</li>
                    <li><strong>GET</strong> /api/system/info</li>
                </ul>
            </div>
        </div>
    </div>
    
    <div class="box">
        <h2>📋 Production Features</h2>
        <div class="grid">
            <div>
                <p>✅ Database: SQLite/PostgreSQL</p>
                <p>✅ Authentication: JWT + Sessions</p>
                <p>✅ User Management: Complete</p>
                <p class="feature-new">✅ Organizations: Team Management</p>
            </div>
            <div>
                <p>✅ Cargo Protocol: Full Support</p>
                <p>✅ Local Storage: File System</p>
                <p class="feature-new">✅ Cloud Storage: S3/MinIO</p>
                <p>✅ Admin Dashboard: Web UI</p>
            </div>
            <div>
                <p class="feature-new">✅ Health Monitoring: Endpoints</p>
                <p class="feature-new">✅ Metrics: Prometheus</p>
                <p class="feature-new">✅ GitHub OAuth: Integration</p>
                <p class="feature-new">✅ Crates.io Mirror: Hybrid</p>
            </div>
            <div>
                <p>✅ Rate Limiting: Built-in</p>
                <p>✅ CORS: Configurable</p>
                <p>✅ Security: Production Ready</p>
                <p>✅ Docker: Container Support</p>
            </div>
        </div>
    </div>
    
    <div class="box">
        <h2>🔐 First Steps</h2>
        <ol>
            <li>Register your first user: <code>curl -X POST http://localhost:8080/api/auth/register -H "Content-Type: application/json" -d '{"username":"admin","email":"admin@example.com","password":"secure_password"}'</code></li>
            <li>Login to get token: <code>curl -X POST http://localhost:8080/api/auth/login -H "Content-Type: application/json" -d '{"username":"admin","password":"secure_password"}'</code></li>
            <li>Check system health: <code>curl http://localhost:8080/health</code></li>
            <li>Access admin panel: <a href="/admin" class="button primary">Open Admin Panel</a></li>
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
