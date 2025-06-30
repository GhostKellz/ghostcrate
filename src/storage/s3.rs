use anyhow::Result;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::{Client, Config as S3Config};
use aws_sdk_s3::config::{Credentials, SharedCredentialsProvider};
use bytes::Bytes;
use std::path::PathBuf;
use tracing::{debug, info, error};

use crate::config::S3Config as S3ConfigStruct;

pub struct S3Storage {
    client: Client,
    bucket: String,
    config: S3ConfigStruct,
}

impl S3Storage {
    pub async fn new(config: S3ConfigStruct) -> Result<Self> {
        info!("Initializing S3 storage with bucket: {}", config.bucket);
        
        // Create credentials
        let credentials = Credentials::new(
            &config.access_key,
            &config.secret_key,
            None,
            None,
            "ghostcrate"
        );

        // Build S3 config
        let mut s3_config_builder = S3Config::builder()
            .region(Region::new(config.region.clone()))
            .credentials_provider(SharedCredentialsProvider::new(credentials));

        // Configure for MinIO or custom S3 endpoint
        if let Some(ref endpoint) = config.endpoint {
            info!("Using custom S3 endpoint: {}", endpoint);
            s3_config_builder = s3_config_builder.endpoint_url(endpoint);
            
            // Force path style for MinIO compatibility
            if config.path_style {
                s3_config_builder = s3_config_builder.force_path_style(true);
                debug!("Using path-style addressing for S3 requests");
            }
        }

        let s3_config = s3_config_builder.build();
        let client = Client::from_conf(s3_config);

        let storage = Self {
            client,
            bucket: config.bucket.clone(),
            config,
        };

        // Test connection
        storage.test_connection().await?;
        
        Ok(storage)
    }

    async fn test_connection(&self) -> Result<()> {
        debug!("Testing S3 connection to bucket: {}", self.bucket);
        
        match self.client.head_bucket().bucket(&self.bucket).send().await {
            Ok(_) => {
                info!("Successfully connected to S3 bucket: {}", self.bucket);
                Ok(())
            }
            Err(e) => {
                error!("Failed to connect to S3 bucket {}: {}", self.bucket, e);
                Err(anyhow::anyhow!("S3 connection test failed: {}", e))
            }
        }
    }

    pub async fn store_crate(&self, name: &str, version: &str, data: &[u8]) -> Result<String> {
        let key = format!("crates/{}/{}-{}.crate", name, name, version);
        debug!("Storing crate to S3: {}", key);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(Bytes::from(data.to_vec()).into())
            .content_type("application/x-tar")
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to upload crate to S3: {}", e))?;

        info!("Successfully stored crate {}-{} to S3", name, version);
        Ok(key)
    }

    pub async fn get_crate(&self, name: &str, version: &str) -> Result<Vec<u8>> {
        let key = format!("crates/{}/{}-{}.crate", name, name, version);
        debug!("Retrieving crate from S3: {}", key);

        let result = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to download crate from S3: {}", e))?;

        let data = result.body.collect().await
            .map_err(|e| anyhow::anyhow!("Failed to read crate data from S3: {}", e))?
            .into_bytes()
            .to_vec();

        debug!("Successfully retrieved crate {}-{} from S3 ({} bytes)", name, version, data.len());
        Ok(data)
    }

    pub async fn crate_exists(&self, name: &str, version: &str) -> bool {
        let key = format!("crates/{}/{}-{}.crate", name, name, version);
        
        match self.client.head_object().bucket(&self.bucket).key(&key).send().await {
            Ok(_) => {
                debug!("Crate exists in S3: {}", key);
                true
            }
            Err(_) => {
                debug!("Crate does not exist in S3: {}", key);
                false
            }
        }
    }

    pub async fn get_crate_size(&self, name: &str, version: &str) -> Result<u64> {
        let key = format!("crates/{}/{}-{}.crate", name, name, version);
        
        let result = self.client
            .head_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get crate metadata from S3: {}", e))?;

        Ok(result.content_length.unwrap_or(0) as u64)
    }

    pub fn get_download_url(&self, name: &str, version: &str) -> String {
        let key = format!("crates/{}/{}-{}.crate", name, name, version);
        
        // If public URL is configured (for MinIO), use it
        if let Some(ref public_url) = self.config.public_url {
            format!("{}/{}/{}", public_url.trim_end_matches('/'), self.bucket, key)
        } else if let Some(ref endpoint) = self.config.endpoint {
            // For custom endpoints like MinIO
            let protocol = if self.config.use_ssl { "https" } else { "http" };
            if self.config.path_style {
                format!("{}{}/{}/{}", protocol, endpoint.trim_start_matches("http://").trim_start_matches("https://"), self.bucket, key)
            } else {
                format!("{}{}.{}/{}", protocol, self.bucket, endpoint.trim_start_matches("http://").trim_start_matches("https://"), key)
            }
        } else {
            // Standard AWS S3 URL
            format!("https://{}.s3.{}.amazonaws.com/{}", self.bucket, self.config.region, key)
        }
    }

    pub async fn list_crates(&self, prefix: Option<&str>) -> Result<Vec<String>> {
        let mut list_request = self.client
            .list_objects_v2()
            .bucket(&self.bucket);

        if let Some(prefix) = prefix {
            list_request = list_request.prefix(format!("crates/{}/", prefix));
        } else {
            list_request = list_request.prefix("crates/");
        }

        let result = list_request.send().await
            .map_err(|e| anyhow::anyhow!("Failed to list crates from S3: {}", e))?;

        let mut crates = Vec::new();
        if let Some(contents) = result.contents {
            for object in contents {
                if let Some(key) = object.key {
                    if key.ends_with(".crate") {
                        crates.push(key);
                    }
                }
            }
        }

        Ok(crates)
    }

    pub async fn delete_crate(&self, name: &str, version: &str) -> Result<()> {
        let key = format!("crates/{}/{}-{}.crate", name, name, version);
        debug!("Deleting crate from S3: {}", key);

        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to delete crate from S3: {}", e))?;

        info!("Successfully deleted crate {}-{} from S3", name, version);
        Ok(())
    }
}
