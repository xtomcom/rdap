//! Query result cache

use super::error::ApiError;
use crate::client::RdapQueryResult;
use moka::Expiry;
use moka::future::Cache;
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// "Not found" answers are cached for at most this long
const NOT_FOUND_TTL: Duration = Duration::from_secs(60);

/// (query type, normalized query). The endpoint and `source` only select a
/// view of the result, so they don't need to be part of the key.
pub type CacheKey = (String, String);

/// A cacheable answer: a result, or a definitive "not found"
#[derive(Debug, Clone)]
pub enum Cached {
    Found(Arc<RdapQueryResult>),
    NotFound(ApiError),
}

struct TtlPolicy {
    ttl: Duration,
}

impl Expiry<CacheKey, Cached> for TtlPolicy {
    fn expire_after_create(
        &self,
        _key: &CacheKey,
        value: &Cached,
        _created_at: Instant,
    ) -> Option<Duration> {
        Some(match value {
            Cached::Found(_) => self.ttl,
            Cached::NotFound(_) => self.ttl.min(NOT_FOUND_TTL),
        })
    }
}

/// Result cache; errors other than "not found" are never cached
pub struct ResultCache {
    inner: Option<Cache<CacheKey, Cached>>,
}

impl ResultCache {
    /// A zero `ttl` disables caching
    pub fn new(ttl: Duration, capacity: u64) -> Self {
        let inner = (!ttl.is_zero()).then(|| {
            Cache::builder()
                .max_capacity(capacity)
                .expire_after(TtlPolicy { ttl })
                .build()
        });
        Self { inner }
    }

    /// Return the cached answer for `key`, or run `fetch`. Concurrent misses
    /// for the same key share one `fetch`.
    pub async fn get_or_fetch<F>(&self, key: CacheKey, fetch: F) -> Result<Cached, ApiError>
    where
        F: Future<Output = Result<Cached, ApiError>> + Send,
    {
        match &self.inner {
            Some(cache) => cache
                .try_get_with(key, fetch)
                .await
                .map_err(|e| (*e).clone()),
            None => fetch.await,
        }
    }
}
