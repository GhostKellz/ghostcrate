use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DownloadMetric {
    pub id: Uuid,
    pub crate_id: Uuid,
    pub version: String,
    pub user_id: Option<Uuid>,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub downloaded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CrateStatistics {
    pub crate_id: Uuid,
    pub total_downloads: i64,
    pub downloads_last_30_days: i64,
    pub downloads_last_7_days: i64,
    pub downloads_today: i64,
    pub unique_downloaders: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct RegistryStats {
    pub total_crates: i64,
    pub total_versions: i64,
    pub total_downloads: i64,
    pub total_users: i64,
    pub total_organizations: i64,
    pub downloads_last_30_days: i64,
    pub new_crates_last_30_days: i64,
    pub new_users_last_30_days: i64,
    pub storage_size_bytes: i64,
    pub top_crates: Vec<TopCrateStats>,
}

#[derive(Debug, Serialize)]
pub struct TopCrateStats {
    pub name: String,
    pub total_downloads: i64,
    pub downloads_last_30_days: i64,
    pub latest_version: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CrateAnalytics {
    pub crate_name: String,
    pub total_downloads: i64,
    pub downloads_by_version: HashMap<String, i64>,
    pub downloads_by_day: Vec<DailyDownload>,
    pub top_countries: Vec<CountryDownload>,
    pub unique_downloaders: i64,
    pub download_trend: DownloadTrend,
}

#[derive(Debug, Serialize)]
pub struct DailyDownload {
    pub date: String, // YYYY-MM-DD
    pub downloads: i64,
}

#[derive(Debug, Serialize)]
pub struct CountryDownload {
    pub country: String,
    pub downloads: i64,
}

#[derive(Debug, Serialize)]
pub struct DownloadTrend {
    pub change_percentage: f64,
    pub trend: TrendDirection,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TrendDirection {
    Up,
    Down,
    Stable,
}

#[derive(Debug, Serialize)]
pub struct UserStats {
    pub user_id: Uuid,
    pub username: String,
    pub total_crates: i64,
    pub total_downloads: i64,
    pub crates_published_last_30_days: i64,
    pub most_popular_crate: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OrganizationStats {
    pub organization_id: Uuid,
    pub name: String,
    pub total_crates: i64,
    pub total_downloads: i64,
    pub member_count: i64,
    pub most_popular_crate: Option<String>,
}

// Health check response
#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub database: HealthComponent,
    pub storage: HealthComponent,
    pub uptime_seconds: u64,
    pub memory_usage_mb: u64,
}

#[derive(Debug, Serialize)]
pub struct HealthComponent {
    pub status: ComponentStatus,
    pub response_time_ms: Option<u64>,
    pub details: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ComponentStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

// Metrics collection
#[derive(Debug)]
pub struct MetricsCollector {
    pub registry_stats: RegistryStats,
    pub request_count: u64,
    pub response_times: Vec<u64>,
    pub error_count: u64,
    pub active_connections: u64,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            registry_stats: RegistryStats {
                total_crates: 0,
                total_versions: 0,
                total_downloads: 0,
                total_users: 0,
                total_organizations: 0,
                downloads_last_30_days: 0,
                new_crates_last_30_days: 0,
                new_users_last_30_days: 0,
                storage_size_bytes: 0,
                top_crates: vec![],
            },
            request_count: 0,
            response_times: Vec::new(),
            error_count: 0,
            active_connections: 0,
        }
    }

    pub fn record_request(&mut self, response_time_ms: u64, is_error: bool) {
        self.request_count += 1;
        self.response_times.push(response_time_ms);
        
        if is_error {
            self.error_count += 1;
        }

        // Keep only last 1000 response times to prevent memory bloat
        if self.response_times.len() > 1000 {
            self.response_times.drain(0..100);
        }
    }

    pub fn average_response_time(&self) -> f64 {
        if self.response_times.is_empty() {
            0.0
        } else {
            self.response_times.iter().sum::<u64>() as f64 / self.response_times.len() as f64
        }
    }

    pub fn error_rate(&self) -> f64 {
        if self.request_count == 0 {
            0.0
        } else {
            self.error_count as f64 / self.request_count as f64
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
