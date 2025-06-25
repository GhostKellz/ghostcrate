use axum::{
    extract::{Query, State, Path},
    http::StatusCode,
    response::{Json, Html},
    Extension,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::Row;

use crate::models::{User, UserResponse};
use crate::AppState;

#[derive(Deserialize)]
pub struct AdminQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Serialize)]
pub struct AdminStats {
    pub total_users: i64,
    pub total_crates: i64,
    pub total_downloads: i64,
    pub recent_users: Vec<UserResponse>,
}

#[cfg(feature = "ssr")]
pub async fn admin_dashboard_handler(
    Extension(user): Extension<User>,
) -> Result<Html<String>, StatusCode> {
    // Check if user is admin
    if !user.is_admin {
        return Err(StatusCode::FORBIDDEN);
    }

    let html = format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>GhostCrate Admin Dashboard</title>
    <style>
        body {{ 
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            max-width: 1200px;
            margin: 0 auto;
            padding: 2rem;
            background: #f8fafc;
        }}
        .header {{ background: #1e293b; color: white; padding: 2rem; border-radius: 1rem; margin-bottom: 2rem; }}
        .title {{ font-size: 2rem; margin: 0; }}
        .subtitle {{ opacity: 0.9; margin-top: 0.5rem; }}
        .grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 1.5rem; }}
        .card {{ 
            background: white; 
            padding: 2rem; 
            border-radius: 1rem; 
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
            border-left: 4px solid #6366f1;
        }}
        .stat {{ font-size: 2rem; font-weight: bold; color: #6366f1; }}
        .stat-label {{ color: #64748b; font-size: 0.875rem; margin-top: 0.5rem; }}
        .nav {{ background: white; padding: 1rem 2rem; border-radius: 1rem; margin-bottom: 2rem; }}
        .nav a {{ 
            color: #6366f1; 
            text-decoration: none; 
            margin-right: 2rem; 
            font-weight: 500;
        }}
        .nav a:hover {{ text-decoration: underline; }}
        table {{ width: 100%; border-collapse: collapse; margin-top: 1rem; }}
        th, td {{ padding: 1rem; text-align: left; border-bottom: 1px solid #e2e8f0; }}
        th {{ background: #f8fafc; font-weight: 600; }}
        .badge {{ 
            background: #f0f9ff; 
            color: #0369a1; 
            padding: 0.25rem 0.75rem; 
            border-radius: 9999px; 
            font-size: 0.75rem;
            font-weight: 500;
        }}
        .badge.admin {{ background: #fef3c7; color: #92400e; }}
    </style>
</head>
<body>
    <div class="header">
        <h1 class="title">👻️ GhostCrate Admin</h1>
        <p class="subtitle">Welcome back, {}</p>
    </div>
    
    <div class="nav">
        <a href="/admin">Dashboard</a>
        <a href="/admin/users">Users</a>
        <a href="/admin/crates">Crates</a>
        <a href="/admin/logs">Logs</a>
        <a href="/">← Back to Registry</a>
    </div>
    
    <div class="grid">
        <div class="card">
            <div class="stat" id="total-users">Loading...</div>
            <div class="stat-label">Total Users</div>
        </div>
        
        <div class="card">
            <div class="stat" id="total-crates">Loading...</div>
            <div class="stat-label">Total Crates</div>
        </div>
        
        <div class="card">
            <div class="stat" id="total-downloads">Loading...</div>
            <div class="stat-label">Total Downloads</div>
        </div>
        
        <div class="card">
            <div class="stat">Active</div>
            <div class="stat-label">Registry Status</div>
        </div>
    </div>
    
    <div class="card" style="margin-top: 2rem;">
        <h2>Recent Users</h2>
        <table id="users-table">
            <thead>
                <tr>
                    <th>Username</th>
                    <th>Email</th>
                    <th>Role</th>
                    <th>Created</th>
                </tr>
            </thead>
            <tbody>
                <tr><td colspan="4">Loading...</td></tr>
            </tbody>
        </table>
    </div>
    
    <script>
        // Load admin stats
        fetch('/admin/api/stats')
            .then(r => r.json())
            .then(data => {{
                document.getElementById('total-users').textContent = data.total_users;
                document.getElementById('total-crates').textContent = data.total_crates;
                document.getElementById('total-downloads').textContent = data.total_downloads;
                
                const tbody = document.querySelector('#users-table tbody');
                tbody.innerHTML = data.recent_users.map(user => `
                    <tr>
                        <td>${{user.username}}</td>
                        <td>${{user.email}}</td>
                        <td><span class="badge ${{user.is_admin ? 'admin' : ''}}">${{user.is_admin ? 'Admin' : 'User'}}</span></td>
                        <td>${{new Date(user.created_at).toLocaleDateString()}}</td>
                    </tr>
                `).join('');
            }})
            .catch(err => console.error('Failed to load stats:', err));
    </script>
</body>
</html>
    "#, user.username);

    Ok(Html(html))
}

#[cfg(feature = "ssr")]
pub async fn admin_stats_handler(
    State(app_state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<AdminStats>, StatusCode> {
    // Check if user is admin
    if !user.is_admin {
        return Err(StatusCode::FORBIDDEN);
    }

    // Get total users
    let total_users = sqlx::query("SELECT COUNT(*) as count FROM users")
        .fetch_one(&app_state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .get::<i64, _>("count");

    // Get total crates
    let total_crates = sqlx::query("SELECT COUNT(*) as count FROM crates")
        .fetch_one(&app_state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .get::<i64, _>("count");

    // Get total downloads
    let total_downloads = sqlx::query("SELECT COALESCE(SUM(downloads), 0) as total FROM crates")
        .fetch_one(&app_state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .get::<i64, _>("total");

    // Get recent users
    let recent_users_rows = sqlx::query(
        "SELECT id, username, email, password_hash, is_admin, created_at, updated_at FROM users ORDER BY created_at DESC LIMIT 10"
    )
    .fetch_all(&app_state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut recent_users = Vec::new();
    for row in recent_users_rows {
        let user = User {
            id: Uuid::parse_str(&row.get::<String, _>("id")).unwrap(),
            username: row.get("username"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            is_admin: row.get("is_admin"),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at")).unwrap().with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at")).unwrap().with_timezone(&chrono::Utc),
        };
        recent_users.push(user.into());
    }

    Ok(Json(AdminStats {
        total_users,
        total_crates,
        total_downloads,
        recent_users,
    }))
}

#[cfg(feature = "ssr")]
pub async fn admin_users_handler(
    State(app_state): State<AppState>,
    Extension(user): Extension<User>,
    Query(params): Query<AdminQuery>,
) -> Result<Json<Vec<UserResponse>>, StatusCode> {
    // Check if user is admin
    if !user.is_admin {
        return Err(StatusCode::FORBIDDEN);
    }

    let per_page = params.per_page.unwrap_or(50).min(100) as i64;
    let page = params.page.unwrap_or(1) as i64;
    let offset = (page - 1) * per_page;

    let rows = sqlx::query(
        "SELECT id, username, email, password_hash, is_admin, created_at, updated_at FROM users ORDER BY created_at DESC LIMIT ?1 OFFSET ?2"
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(&app_state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut users = Vec::new();
    for row in rows {
        let user = User {
            id: Uuid::parse_str(&row.get::<String, _>("id")).unwrap(),
            username: row.get("username"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            is_admin: row.get("is_admin"),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at")).unwrap().with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("updated_at")).unwrap().with_timezone(&chrono::Utc),
        };
        users.push(user.into());
    }

    Ok(Json(users))
}

#[cfg(feature = "ssr")]
pub async fn admin_delete_user_handler(
    State(app_state): State<AppState>,
    Extension(user): Extension<User>,
    Path(user_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    // Check if user is admin
    if !user.is_admin {
        return Err(StatusCode::FORBIDDEN);
    }

    // Prevent self-deletion
    if user.id == user_id {
        return Err(StatusCode::BAD_REQUEST);
    }

    sqlx::query("DELETE FROM users WHERE id = ?1")
        .bind(user_id.to_string())
        .execute(&app_state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}