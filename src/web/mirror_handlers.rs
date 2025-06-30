use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use serde::{Deserialize, Serialize};
use tracing::{info, error, warn, debug};
use chrono::Utc;

use crate::models::{
    User, MirrorStatus, MirrorSyncRequest, MirrorSyncProgress, 
    CratesIoSearchResponse, CratesIoCrate, GitHubApiClient
};
use crate::{AppState, db};

#[derive(Debug, Deserialize)]
pub struct MirrorQuery {
    pub force: Option<bool>,
    pub crate_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProxyQuery {
    pub q: Option<String>,
    pub per_page: Option<u32>,
    pub page: Option<u32>,
}

#[cfg(feature = "ssr")]
pub async fn mirror_status_handler(
    State(app_state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<MirrorStatus>, StatusCode> {
    if !user.is_admin {
        return Err(StatusCode::FORBIDDEN);
    }

    if !app_state.config.registry.crates_io_mirror.enabled {
        return Ok(Json(MirrorStatus {
            enabled: false,
            last_sync: None,
            next_sync: None,
            sync_in_progress: false,
            total_crates_mirrored: 0,
            total_versions_mirrored: 0,
            last_error: None,
            storage_used_bytes: 0,
        }));
    }

    let status = get_mirror_status(&app_state).await
        .map_err(|e| {
            error!("Failed to get mirror status: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(status))
}

#[cfg(feature = "ssr")]
pub async fn start_mirror_sync_handler(
    State(app_state): State<AppState>,
    Extension(user): Extension<User>,
    Json(request): Json<MirrorSyncRequest>,
) -> Result<Json<MirrorSyncProgress>, StatusCode> {
    if !user.is_admin {
        return Err(StatusCode::FORBIDDEN);
    }

    if !app_state.config.registry.crates_io_mirror.enabled {
        return Err(StatusCode::NOT_IMPLEMENTED);
    }

    // Check if sync is already in progress
    if is_sync_in_progress(&app_state).await {
        return Err(StatusCode::CONFLICT);
    }

    info!("Starting crates.io mirror sync requested by user: {}", user.username);

    // Start the sync process in the background
    let app_state_clone = app_state.clone();
    tokio::spawn(async move {
        if let Err(e) = perform_mirror_sync(app_state_clone, request).await {
            error!("Mirror sync failed: {}", e);
            // TODO: Update sync status with error
        }
    });

    let progress = MirrorSyncProgress {
        total_crates: 0,
        processed_crates: 0,
        failed_crates: 0,
        current_crate: None,
        started_at: Utc::now(),
        estimated_completion: None,
    };

    Ok(Json(progress))
}

#[cfg(feature = "ssr")]
pub async fn mirror_sync_progress_handler(
    State(app_state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<MirrorSyncProgress>, StatusCode> {
    if !user.is_admin {
        return Err(StatusCode::FORBIDDEN);
    }

    let progress = get_sync_progress(&app_state).await
        .map_err(|e| {
            error!("Failed to get sync progress: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(progress))
}

#[cfg(feature = "ssr")]
pub async fn proxy_crates_io_search_handler(
    State(app_state): State<AppState>,
    Query(params): Query<ProxyQuery>,
) -> Result<Json<CratesIoSearchResponse>, StatusCode> {
    if !app_state.config.registry.crates_io_mirror.enabled {
        return Err(StatusCode::NOT_IMPLEMENTED);
    }

    let query = params.q.unwrap_or_default();
    let per_page = params.per_page.unwrap_or(10).min(100);
    let page = params.page.unwrap_or(1);

    debug!("Proxying crates.io search: query='{}', per_page={}, page={}", query, per_page, page);

    // First, try to serve from local mirror if we have the data
    if let Ok(local_results) = search_local_mirror(&app_state, &query, per_page, page).await {
        if !local_results.crates.is_empty() {
            info!("Served search results from local mirror");
            return Ok(Json(local_results));
        }
    }

    // Fallback to proxying crates.io
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/crates?q={}&per_page={}&page={}",
        app_state.config.registry.crates_io_mirror.upstream_url,
        urlencoding::encode(&query),
        per_page,
        page
    );

    let response = client
        .get(&url)
        .header("User-Agent", &app_state.config.github.user_agent)
        .send()
        .await
        .map_err(|e| {
            error!("Failed to proxy crates.io search: {}", e);
            StatusCode::BAD_GATEWAY
        })?;

    let search_response: CratesIoSearchResponse = response
        .json()
        .await
        .map_err(|e| {
            error!("Failed to parse crates.io search response: {}", e);
            StatusCode::BAD_GATEWAY
        })?;

    info!("Proxied search results from crates.io");
    Ok(Json(search_response))
}

#[cfg(feature = "ssr")]
pub async fn proxy_crate_download_handler(
    State(app_state): State<AppState>,
    Path((crate_name, version)): Path<(String, String)>,
) -> Result<axum::response::Response, StatusCode> {
    if !app_state.config.registry.crates_io_mirror.enabled {
        return Err(StatusCode::NOT_IMPLEMENTED);
    }

    debug!("Proxying crate download: {}-{}", crate_name, version);

    // First, check if we have it in local storage
    if app_state.storage.crate_exists(&crate_name, &version).await {
        info!("Serving crate from local mirror: {}-{}", crate_name, version);
        
        // Serve from local storage
        let data = app_state.storage.get_crate_data(&crate_name, &version).await
            .map_err(|e| {
                error!("Failed to read crate from storage: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let response = axum::response::Response::builder()
            .header("Content-Type", "application/x-tar")
            .header("Content-Disposition", format!("attachment; filename=\"{}-{}.crate\"", crate_name, version))
            .body(axum::body::Body::from(data))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        return Ok(response);
    }

    // Proxy from crates.io
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/crates/{}/{}/download",
        app_state.config.registry.crates_io_mirror.upstream_url,
        crate_name,
        version
    );

    let response = client
        .get(&url)
        .header("User-Agent", &app_state.config.github.user_agent)
        .send()
        .await
        .map_err(|e| {
            error!("Failed to proxy crate download: {}", e);
            StatusCode::BAD_GATEWAY
        })?;

    if !response.status().is_success() {
        return Err(StatusCode::NOT_FOUND);
    }

    let data = response.bytes().await
        .map_err(|e| {
            error!("Failed to read crate data from crates.io: {}", e);
            StatusCode::BAD_GATEWAY
        })?;

    // Optionally cache the crate for future requests
    if let Err(e) = app_state.storage.store_crate(&crate_name, &version, &data).await {
        warn!("Failed to cache crate locally: {}", e);
    } else {
        debug!("Cached crate locally: {}-{}", crate_name, version);
    }

    info!("Proxied crate download from crates.io: {}-{}", crate_name, version);

    let response = axum::response::Response::builder()
        .header("Content-Type", "application/x-tar")
        .header("Content-Disposition", format!("attachment; filename=\"{}-{}.crate\"", crate_name, version))
        .body(axum::body::Body::from(data))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(response)
}

async fn get_mirror_status(app_state: &AppState) -> Result<MirrorStatus, Box<dyn std::error::Error + Send + Sync>> {
    // This would typically read from a database table or redis
    // For now, return a basic status
    Ok(MirrorStatus {
        enabled: app_state.config.registry.crates_io_mirror.enabled,
        last_sync: None, // TODO: Implement status tracking
        next_sync: None,
        sync_in_progress: false,
        total_crates_mirrored: 0,
        total_versions_mirrored: 0,
        last_error: None,
        storage_used_bytes: 0,
    })
}

async fn is_sync_in_progress(_app_state: &AppState) -> bool {
    // TODO: Implement sync status tracking
    false
}

async fn get_sync_progress(_app_state: &AppState) -> Result<MirrorSyncProgress, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Implement progress tracking
    Ok(MirrorSyncProgress {
        total_crates: 0,
        processed_crates: 0,
        failed_crates: 0,
        current_crate: None,
        started_at: Utc::now(),
        estimated_completion: None,
    })
}

async fn search_local_mirror(
    _app_state: &AppState,
    _query: &str,
    _per_page: u32,
    _page: u32,
) -> Result<CratesIoSearchResponse, Box<dyn std::error::Error + Send + Sync>> {
    // TODO: Implement local mirror search
    // This would search through locally mirrored crates
    Err("Local mirror search not implemented".into())
}

async fn perform_mirror_sync(
    _app_state: AppState,
    _request: MirrorSyncRequest,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting mirror sync process");
    
    // TODO: Implement the actual sync logic
    // This would:
    // 1. Fetch the latest crates.io index
    // 2. Compare with local mirror
    // 3. Download missing/updated crates
    // 4. Update local database
    // 5. Update sync status
    
    info!("Mirror sync completed");
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn clear_mirror_cache_handler(
    State(app_state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<StatusCode, StatusCode> {
    if !user.is_admin {
        return Err(StatusCode::FORBIDDEN);
    }

    if !app_state.config.registry.crates_io_mirror.enabled {
        return Err(StatusCode::NOT_IMPLEMENTED);
    }

    // TODO: Implement cache clearing
    // This would remove all mirrored crates from storage
    
    info!("Mirror cache cleared by user: {}", user.username);
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(feature = "ssr")]
pub async fn mirror_config_handler(
    State(app_state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !user.is_admin {
        return Err(StatusCode::FORBIDDEN);
    }

    let config = serde_json::json!({
        "enabled": app_state.config.registry.crates_io_mirror.enabled,
        "upstream_url": app_state.config.registry.crates_io_mirror.upstream_url,
        "sync_interval_hours": app_state.config.registry.crates_io_mirror.sync_interval_hours,
        "cache_duration_hours": app_state.config.registry.crates_io_mirror.cache_duration_hours,
    });

    Ok(Json(config))
}
