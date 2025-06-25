use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

#[derive(Clone)]
pub struct Storage {
    pub base_path: PathBuf,
}

impl Storage {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }
    
    pub async fn init(&self) -> Result<()> {
        fs::create_dir_all(&self.base_path).await?;
        fs::create_dir_all(self.base_path.join("crates")).await?;
        Ok(())
    }
    
    pub async fn store_crate(&self, name: &str, version: &str, data: &[u8]) -> Result<String> {
        let crate_dir = self.base_path.join("crates").join(name);
        fs::create_dir_all(&crate_dir).await?;
        
        let filename = format!("{}-{}.crate", name, version);
        let file_path = crate_dir.join(&filename);
        
        let mut file = fs::File::create(&file_path).await?;
        file.write_all(data).await?;
        file.flush().await?;
        
        Ok(filename)
    }
    
    pub async fn get_crate_path(&self, name: &str, version: &str) -> PathBuf {
        let filename = format!("{}-{}.crate", name, version);
        self.base_path.join("crates").join(name).join(filename)
    }
    
    pub async fn crate_exists(&self, name: &str, version: &str) -> bool {
        let path = self.get_crate_path(name, version).await;
        path.exists()
    }
    
    pub async fn get_crate_size(&self, name: &str, version: &str) -> Result<u64> {
        let path = self.get_crate_path(name, version).await;
        let metadata = fs::metadata(path).await?;
        Ok(metadata.len())
    }
}