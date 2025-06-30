use serde::{Deserialize, Serialize};
use std::env;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub storage: StorageConfig,
    pub auth: AuthConfig,
    pub github: GitHubConfig,
    pub registry: RegistryConfig,
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub environment: String,
    pub cors_origins: Vec<String>,
    pub rate_limit_requests_per_minute: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub backend: StorageBackend,
    pub local_path: String,
    pub s3: Option<S3Config>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    Local,
    S3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>, // For MinIO/custom S3 compatible
    pub access_key: String,
    pub secret_key: String,
    pub path_style: bool, // For MinIO - should be true
    pub use_ssl: bool,    // Whether to use HTTPS
    pub public_url: Option<String>, // For MinIO public access
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub session_duration_hours: i64,
    pub bcrypt_cost: u32,
    pub github_oauth: Option<GitHubOAuthConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubConfig {
    pub api_token: Option<String>,
    pub user_agent: String,
    pub rate_limit_per_hour: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    pub name: String,
    pub url: String,
    pub description: String,
    pub crates_io_mirror: CratesIoMirrorConfig,
    pub organizations_enabled: bool,
    pub public_registration: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CratesIoMirrorConfig {
    pub enabled: bool,
    pub upstream_url: String,
    pub sync_interval_hours: u32,
    pub cache_duration_hours: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub metrics_enabled: bool,
    pub health_check_enabled: bool,
    pub log_level: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                environment: "development".to_string(),
                cors_origins: vec!["*".to_string()],
                rate_limit_requests_per_minute: 60,
            },
            database: DatabaseConfig {
                url: "sqlite:data/ghostcrate.db".to_string(),
                max_connections: 10,
                min_connections: 1,
            },
            storage: StorageConfig {
                backend: StorageBackend::Local,
                local_path: "./data".to_string(),
                s3: None,
            },
            auth: AuthConfig {
                jwt_secret: "change-this-in-production".to_string(),
                session_duration_hours: 24 * 7, // 7 days
                bcrypt_cost: 12,
                github_oauth: None,
            },
            github: GitHubConfig {
                api_token: None,
                user_agent: "GhostCrate/0.2.0".to_string(),
                rate_limit_per_hour: 5000,
            },
            registry: RegistryConfig {
                name: "GhostCrate".to_string(),
                url: "http://localhost:8080".to_string(),
                description: "Self-hosted Rust crate registry".to_string(),
                crates_io_mirror: CratesIoMirrorConfig {
                    enabled: false,
                    upstream_url: "https://crates.io".to_string(),
                    sync_interval_hours: 24,
                    cache_duration_hours: 6,
                },
                organizations_enabled: true,
                public_registration: true,
            },
            monitoring: MonitoringConfig {
                metrics_enabled: true,
                health_check_enabled: true,
                log_level: "info".to_string(),
            },
        }
    }
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();

        // Server configuration
        if let Ok(host) = env::var("GHOSTCRATE_HOST") {
            config.server.host = host;
        }
        if let Ok(port) = env::var("GHOSTCRATE_PORT") {
            config.server.port = port.parse()?;
        }
        if let Ok(env) = env::var("GHOSTCRATE_ENVIRONMENT") {
            config.server.environment = env;
        }

        // Database configuration
        if let Ok(url) = env::var("DATABASE_URL") {
            config.database.url = url;
        }

        // Storage configuration
        if let Ok(backend) = env::var("STORAGE_BACKEND") {
            config.storage.backend = match backend.to_lowercase().as_str() {
                "s3" => StorageBackend::S3,
                _ => StorageBackend::Local,
            };
        }

        if let Ok(path) = env::var("STORAGE_LOCAL_PATH") {
            config.storage.local_path = path;
        }

        // S3 configuration
        if config.storage.backend == StorageBackend::S3 {
            config.storage.s3 = Some(S3Config {
                bucket: env::var("S3_BUCKET")?,
                region: env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
                endpoint: env::var("S3_ENDPOINT").ok(),
                access_key: env::var("S3_ACCESS_KEY")?,
                secret_key: env::var("S3_SECRET_KEY")?,
                path_style: env::var("S3_PATH_STYLE").unwrap_or_else(|_| "true".to_string()).parse().unwrap_or(true), // Default true for MinIO
                use_ssl: env::var("S3_USE_SSL").unwrap_or_else(|_| "true".to_string()).parse().unwrap_or(true),
                public_url: env::var("S3_PUBLIC_URL").ok(),
            });
        }

        // Auth configuration
        if let Ok(secret) = env::var("JWT_SECRET") {
            config.auth.jwt_secret = secret;
        }

        // GitHub configuration
        if let Ok(token) = env::var("GITHUB_API_TOKEN") {
            config.github.api_token = Some(token);
        }

        if let Ok(client_id) = env::var("GITHUB_CLIENT_ID") {
            if let Ok(client_secret) = env::var("GITHUB_CLIENT_SECRET") {
                config.auth.github_oauth = Some(GitHubOAuthConfig {
                    client_id,
                    client_secret,
                    redirect_url: env::var("GITHUB_REDIRECT_URL")
                        .unwrap_or_else(|_| format!("{}/auth/github/callback", config.registry.url)),
                });
            }
        }

        // Registry configuration
        if let Ok(name) = env::var("REGISTRY_NAME") {
            config.registry.name = name;
        }
        if let Ok(url) = env::var("REGISTRY_URL") {
            config.registry.url = url;
        }
        if let Ok(description) = env::var("REGISTRY_DESCRIPTION") {
            config.registry.description = description;
        }

        // Crates.io mirror configuration
        if let Ok(enabled) = env::var("CRATESIO_MIRROR_ENABLED") {
            config.registry.crates_io_mirror.enabled = enabled.parse().unwrap_or(false);
        }

        Ok(config)
    }

    pub fn is_production(&self) -> bool {
        self.server.environment == "production"
    }

    pub fn is_development(&self) -> bool {
        self.server.environment == "development"
    }
}
