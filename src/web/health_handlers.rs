use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    Extension,
};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, error};

use crate::models::{
    User, HealthStatus, HealthComponent, ComponentStatus, RegistryStats, 
    MetricsCollector, TopCrateStats
};
use crate::{AppState, db};

#[cfg(feature = "ssr")]
pub async fn health_handler(
    State(app_state): State<AppState>,
) -> Result<Json<HealthStatus>, StatusCode> {
    let start_time = std::time::Instant::now();

    // Test database connection
    let db_start = std::time::Instant::now();
    let database_status = match sqlx::query("SELECT 1").fetch_one(&app_state.pool).await {
        Ok(_) => HealthComponent {
            status: ComponentStatus::Healthy,
            response_time_ms: Some(db_start.elapsed().as_millis() as u64),
            details: Some("Database connection successful".to_string()),
        },
        Err(e) => {
            error!("Database health check failed: {}", e);
            HealthComponent {
                status: ComponentStatus::Unhealthy,
                response_time_ms: Some(db_start.elapsed().as_millis() as u64),
                details: Some(format!("Database error: {}", e)),
            }
        }
    };

    // Test storage
    let storage_start = std::time::Instant::now();
    let storage_status = match test_storage_health(&app_state).await {
        Ok(_) => HealthComponent {
            status: ComponentStatus::Healthy,
            response_time_ms: Some(storage_start.elapsed().as_millis() as u64),
            details: Some("Storage accessible".to_string()),
        },
        Err(e) => {
            error!("Storage health check failed: {}", e);
            HealthComponent {
                status: ComponentStatus::Unhealthy,
                response_time_ms: Some(storage_start.elapsed().as_millis() as u64),
                details: Some(format!("Storage error: {}", e)),
            }
        }
    };

    // Determine overall status
    let overall_status = if matches!(database_status.status, ComponentStatus::Healthy) 
        && matches!(storage_status.status, ComponentStatus::Healthy) {
        "healthy"
    } else {
        "unhealthy"
    };

    // Get system metrics
    let uptime = get_uptime_seconds();
    let memory_usage = get_memory_usage_mb();

    let health_status = HealthStatus {
        status: overall_status.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: database_status,
        storage: storage_status,
        uptime_seconds: uptime,
        memory_usage_mb: memory_usage,
    };

    info!("Health check completed in {}ms", start_time.elapsed().as_millis());
    Ok(Json(health_status))
}

#[cfg(feature = "ssr")]
pub async fn readiness_handler(
    State(app_state): State<AppState>,
) -> Result<StatusCode, StatusCode> {
    // Simple readiness check - just verify database is accessible
    match sqlx::query("SELECT 1").fetch_one(&app_state.pool).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

#[cfg(feature = "ssr")]
pub async fn liveness_handler() -> StatusCode {
    // Simple liveness check - if this handler runs, the service is alive
    StatusCode::OK
}

#[cfg(feature = "ssr")]
pub async fn metrics_handler(
    State(app_state): State<AppState>,
) -> Result<Json<RegistryStats>, StatusCode> {
    let stats = gather_registry_stats(&app_state).await
        .map_err(|e| {
            error!("Failed to gather registry stats: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(stats))
}

#[cfg(feature = "ssr")]
pub async fn prometheus_metrics_handler(
    State(_app_state): State<AppState>,
) -> Result<String, StatusCode> {
    // TODO: Implement Prometheus metrics export
    // For now, return basic metrics in Prometheus format
    let metrics = format!(
        "# HELP ghostcrate_info Information about GhostCrate instance\n\
         # TYPE ghostcrate_info gauge\n\
         ghostcrate_info{{version=\"{}\"}} 1\n\
         \n\
         # HELP ghostcrate_uptime_seconds Uptime of the service in seconds\n\
         # TYPE ghostcrate_uptime_seconds counter\n\
         ghostcrate_uptime_seconds {}\n",
        env!("CARGO_PKG_VERSION"),
        get_uptime_seconds()
    );

    Ok(metrics)
}

#[cfg(feature = "ssr")]
pub async fn admin_stats_handler(
    State(app_state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<RegistryStats>, StatusCode> {
    // Check if user is admin
    if !user.is_admin {
        return Err(StatusCode::FORBIDDEN);
    }

    let stats = gather_registry_stats(&app_state).await
        .map_err(|e| {
            error!("Failed to gather admin stats: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(stats))
}

async fn test_storage_health(app_state: &AppState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // For local storage, check if directory is writable
    // For S3, this would test the connection
    match app_state.storage.backend() {
        crate::config::StorageBackend::Local => {
            let test_path = std::path::Path::new(app_state.storage.local_path());
            if !test_path.exists() {
                return Err("Storage directory does not exist".into());
            }
            
            // Try to create a test file
            let test_file = test_path.join(".health_check");
            tokio::fs::write(&test_file, "health_check").await?;
            tokio::fs::remove_file(&test_file).await?;
            
            Ok(())
        }
        #[cfg(feature = "ssr")]
        crate::config::StorageBackend::S3 => {
            // For S3, we could do a lightweight operation like listing objects with limit 1
            // For now, just assume it's healthy if configured
            if app_state.storage.s3_config().is_some() {
                Ok(())
            } else {
                Err("S3 not configured".into())
            }
        }
    }
}

async fn gather_registry_stats(app_state: &AppState) -> Result<RegistryStats, anyhow::Error> {
    // Get basic counts
    let total_crates = db::count_total_crates(&app_state.pool).await?;
    let total_versions = db::count_total_versions(&app_state.pool).await?;
    let total_downloads = db::count_total_downloads(&app_state.pool).await?;
    let total_users = db::count_total_users(&app_state.pool).await?;
    
    // Organization count (if enabled)
    let total_organizations = if app_state.config.registry.organizations_enabled {
        db::count_total_organizations(&app_state.pool).await.unwrap_or(0)
    } else {
        0
    };

    // Recent activity
    let downloads_last_30_days = db::count_downloads_last_days(&app_state.pool, 30).await.unwrap_or(0);
    let new_crates_last_30_days = db::count_new_crates_last_days(&app_state.pool, 30).await.unwrap_or(0);
    let new_users_last_30_days = db::count_new_users_last_days(&app_state.pool, 30).await.unwrap_or(0);

    // Storage size (approximate)
    let storage_size_bytes = estimate_storage_size(&app_state).await.unwrap_or(0);

    // Top crates
    let top_crates = db::get_top_crates(&app_state.pool, 10).await.unwrap_or_default();

    Ok(RegistryStats {
        total_crates,
        total_versions,
        total_downloads,
        total_users,
        total_organizations,
        downloads_last_30_days,
        new_crates_last_30_days,
        new_users_last_30_days,
        storage_size_bytes,
        top_crates,
    })
}

async fn estimate_storage_size(app_state: &AppState) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
    match app_state.storage.backend() {
        crate::config::StorageBackend::Local => {
            let storage_path = std::path::Path::new(app_state.storage.local_path());
            get_directory_size(storage_path).await
        }
        #[cfg(feature = "ssr")]
        crate::config::StorageBackend::S3 => {
            // For S3, this would require listing all objects and summing their sizes
            // For now, return 0 as a placeholder
            Ok(0)
        }
    }
}

fn get_directory_size(path: &std::path::Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<i64, Box<dyn std::error::Error + Send + Sync>>> + Send + '_>> {
    Box::pin(async move {
        let mut total_size = 0i64;
        let mut entries = tokio::fs::read_dir(path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            if metadata.is_file() {
                total_size += metadata.len() as i64;
            } else if metadata.is_dir() {
                total_size += get_directory_size(&entry.path()).await?;
            }
        }
        
        Ok(total_size)
    })
}

fn get_uptime_seconds() -> u64 {
    // This is a simple implementation - in a real application, you'd track the start time
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn get_memory_usage_mb() -> u64 {
    // Basic memory usage - on Linux you could read from /proc/self/status
    // For now, return a placeholder
    0
}

#[cfg(feature = "ssr")]
pub async fn system_info_handler(
    Extension(user): Extension<User>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !user.is_admin {
        return Err(StatusCode::FORBIDDEN);
    }

    let info = json!({
        "version": env!("CARGO_PKG_VERSION"),
        "rust_version": std::env::var("RUSTC_VERSION").unwrap_or_else(|_| "unknown".to_string()),
        "platform": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "uptime_seconds": get_uptime_seconds(),
        "memory_usage_mb": get_memory_usage_mb(),
    });

    Ok(Json(info))
}
