use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    Extension,
};

use crate::auth::{authenticate_user, register_user};
use crate::models::{LoginRequest, CreateUserRequest, LoginResponse, UserResponse};

#[cfg(feature = "ssr")]
pub async fn login_handler(
    State(app_state): State<crate::AppState>,
    Json(login_request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    match authenticate_user(&app_state.pool, login_request, &app_state.config.auth).await {
        Ok(response) => Ok(Json(response)),
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}

#[cfg(feature = "ssr")]
pub async fn register_handler(
    State(app_state): State<crate::AppState>,
    Json(create_request): Json<CreateUserRequest>,
) -> Result<Json<UserResponse>, StatusCode> {
    if !app_state.config.registry.public_registration {
        return Err(StatusCode::FORBIDDEN);
    }

    match register_user(&app_state.pool, create_request, &app_state.config.auth).await {
        Ok(user) => Ok(Json(user)),
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}


pub async fn logout_handler(
    State(_app_state): State<crate::AppState>,
    Extension(_user): Extension<crate::models::User>,
) -> Result<StatusCode, StatusCode> {
    // TODO: Invalidate the specific token by deleting from sessions table
    // For now, we'll just return success
    Ok(StatusCode::OK)
}

pub async fn me_handler(
    State(_app_state): State<crate::AppState>,
    Extension(user): Extension<crate::models::User>,
) -> Result<Json<UserResponse>, StatusCode> {
    Ok(Json(user.into()))
}