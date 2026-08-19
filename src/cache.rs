//! Cache implementation for bootstrap files

use crate::error::Result;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// Bootstrap cache manager
pub struct Cache {
    cache_dir: PathBuf,
    ttl: Duration,
}

impl Cache {
    /// Create a new cache (~/.cache/rdap/ on all platforms)
    pub fn new() -> Result<Self> {
        let cache_dir = std::env::var("HOME").map_or_else(
            |_| PathBuf::from(".cache/rdap"),
            |h| PathBuf::from(h).join(".cache/rdap"),
        );

        fs::create_dir_all(&cache_dir)?;

        Ok(Self {
            cache_dir,
            ttl: Duration::from_hours(24),
        })
    }

    /// Set cache TTL
    pub const fn with_ttl(mut self, ttl: Duration) -> Self {
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
        if let Ok(metadata) = fs::metadata(&path)
            && let Ok(modified) = metadata.modified()
            && let Ok(elapsed) = SystemTime::now().duration_since(modified)
            && elapsed > self.ttl
        {
            log::debug!("Cache expired for {key}");
            let _ = fs::remove_file(&path);
            return None;
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
