use axum::{
    extract::{Query, State, Path},
    http::{StatusCode, Uri},
    response::{Json, Redirect},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;
use tracing::{info, error, debug};
use anyhow::Result;

use crate::models::{
    User, LoginResponse, 
    EntraIdConfig, GitHubOidcConfig,
};
use crate::{AppState, auth, db};

#[derive(Debug, Deserialize)]
pub struct OidcAuthQuery {
    pub code: String,
    pub state: String,
    pub session_state: Option<String>, // For Entra ID
}

#[derive(Debug, Deserialize)]
pub struct OidcLoginRequest {
    pub provider: String,               // Provider name
    pub return_url: Option<String>,     // Where to redirect after auth
}

/// Initiate OIDC authentication flow
#[cfg(feature = "ssr")]
pub async fn oidc_login_handler(
    State(app_state): State<AppState>,
    Path(provider): Path<String>,
) -> Result<Redirect, StatusCode> {
    
    // Get OIDC configuration
    let oidc_config = app_state.config.auth.oidc
        .as_ref()
        .ok_or(StatusCode::NOT_IMPLEMENTED)?;

    match provider.as_str() {
        "entra" | "entraid" => {
            handle_entra_id_login(&app_state, oidc_config.entra_id.as_ref()).await
        }
        "github" => {
            handle_github_oidc_login(&app_state, oidc_config.github.as_ref()).await
        }
        _ => {
            error!("Unsupported OIDC provider: {}", provider);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// Handle OIDC callback and complete authentication
#[cfg(feature = "ssr")]
pub async fn oidc_callback_handler(
    State(app_state): State<AppState>,
    Path(provider): Path<String>,
    Query(params): Query<OidcAuthQuery>,
) -> Result<Json<LoginResponse>, StatusCode> {
    
    let oidc_config = app_state.config.auth.oidc
        .as_ref()
        .ok_or(StatusCode::NOT_IMPLEMENTED)?;

    match provider.as_str() {
        "entra" | "entraid" => {
            handle_entra_id_callback(&app_state, &params, oidc_config.entra_id.as_ref()).await
        }
        "github" => {
            handle_github_oidc_callback(&app_state, &params, oidc_config.github.as_ref()).await
        }
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

/// Handle Microsoft Entra ID login initiation
async fn handle_entra_id_login(
    _app_state: &AppState,
    entra_config: Option<&EntraIdConfig>,
) -> Result<Redirect, StatusCode> {
    let config = entra_config.ok_or(StatusCode::NOT_IMPLEMENTED)?;
    
    let auth_url = format!(
        "https://login.microsoftonline.com/{}/oauth2/v2.0/authorize?client_id={}&response_type=code&redirect_uri={}&scope={}&state={}",
        config.tenant_id,
        config.client_id,
        urlencoding::encode(&config.redirect_uri),
        config.scopes.join("%20"),
        Uuid::new_v4()
    );

    debug!("Redirecting to Entra ID OAuth: {}", auth_url);
    Ok(Redirect::permanent(&auth_url))
}

/// Handle GitHub OIDC login initiation
async fn handle_github_oidc_login(
    _app_state: &AppState,
    github_config: Option<&GitHubOidcConfig>,
) -> Result<Redirect, StatusCode> {
    let config = github_config.ok_or(StatusCode::NOT_IMPLEMENTED)?;
    
    let auth_url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope={}&state={}",
        config.client_id,
        urlencoding::encode(&config.redirect_uri),
        config.scopes.join("%20"),
        Uuid::new_v4()
    );
    
    debug!("Redirecting to GitHub OAuth: {}", auth_url);
    Ok(Redirect::permanent(&auth_url))
}

/// Handle Microsoft Entra ID callback
async fn handle_entra_id_callback(
    app_state: &AppState,
    params: &OidcAuthQuery,
    entra_config: Option<&EntraIdConfig>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let config = entra_config.ok_or(StatusCode::NOT_IMPLEMENTED)?;
    
    // Exchange code for access token
    let client = reqwest::Client::new();
    
    let token_params = [
        ("client_id", config.client_id.as_str()),
        ("client_secret", config.client_secret.as_str()),
        ("code", &params.code),
        ("grant_type", "authorization_code"),
        ("redirect_uri", &config.redirect_uri),
    ];

    let token_url = format!("https://login.microsoftonline.com/{}/oauth2/v2.0/token", config.tenant_id);
    
    let token_response = client
        .post(&token_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&token_params)
        .send()
        .await
        .map_err(|e| {
            error!("Failed to exchange Entra ID code for token: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let token_data: serde_json::Value = token_response
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let access_token = token_data["access_token"]
        .as_str()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get user info from Microsoft Graph
    let user_response = client
        .get("https://graph.microsoft.com/v1.0/me")
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let entra_user: serde_json::Value = user_response
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let email = entra_user["mail"]
        .as_str()
        .or_else(|| entra_user["userPrincipalName"].as_str())
        .unwrap_or("")
        .to_string();

    let user = create_or_update_oidc_user(
        app_state,
        &entra_user["id"].to_string(),
        "entraid",
        &email,
        entra_user["displayName"].as_str().map(|s| s.to_string()),
    ).await?;

    // Create JWT token
    let token = auth::create_jwt_token(&user, &app_state.config.auth)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let expires_at = chrono::Utc::now() + chrono::Duration::hours(app_state.config.auth.session_duration_hours);

    Ok(Json(LoginResponse {
        token,
        user: user.into(),
        expires_at,
    }))
}

/// Handle GitHub OIDC callback
async fn handle_github_oidc_callback(
    app_state: &AppState,
    params: &OidcAuthQuery,
    github_config: Option<&GitHubOidcConfig>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let config = github_config.ok_or(StatusCode::NOT_IMPLEMENTED)?;
    
    // Exchange code for access token (similar to existing GitHub handler)
    let client = reqwest::Client::new();
    
    let token_params = [
        ("client_id", config.client_id.as_str()),
        ("client_secret", config.client_secret.as_str()),
        ("code", &params.code),
    ];

    let token_response = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .header("User-Agent", "GhostCrate/0.2.0")
        .form(&token_params)
        .send()
        .await
        .map_err(|e| {
            error!("Failed to exchange GitHub code for token: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let token_data: serde_json::Value = token_response
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let access_token = token_data["access_token"]
        .as_str()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get user info from GitHub
    let user_response = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("token {}", access_token))
        .header("User-Agent", "GhostCrate/0.2.0")
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let github_user: serde_json::Value = user_response
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let email = github_user["email"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let user = create_or_update_oidc_user(
        app_state,
        &github_user["id"].to_string(),
        "github",
        &email,
        github_user["name"].as_str().map(|s| s.to_string()),
    ).await?;

    // Create JWT token
    let token = auth::create_jwt_token(&user, &app_state.config.auth)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let expires_at = chrono::Utc::now() + chrono::Duration::hours(app_state.config.auth.session_duration_hours);

    Ok(Json(LoginResponse {
        token,
        user: user.into(),
        expires_at,
    }))
}

/// Create or update user from OIDC authentication
async fn create_or_update_oidc_user(
    app_state: &AppState,
    external_id: &str,
    provider: &str,
    email: &str,
    name: Option<String>,
) -> Result<User, StatusCode> {
    // Check if user already exists with this OIDC link
    if let Ok(Some(existing_user)) = db::get_user_by_oidc_link(&app_state.pool, external_id, provider).await {
        info!("User {} logged in via OIDC ({})", existing_user.username, provider);
        return Ok(existing_user);
    }

    // Check if user exists by email
    if let Ok(Some(existing_user)) = db::get_user_by_email(&app_state.pool, email).await {
        // Link existing user to OIDC provider
        if let Err(e) = db::create_oidc_user_link(&app_state.pool, existing_user.id, external_id, provider, email, name.as_deref()).await {
            error!("Failed to create OIDC link for existing user: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
        info!("Linked existing user {} to OIDC provider {}", existing_user.username, provider);
        return Ok(existing_user);
    }

    // Create new user if auto-registration is enabled
    let username = generate_username_from_email(email);
    let user_id = Uuid::new_v4();
    
    let new_user = User {
        id: user_id,
        username: username.clone(),
        email: email.to_string(),
        password_hash: String::new(), // OIDC users don't need password
        is_admin: false,
        github_id: if provider == "github" { Some(external_id.parse().unwrap_or(0)) } else { None },
        github_username: None,
        avatar_url: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // Create user in database
    match db::create_oidc_user(&app_state.pool, &new_user).await {
        Ok(_) => {
            // Create OIDC link
            if let Err(e) = db::create_oidc_user_link(&app_state.pool, user_id, external_id, provider, email, name.as_deref()).await {
                error!("Failed to create OIDC link for new user: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
            info!("Created new user {} via OIDC ({})", username, provider);
            Ok(new_user)
        }
        Err(e) => {
            error!("Failed to create user: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Generate username from email
fn generate_username_from_email(email: &str) -> String {
    let base = email.split('@').next().unwrap_or(email);
    // Remove special characters and make lowercase
    base.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect::<String>()
        .to_lowercase()
}
