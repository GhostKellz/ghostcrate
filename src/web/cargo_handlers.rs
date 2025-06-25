use axum::{
    extract::{Path, Query, State, Multipart},
    http::{StatusCode, HeaderMap},
    response::{Json, Response},
    body::Body,
    Extension,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::Utc;
use sha2::{Sha256, Digest};
use tokio_util::io::ReaderStream;

use crate::models::{PublishRequest, PublishResponse, PublishWarnings, SearchResponse, SearchMeta, CrateResponse, User, VersionResponse, LinksResponse, VersionLinksResponse, UserLinkResponse};
use crate::{AppState, db};

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub per_page: Option<u32>,
    pub page: Option<u32>,
}

#[derive(Deserialize)]
pub struct DownloadQuery {
    // No query parameters for download currently
}

#[cfg(feature = "ssr")]
pub async fn config_handler() -> Json<serde_json::Value> {
    Json(json!({
        "dl": "http://localhost:8080/api/v1/crates/{crate}/{version}/download",
        "api": "http://localhost:8080",
        "auth-required": true
    }))
}

#[cfg(feature = "ssr")]
pub async fn publish_handler(
    State(app_state): State<AppState>,
    Extension(user): Extension<User>,
    mut multipart: Multipart,
) -> Result<Json<PublishResponse>, StatusCode> {
    let mut crate_file: Option<Vec<u8>> = None;
    let mut metadata: Option<PublishRequest> = None;

    // Parse multipart form data
    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let name = field.name().unwrap_or("").to_string();
        
        match name.as_str() {
            "crate" => {
                let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                crate_file = Some(data.to_vec());
            }
            "metadata" => {
                let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                let metadata_str = String::from_utf8(data.to_vec()).map_err(|_| StatusCode::BAD_REQUEST)?;
                metadata = Some(serde_json::from_str(&metadata_str).map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            _ => {} // Ignore unknown fields
        }
    }

    let crate_file = crate_file.ok_or(StatusCode::BAD_REQUEST)?;
    let metadata = metadata.ok_or(StatusCode::BAD_REQUEST)?;

    // Calculate checksum
    let mut hasher = Sha256::new();
    hasher.update(&crate_file);
    let checksum = format!("{:x}", hasher.finalize());

    // Store the crate file
    let _filename = app_state.storage
        .store_crate(&metadata.name, &metadata.vers, &crate_file)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Check if crate exists, create if not
    let crate_model = match db::get_crate_by_name(&app_state.pool, &metadata.name).await {
        Ok(Some(existing_crate)) => {
            // Check ownership
            if existing_crate.owner_id != user.id {
                return Err(StatusCode::FORBIDDEN);
            }
            existing_crate
        }
        Ok(None) => {
            // Create new crate
            db::create_crate(&app_state.pool, &metadata, user.id)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        }
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // Create new version
    let _version = db::create_crate_version(
        &app_state.pool,
        crate_model.id,
        &metadata,
        &checksum,
        crate_file.len() as i64,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    tracing::info!(
        "Published crate {} version {} by user {} ({} bytes, checksum: {})",
        metadata.name,
        metadata.vers,
        user.username,
        crate_file.len(),
        checksum
    );

    Ok(Json(PublishResponse {
        warnings: PublishWarnings {
            invalid_categories: vec![],
            invalid_badges: vec![],
            other: vec![],
        },
    }))
}

#[cfg(feature = "ssr")]
pub async fn download_handler(
    State(app_state): State<AppState>,
    Path((crate_name, version)): Path<(String, String)>,
) -> Result<Response<Body>, StatusCode> {
    let file_path = app_state.storage.get_crate_path(&crate_name, &version).await;
    
    if !file_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Get crate info and increment download counter
    if let Ok(Some(crate_model)) = db::get_crate_by_name(&app_state.pool, &crate_name).await {
        if let Err(e) = db::increment_download_count(&app_state.pool, crate_model.id).await {
            tracing::warn!("Failed to increment download count: {}", e);
        }
    }

    let file = tokio::fs::File::open(file_path).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);
    
    let response = Response::builder()
        .header("Content-Type", "application/x-tar")
        .header("Content-Disposition", format!("attachment; filename=\"{}-{}.crate\"", crate_name, version))
        .body(body)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    tracing::info!("Downloaded crate {} version {}", crate_name, version);
    
    Ok(response)
}

#[cfg(feature = "ssr")]
pub async fn search_handler(
    State(app_state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, StatusCode> {
    let query = params.q.unwrap_or_default();
    let per_page = params.per_page.unwrap_or(10).min(100) as i64;
    let page = params.page.unwrap_or(1) as i64;
    let offset = (page - 1) * per_page;

    let crates = db::search_crates(&app_state.pool, &query, per_page, offset)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let total = db::count_search_results(&app_state.pool, &query)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut crate_responses = Vec::new();
    for crate_model in crates {
        // Get versions for each crate
        let versions = db::get_crate_versions(&app_state.pool, crate_model.id)
            .await
            .unwrap_or_default();
        
        let version_responses: Vec<VersionResponse> = versions.iter().map(|v| {
            let features: HashMap<String, Vec<String>> = v.features
                .as_ref()
                .and_then(|f| serde_json::from_str(f).ok())
                .unwrap_or_default();
            
            VersionResponse {
                id: 1, // TODO: Use proper ID
                num: v.version.clone(),
                dl_path: format!("/api/v1/crates/{}/{}/download", crate_model.name, v.version),
                readme_path: format!("/api/v1/crates/{}/{}/readme", crate_model.name, v.version),
                updated_at: v.created_at,
                created_at: v.created_at,
                downloads: 0, // TODO: Track per-version downloads
                features,
                yanked: v.yanked,
                license: v.license.clone(),
                links: VersionLinksResponse {
                    dependencies: format!("/api/v1/crates/{}/{}/dependencies", crate_model.name, v.version),
                    version_downloads: format!("/api/v1/crates/{}/{}/downloads", crate_model.name, v.version),
                    authors: format!("/api/v1/crates/{}/{}/authors", crate_model.name, v.version),
                },
                crate_size: Some(v.file_size),
                published_by: Some(UserLinkResponse {
                    id: 1,
                    login: "user".to_string(), // TODO: Get actual user info
                    name: None,
                    avatar: None,
                    url: "/users/user".to_string(),
                }),
                audit_actions: vec![],
            }
        }).collect();
        
        let keywords: Vec<String> = crate_model.keywords
            .as_ref()
            .and_then(|k| serde_json::from_str(k).ok())
            .unwrap_or_default();
        
        let categories: Vec<String> = crate_model.categories
            .as_ref()
            .and_then(|c| serde_json::from_str(c).ok())
            .unwrap_or_default();
        
        let max_version = versions.first().map(|v| v.version.clone()).unwrap_or_default();
        
        let crate_response = CrateResponse {
            id: crate_model.id.to_string(),
            name: crate_model.name.clone(),
            updated_at: crate_model.updated_at,
            versions: version_responses,
            keywords,
            categories,
            badges: vec![],
            created_at: crate_model.created_at,
            downloads: crate_model.downloads,
            recent_downloads: Some(crate_model.downloads),
            max_version,
            max_stable_version: None,
            description: crate_model.description,
            homepage: crate_model.homepage,
            documentation: crate_model.documentation,
            repository: crate_model.repository,
            links: LinksResponse {
                version_downloads: format!("/api/v1/crates/{}/downloads", crate_model.name),
                versions: format!("/api/v1/crates/{}/versions", crate_model.name),
                owners: format!("/api/v1/crates/{}/owners", crate_model.name),
                owner_team: format!("/api/v1/crates/{}/owner_team", crate_model.name),
                owner_user: format!("/api/v1/crates/{}/owner_user", crate_model.name),
                reverse_dependencies: format!("/api/v1/crates/{}/reverse_dependencies", crate_model.name),
            },
            exact_match: crate_model.name.to_lowercase() == query.to_lowercase(),
        };
        
        crate_responses.push(crate_response);
    }

    Ok(Json(SearchResponse {
        crates: crate_responses,
        meta: SearchMeta { total },
    }))
}

#[cfg(feature = "ssr")]
pub async fn crate_info_handler(
    State(_app_state): State<AppState>,
    Path(crate_name): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: Implement crate info lookup
    Err(StatusCode::NOT_FOUND)
}