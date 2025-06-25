use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Crate {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub repository: Option<String>,
    pub keywords: Option<String>, // JSON encoded Vec<String>
    pub categories: Option<String>, // JSON encoded Vec<String>
    pub license: Option<String>,
    pub owner_id: Uuid,
    pub downloads: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CrateVersion {
    pub id: Uuid,
    pub crate_id: Uuid,
    pub version: String,
    pub checksum: String,
    pub file_size: i64,
    pub dependencies: Option<String>, // JSON encoded Vec<Dependency>
    pub features: Option<String>, // JSON encoded HashMap<String, Vec<String>>
    pub yanked: bool,
    pub license: Option<String>,
    pub readme: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version_req: String,
    pub features: Vec<String>,
    pub optional: bool,
    pub default_features: bool,
    pub target: Option<String>,
    pub kind: DependencyKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyKind {
    Normal,
    Dev,
    Build,
}

// Cargo Registry API types
#[derive(Debug, Serialize, Deserialize)]
pub struct PublishRequest {
    pub name: String,
    pub vers: String,
    pub deps: Vec<PublishDependency>,
    pub features: HashMap<String, Vec<String>>,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub readme: Option<String>,
    pub readme_file: Option<String>,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
    pub license: Option<String>,
    pub license_file: Option<String>,
    pub repository: Option<String>,
    pub badges: HashMap<String, serde_json::Value>,
    pub links: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PublishDependency {
    pub name: String,
    pub version_req: String,
    pub features: Vec<String>,
    pub optional: bool,
    pub default_features: bool,
    pub target: Option<String>,
    pub kind: DependencyKind,
    pub registry: Option<String>,
    pub explicit_name_in_toml: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CrateResponse {
    pub id: String,
    pub name: String,
    pub updated_at: DateTime<Utc>,
    pub versions: Vec<VersionResponse>,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
    pub badges: Vec<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub downloads: i64,
    pub recent_downloads: Option<i64>,
    pub max_version: String,
    pub max_stable_version: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub repository: Option<String>,
    pub links: LinksResponse,
    pub exact_match: bool,
}

#[derive(Debug, Serialize)]
pub struct VersionResponse {
    pub id: i64,
    pub num: String,
    pub dl_path: String,
    pub readme_path: String,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub downloads: i64,
    pub features: HashMap<String, Vec<String>>,
    pub yanked: bool,
    pub license: Option<String>,
    pub links: VersionLinksResponse,
    pub crate_size: Option<i64>,
    pub published_by: Option<UserLinkResponse>,
    pub audit_actions: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct LinksResponse {
    pub version_downloads: String,
    pub versions: String,
    pub owners: String,
    pub owner_team: String,
    pub owner_user: String,
    pub reverse_dependencies: String,
}

#[derive(Debug, Serialize)]
pub struct VersionLinksResponse {
    pub dependencies: String,
    pub version_downloads: String,
    pub authors: String,
}

#[derive(Debug, Serialize)]
pub struct UserLinkResponse {
    pub id: i64,
    pub login: String,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub crates: Vec<CrateResponse>,
    pub meta: SearchMeta,
}

#[derive(Debug, Serialize)]
pub struct SearchMeta {
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct PublishResponse {
    pub warnings: PublishWarnings,
}

#[derive(Debug, Serialize)]
pub struct PublishWarnings {
    pub invalid_categories: Vec<String>,
    pub invalid_badges: Vec<String>,
    pub other: Vec<String>,
}