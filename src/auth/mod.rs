use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use chrono::{Duration, Utc};
use anyhow::Result;

use crate::models::{User, LoginRequest, CreateUserRequest, LoginResponse, UserResponse};
use crate::db;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user id
    pub username: String,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub session_duration_hours: i64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "your-secret-key-change-this-in-production".to_string(),
            session_duration_hours: 24 * 7, // 7 days
        }
    }
}

pub fn hash_password(password: &str) -> Result<String> {
    let hashed = hash(password, DEFAULT_COST)?;
    Ok(hashed)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let is_valid = verify(password, hash)?;
    Ok(is_valid)
}

pub fn create_jwt_token(user: &User, config: &AuthConfig) -> Result<String> {
    let now = Utc::now();
    let expires_at = now + Duration::hours(config.session_duration_hours);
    
    let claims = Claims {
        sub: user.id.to_string(),
        username: user.username.clone(),
        exp: expires_at.timestamp() as usize,
        iat: now.timestamp() as usize,
    };
    
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )?;
    
    Ok(token)
}

pub fn verify_jwt_token(token: &str, config: &AuthConfig) -> Result<Claims> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &Validation::default(),
    )?;
    
    Ok(token_data.claims)
}

pub async fn authenticate_user(
    pool: &sqlx::SqlitePool,
    login_request: LoginRequest,
    config: &AuthConfig,
) -> Result<LoginResponse> {
    let user = db::get_user_by_username(pool, &login_request.username)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Invalid username or password"))?;
    
    if !verify_password(&login_request.password, &user.password_hash)? {
        return Err(anyhow::anyhow!("Invalid username or password"));
    }
    
    let token = create_jwt_token(&user, config)?;
    let expires_at = Utc::now() + Duration::hours(config.session_duration_hours);
    
    // Store session in database
    db::create_session(pool, user.id, &token, expires_at).await?;
    
    Ok(LoginResponse {
        token,
        user: user.into(),
        expires_at,
    })
}

pub async fn register_user(
    pool: &sqlx::SqlitePool,
    create_request: CreateUserRequest,
) -> Result<UserResponse> {
    let password_hash = hash_password(&create_request.password)?;
    
    let user = db::create_user(
        pool,
        &create_request.username,
        &create_request.email,
        &password_hash,
    ).await?;
    
    Ok(user.into())
}

// Middleware to require authentication
pub async fn auth_middleware(
    State(app_state): State<crate::AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "));
    
    let token = match auth_header {
        Some(token) => token,
        None => return Err(StatusCode::UNAUTHORIZED),
    };
    
    let session = match db::get_session_by_token(&app_state.pool, token).await {
        Ok(Some(session)) => session,
        _ => return Err(StatusCode::UNAUTHORIZED),
    };
    
    // Get user details
    let user = match db::get_user_by_id(&app_state.pool, session.user_id).await {
        Ok(Some(user)) => user,
        _ => return Err(StatusCode::UNAUTHORIZED),
    };
    
    // Add user to request extensions
    request.extensions_mut().insert(user);
    
    Ok(next.run(request).await)
}