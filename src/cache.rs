//! Cache implementation for bootstrap files

use crate::error::Result;
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// Bootstrap cache manager
pub struct Cache {
    cache_dir: PathBuf,
    ttl: Duration,
}

impl Cache {
    /// Create a new cache
    pub fn new() -> Result<Self> {
        let cache_dir = if let Some(proj_dirs) = ProjectDirs::from("org", "openrdap", "rdap") {
            proj_dirs.cache_dir().to_path_buf()
        } else {
            PathBuf::from(".rdap_cache")
        };
        
        fs::create_dir_all(&cache_dir)?;
        
        Ok(Self {
            cache_dir,
            ttl: Duration::from_secs(24 * 3600), // 24 hours
        })
    }
    
    /// Set cache TTL
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }
    
    /// Get cached file if valid
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        let path = self.cache_dir.join(key);
        
        if !path.exists() {
            return None;
        }
        
        // Check if expired
        if let Ok(metadata) = fs::metadata(&path) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                    if elapsed > self.ttl {
                        log::debug!("Cache expired for {}", key);
                        let _ = fs::remove_file(&path);
                        return None;
                    }
                }
            }
        }
        
        fs::read(&path).ok()
    }
    
    /// Save to cache
    pub fn set(&self, key: &str, data: &[u8]) -> Result<()> {
        let path = self.cache_dir.join(key);
        fs::write(&path, data)?;
        Ok(())
    }
    
    /// Clear cache
    pub fn clear(&self) -> Result<()> {
        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            if entry.path().is_file() {
                fs::remove_file(entry.path())?;
            }
        }
        Ok(())
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new().expect("Failed to create cache")
    }
}
