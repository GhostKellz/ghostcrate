use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubUser {
    pub id: u64,
    pub login: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: String,
    pub html_url: String,
    pub company: Option<String>,
    pub location: Option<String>,
    pub bio: Option<String>,
    pub public_repos: u32,
    pub followers: u32,
    pub following: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubOAuthToken {
    pub access_token: String,
    pub token_type: String,
    pub scope: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubRepository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub clone_url: String,
    pub ssh_url: String,
    pub default_branch: String,
    pub language: Option<String>,
    pub stars: u32,
    pub forks: u32,
    pub size: u32,
    pub is_private: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub pushed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CratesIoIndex {
    pub name: String,
    pub vers: String,
    pub deps: Vec<CratesIoDependency>,
    pub features: serde_json::Value,
    pub cksum: String,
    pub yanked: bool,
    pub links: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CratesIoDependency {
    pub name: String,
    pub req: String,
    pub features: Vec<String>,
    pub optional: bool,
    pub default_features: bool,
    pub target: Option<String>,
    pub kind: Option<String>,
    pub registry: Option<String>,
    pub package: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CratesIoCrate {
    pub id: String,
    pub name: String,
    pub updated_at: DateTime<Utc>,
    pub versions: Vec<CratesIoVersion>,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
    pub badges: Vec<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub downloads: u64,
    pub recent_downloads: Option<u64>,
    pub max_version: String,
    pub max_stable_version: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub repository: Option<String>,
    pub links: CratesIoLinks,
    pub exact_match: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CratesIoVersion {
    pub id: u64,
    pub num: String,
    pub dl_path: String,
    pub readme_path: String,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub downloads: u64,
    pub features: serde_json::Value,
    pub yanked: bool,
    pub license: Option<String>,
    pub links: CratesIoVersionLinks,
    pub crate_size: Option<u64>,
    pub published_by: Option<CratesIoUser>,
    pub audit_actions: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CratesIoLinks {
    pub version_downloads: String,
    pub versions: String,
    pub owners: String,
    pub owner_team: String,
    pub owner_user: String,
    pub reverse_dependencies: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CratesIoVersionLinks {
    pub dependencies: String,
    pub version_downloads: String,
    pub authors: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CratesIoUser {
    pub id: u64,
    pub login: String,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CratesIoSearchResponse {
    pub crates: Vec<CratesIoCrate>,
    pub meta: CratesIoMeta,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CratesIoMeta {
    pub total: u64,
    pub next_page: Option<String>,
    pub prev_page: Option<String>,
}

// Mirror configuration and status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorStatus {
    pub enabled: bool,
    pub last_sync: Option<DateTime<Utc>>,
    pub next_sync: Option<DateTime<Utc>>,
    pub sync_in_progress: bool,
    pub total_crates_mirrored: u64,
    pub total_versions_mirrored: u64,
    pub last_error: Option<String>,
    pub storage_used_bytes: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MirrorSyncRequest {
    pub force: bool,
    pub crate_names: Option<Vec<String>>, // If specified, sync only these crates
    pub max_crates: Option<u32>,          // Limit number of crates to sync
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MirrorSyncProgress {
    pub total_crates: u64,
    pub processed_crates: u64,
    pub failed_crates: u64,
    pub current_crate: Option<String>,
    pub started_at: DateTime<Utc>,
    pub estimated_completion: Option<DateTime<Utc>>,
}

// GitHub webhook events
#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubWebhookEvent {
    pub action: String,
    pub repository: GitHubRepository,
    pub sender: GitHubUser,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubRelease {
    pub id: u64,
    pub tag_name: String,
    pub name: Option<String>,
    pub body: Option<String>,
    pub draft: bool,
    pub prerelease: bool,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub assets: Vec<GitHubAsset>,
    pub html_url: String,
    pub tarball_url: String,
    pub zipball_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubAsset {
    pub id: u64,
    pub name: String,
    pub content_type: String,
    pub size: u64,
    pub download_count: u64,
    pub browser_download_url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// API client for GitHub integration
#[derive(Debug)]
pub struct GitHubApiClient {
    pub token: Option<String>,
    pub user_agent: String,
    pub client: reqwest::Client,
}

impl GitHubApiClient {
    pub fn new(token: Option<String>, user_agent: String) -> Self {
        Self {
            token,
            user_agent,
            client: reqwest::Client::new(),
        }
    }

    pub async fn get_user(&self, username: &str) -> Result<GitHubUser, reqwest::Error> {
        let url = format!("https://api.github.com/users/{}", username);
        let mut request = self.client.get(&url).header("User-Agent", &self.user_agent);

        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("token {}", token));
        }

        request.send().await?.json().await
    }

    pub async fn get_user_repos(&self, username: &str) -> Result<Vec<GitHubRepository>, reqwest::Error> {
        let url = format!("https://api.github.com/users/{}/repos", username);
        let mut request = self.client.get(&url).header("User-Agent", &self.user_agent);

        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("token {}", token));
        }

        request.send().await?.json().await
    }

    pub async fn search_repositories(&self, query: &str) -> Result<serde_json::Value, reqwest::Error> {
        let url = format!("https://api.github.com/search/repositories?q={}", query);
        let mut request = self.client.get(&url).header("User-Agent", &self.user_agent);

        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("token {}", token));
        }

        request.send().await?.json().await
    }
}
