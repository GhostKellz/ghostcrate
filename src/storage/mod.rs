use std::path::{PathBuf};
use anyhow::Result;
use tokio::fs;

#[cfg(feature = "ssr")]
use aws_sdk_s3::{Client as S3Client, primitives::ByteStream, config::{Credentials, Region}};

#[cfg(feature = "ssr")]
use aws_config::BehaviorVersion;

use crate::config::{StorageConfig, StorageBackend, S3Config};

#[cfg(feature = "ssr")]
pub mod s3;

#[derive(Clone)]
pub struct Storage {
    config: StorageConfig,
    #[cfg(feature = "ssr")]
    s3_client: Option<S3Client>,
}

impl Storage {
    pub fn new(config: StorageConfig) -> Result<Self> {
        Ok(Self {
            config,
            #[cfg(feature = "ssr")]
            s3_client: None,
        })
    }
    
    #[cfg(feature = "ssr")]
    pub async fn init(&mut self) -> Result<()> {
        match &self.config.backend {
            StorageBackend::Local => {
                fs::create_dir_all(&self.config.local_path).await?;
                fs::create_dir_all(format!("{}/crates", &self.config.local_path)).await?;
                tracing::info!("Local storage initialized at: {}", self.config.local_path);
            }
            StorageBackend::S3 => {
                if let Some(s3_config) = &self.config.s3 {
                    // Create credentials
                    let credentials = Credentials::new(
                        &s3_config.access_key,
                        &s3_config.secret_key,
                        None,
                        None,
                        "ghostcrate"
                    );

                    let mut config_builder = aws_config::defaults(BehaviorVersion::latest())
                        .credentials_provider(credentials)
                        .region(Region::new(s3_config.region.clone()));

                    // Handle custom endpoint (MinIO, etc.)
                    if let Some(endpoint) = &s3_config.endpoint {
                        tracing::info!("Using custom S3 endpoint: {}", endpoint);
                        config_builder = config_builder.endpoint_url(endpoint);
                    }

                    let aws_config = config_builder.load().await;
                    
                    let mut s3_config_builder = aws_sdk_s3::config::Builder::from(&aws_config);
                    
                    // Force path style for MinIO compatibility
                    if s3_config.path_style {
                        s3_config_builder = s3_config_builder.force_path_style(true);
                        tracing::debug!("Using path-style addressing for S3 requests");
                    }
                    
                    let s3_client_config = s3_config_builder.build();
                    self.s3_client = Some(S3Client::from_conf(s3_client_config));
                    
                    // Test connection
                    if let Some(client) = &self.s3_client {
                        client.head_bucket()
                            .bucket(&s3_config.bucket)
                            .send()
                            .await
                            .map_err(|e| anyhow::anyhow!("Failed to connect to S3 bucket: {}", e))?;
                        
                        tracing::info!("S3 storage initialized for bucket: {} (MinIO compatible: {})", 
                                     s3_config.bucket, s3_config.path_style);
                    }
                } else {
                    return Err(anyhow::anyhow!("S3 backend selected but no S3 configuration provided"));
                }
            }
        }
        Ok(())
    }
    
    #[cfg(not(feature = "ssr"))]
    pub async fn init(&mut self) -> Result<()> {
        Ok(())
    }
    
    pub async fn store_crate(&self, name: &str, version: &str, data: &[u8]) -> Result<String> {
        let filename = format!("{}-{}.crate", name, version);
        
        match &self.config.backend {
            StorageBackend::Local => {
                let crate_path = self.get_local_crate_path(name, version).await;
                
                if let Some(parent) = crate_path.parent() {
                    fs::create_dir_all(parent).await?;
                }
                
                fs::write(&crate_path, data).await?;
                tracing::info!("Stored crate locally: {}", crate_path.display());
                Ok(filename)
            }
            
            #[cfg(feature = "ssr")]
            StorageBackend::S3 => {
                if let (Some(s3_config), Some(client)) = (&self.config.s3, &self.s3_client) {
                    let key = format!("crates/{}/{}/{}", name, version, filename);
                    
                    client
                        .put_object()
                        .bucket(&s3_config.bucket)
                        .key(&key)
                        .body(ByteStream::from(data.to_vec()))
                        .content_type("application/x-tar")
                        .send()
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to upload to S3: {}", e))?;
                    
                    tracing::info!("Stored crate in S3: {}", key);
                    Ok(key)
                } else {
                    Err(anyhow::anyhow!("S3 client not initialized"))
                }
            }
            
            #[cfg(not(feature = "ssr"))]
            StorageBackend::S3 => {
                Err(anyhow::anyhow!("S3 storage not available in client-side builds"))
            }
        }
    }
    
    pub async fn get_crate_path(&self, name: &str, version: &str) -> PathBuf {
        match &self.config.backend {
            StorageBackend::Local => self.get_local_crate_path(name, version).await,
            StorageBackend::S3 => {
                // For S3, return a placeholder path - actual retrieval will be handled differently
                PathBuf::from(format!("s3://{}/{}", name, version))
            }
        }
    }
    
    async fn get_local_crate_path(&self, name: &str, version: &str) -> PathBuf {
        let mut path = PathBuf::from(&self.config.local_path);
        path.push("crates");
        path.push(name);
        path.push(format!("{}-{}.crate", name, version));
        path
    }
    
    #[cfg(feature = "ssr")]
    pub async fn get_crate_data(&self, name: &str, version: &str) -> Result<Vec<u8>> {
        match &self.config.backend {
            StorageBackend::Local => {
                let path = self.get_local_crate_path(name, version).await;
                let data = fs::read(path).await?;
                Ok(data)
            }
            StorageBackend::S3 => {
                if let (Some(s3_config), Some(client)) = (&self.config.s3, &self.s3_client) {
                    let key = format!("crates/{}/{}/{}-{}.crate", name, version, name, version);
                    
                    let response = client
                        .get_object()
                        .bucket(&s3_config.bucket)
                        .key(&key)
                        .send()
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to get from S3: {}", e))?;
                    
                    let data = response.body.collect().await?.into_bytes().to_vec();
                    Ok(data)
                } else {
                    Err(anyhow::anyhow!("S3 client not initialized"))
                }
            }
        }
    }
    
    #[cfg(not(feature = "ssr"))]
    pub async fn get_crate_data(&self, _name: &str, _version: &str) -> Result<Vec<u8>> {
        Err(anyhow::anyhow!("Storage operations not available in client-side builds"))
    }
    
    pub async fn crate_exists(&self, name: &str, version: &str) -> bool {
        match &self.config.backend {
            StorageBackend::Local => {
                let path = self.get_local_crate_path(name, version).await;
                path.exists()
            }
            
            #[cfg(feature = "ssr")]
            StorageBackend::S3 => {
                if let (Some(s3_config), Some(client)) = (&self.config.s3, &self.s3_client) {
                    let key = format!("crates/{}/{}/{}-{}.crate", name, version, name, version);
                    
                    client
                        .head_object()
                        .bucket(&s3_config.bucket)
                        .key(&key)
                        .send()
                        .await
                        .is_ok()
                } else {
                    false
                }
            }
            
            #[cfg(not(feature = "ssr"))]
            StorageBackend::S3 => false,
        }
    }
    
    pub async fn get_crate_size(&self, name: &str, version: &str) -> Result<u64> {
        match &self.config.backend {
            StorageBackend::Local => {
                let path = self.get_local_crate_path(name, version).await;
                let metadata = fs::metadata(path).await?;
                Ok(metadata.len())
            }
            
            #[cfg(feature = "ssr")]
            StorageBackend::S3 => {
                if let (Some(s3_config), Some(client)) = (&self.config.s3, &self.s3_client) {
                    let key = format!("crates/{}/{}/{}-{}.crate", name, version, name, version);
                    
                    let response = client
                        .head_object()
                        .bucket(&s3_config.bucket)
                        .key(&key)
                        .send()
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to get S3 object metadata: {}", e))?;
                    
                    Ok(response.content_length().unwrap_or(0) as u64)
                } else {
                    Err(anyhow::anyhow!("S3 client not initialized"))
                }
            }
            
            #[cfg(not(feature = "ssr"))]
            StorageBackend::S3 => {
                Err(anyhow::anyhow!("Storage operations not available in client-side builds"))
            }
        }
    }
    
    // Legacy compatibility method
    pub fn base_path(&self) -> &str {
        &self.config.local_path
    }

    // Public accessor methods for health checks
    pub fn config(&self) -> &StorageConfig {
        &self.config
    }

    pub fn backend(&self) -> &StorageBackend {
        &self.config.backend
    }

    pub fn local_path(&self) -> &str {
        &self.config.local_path
    }

    pub fn s3_config(&self) -> Option<&S3Config> {
        self.config.s3.as_ref()
    }
}