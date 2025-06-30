use axum::{
    extract::{Query, State},
    http::{StatusCode, HeaderMap},
    response::{Json, Redirect},
    Extension,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;
use tracing::{info, error, debug};

use crate::models::{GitHubUser, GitHubOAuthToken, LoginResponse, User, UserResponse};
use crate::{AppState, db};

#[derive(Debug, Deserialize)]
pub struct GitHubAuthQuery {
    pub code: String,
    pub state: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubErrorQuery {
    pub error: String,
    pub error_description: Option<String>,
    pub error_uri: Option<String>,
}

#[cfg(feature = "ssr")]
pub async fn github_login_handler(
    State(app_state): State<AppState>,
) -> Result<Redirect, StatusCode> {
    if let Some(github_oauth) = &app_state.config.auth.github_oauth {
        let auth_url = format!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=user:email&state={}",
            github_oauth.client_id,
            urlencoding::encode(&github_oauth.redirect_url),
            Uuid::new_v4()
        );
        
        debug!("Redirecting to GitHub OAuth: {}", auth_url);
        Ok(Redirect::permanent(&auth_url))
    } else {
        error!("GitHub OAuth not configured");
        Err(StatusCode::NOT_IMPLEMENTED)
    }
}

#[cfg(feature = "ssr")]
pub async fn github_callback_handler(
    State(app_state): State<AppState>,
    Query(params): Query<GitHubAuthQuery>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let github_oauth = app_state.config.auth.github_oauth
        .as_ref()
        .ok_or(StatusCode::NOT_IMPLEMENTED)?;

    // Exchange code for access token
    let token = exchange_code_for_token(&params.code, github_oauth)
        .await
        .map_err(|e| {
            error!("Failed to exchange GitHub code for token: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get user info from GitHub
    let github_user = get_github_user(&token.access_token, &app_state.config.github.user_agent)
        .await
        .map_err(|e| {
            error!("Failed to get GitHub user info: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Check if user exists or create new user
    let user = match db::get_user_by_github_id(&app_state.pool, github_user.id as i64).await {
        Ok(Some(user)) => {
            info!("Existing GitHub user logged in: {}", github_user.login);
            user
        }
        Ok(None) => {
            // Create new user from GitHub info
            let username = ensure_unique_username(&app_state.pool, &github_user.login).await?;
            let email = github_user.email.clone().unwrap_or_else(|| {
                format!("{}@users.noreply.github.com", github_user.login)
            });

            let user = db::create_github_user(
                &app_state.pool,
                &username,
                &email,
                github_user.id as i64,
                github_user.name.as_deref(),
                Some(&github_user.avatar_url),
            ).await.map_err(|e| {
                error!("Failed to create GitHub user: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            info!("Created new user from GitHub: {}", username);
            user
        }
        Err(e) => {
            error!("Database error during GitHub login: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Create session token
    let session_token = crate::auth::generate_session_token();
    let expires_at = Utc::now() + chrono::Duration::hours(app_state.config.auth.session_duration_hours);

    db::create_session(&app_state.pool, user.id, &session_token, expires_at)
        .await
        .map_err(|e| {
            error!("Failed to create session: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(LoginResponse {
        token: session_token,
        user: user.into(),
        expires_at,
    }))
}

async fn exchange_code_for_token(
    code: &str,
    oauth_config: &crate::config::GitHubOAuthConfig,
) -> Result<GitHubOAuthToken, reqwest::Error> {
    let client = reqwest::Client::new();
    
    let params = [
        ("client_id", oauth_config.client_id.as_str()),
        ("client_secret", oauth_config.client_secret.as_str()),
        ("code", code),
    ];

    let response = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .header("User-Agent", "GhostCrate/0.2.0")
        .form(&params)
        .send()
        .await?;

    response.json::<GitHubOAuthToken>().await
}

async fn get_github_user(
    access_token: &str,
    user_agent: &str,
) -> Result<GitHubUser, reqwest::Error> {
    let client = reqwest::Client::new();
    
    let response = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("token {}", access_token))
        .header("User-Agent", user_agent)
        .send()
        .await?;

    response.json::<GitHubUser>().await
}

async fn ensure_unique_username(
    pool: &sqlx::SqlitePool,
    preferred_username: &str,
) -> Result<String, StatusCode> {
    let mut username = preferred_username.to_string();
    let mut counter = 1;

    loop {
        match db::get_user_by_username(pool, &username).await {
            Ok(None) => return Ok(username),
            Ok(Some(_)) => {
                username = format!("{}{}", preferred_username, counter);
                counter += 1;
                if counter > 100 {
                    error!("Too many attempts to find unique username for: {}", preferred_username);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
            Err(e) => {
                error!("Database error checking username uniqueness: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }
}

#[cfg(feature = "ssr")]
pub async fn github_disconnect_handler(
    State(app_state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<StatusCode, StatusCode> {
    db::disconnect_github_user(&app_state.pool, user.id)
        .await
        .map_err(|e| {
            error!("Failed to disconnect GitHub account: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!("User {} disconnected GitHub account", user.username);
    Ok(StatusCode::OK)
}

// For linking GitHub to existing account
#[cfg(feature = "ssr")]
pub async fn github_link_handler(
    State(app_state): State<AppState>,
    Extension(user): Extension<User>,
    Query(params): Query<GitHubAuthQuery>,
) -> Result<Json<UserResponse>, StatusCode> {
    let github_oauth = app_state.config.auth.github_oauth
        .as_ref()
        .ok_or(StatusCode::NOT_IMPLEMENTED)?;

    let token = exchange_code_for_token(&params.code, github_oauth)
        .await
        .map_err(|e| {
            error!("Failed to exchange GitHub code for token: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let github_user = get_github_user(&token.access_token, &app_state.config.github.user_agent)
        .await
        .map_err(|e| {
            error!("Failed to get GitHub user info: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Check if GitHub account is already linked to another user
    if let Ok(Some(_)) = db::get_user_by_github_id(&app_state.pool, github_user.id as i64).await {
        error!("GitHub account already linked to another user");
        return Err(StatusCode::CONFLICT);
    }

    // Link GitHub account to current user
    let updated_user = db::link_github_user(
        &app_state.pool,
        user.id,
        github_user.id as i64,
        github_user.name.as_deref(),
        Some(&github_user.avatar_url),
    ).await.map_err(|e| {
        error!("Failed to link GitHub account: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    info!("User {} linked GitHub account: {}", user.username, github_user.login);
    Ok(Json(updated_user.into()))
}
