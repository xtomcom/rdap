//! Query result cache

use super::error::ApiError;
use crate::client::RdapQueryResult;
use moka::Expiry;
use moka::future::Cache;
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// "Not found" answers, and results whose registrar referral failed, are
/// cached for at most this long
const SHORT_TTL: Duration = Duration::from_secs(60);

/// (query type, normalized query). The endpoint and `source` only select a
/// view of the result, so they don't need to be part of the key.
pub type CacheKey = (String, String);

/// A cacheable answer: a result, or a definitive "not found"
#[derive(Debug, Clone)]
pub enum Cached {
    Found(Arc<RdapQueryResult>),
    NotFound(ApiError),
}

/// Estimated memory of an entry. Each result holds the parsed models and the
/// raw JSON tree, measured at roughly 10x the JSON text size.
fn weight(value: &Cached) -> u32 {
    const OVERHEAD: usize = 1024;
    let bytes = match value {
        Cached::Found(result) => {
            let json_len = |v: &serde_json::Value| serde_json::to_vec(v).map_or(0, |b| b.len());
            let text = json_len(&result.registry_json)
                + result.registrar_json.as_ref().map_or(0, json_len);
            text.saturating_mul(10)
        }
        Cached::NotFound(_) => 0,
    };
    u32::try_from(bytes.saturating_add(OVERHEAD)).unwrap_or(u32::MAX)
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
            Cached::Found(result) if !result.registrar_failed => self.ttl,
            Cached::Found(_) | Cached::NotFound(_) => self.ttl.min(SHORT_TTL),
        })
    }
}

/// Result cache; errors other than "not found" are never cached
pub struct ResultCache {
    inner: Option<Cache<CacheKey, Cached>>,
}

impl ResultCache {
    /// A zero `ttl` disables caching. `max_bytes` bounds the estimated memory
    /// used by cached entries.
    pub fn new(ttl: Duration, max_bytes: u64) -> Self {
        let inner = (!ttl.is_zero()).then(|| {
            Cache::builder()
                .max_capacity(max_bytes)
                .weigher(|_key: &CacheKey, value: &Cached| weight(value))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{HelpResponse, RdapObject};

    fn result(json_bytes: usize) -> Cached {
        let json = serde_json::json!({ "pad": "x".repeat(json_bytes) });
        let help: HelpResponse = serde_json::from_value(serde_json::json!({})).unwrap();
        Cached::Found(Arc::new(RdapQueryResult {
            registry: RdapObject::Help(help),
            registry_url: "https://rdap.example/".parse().unwrap(),
            registrar: None,
            registrar_url: None,
            registry_json: json,
            registrar_json: None,
            registrar_failed: false,
        }))
    }

    #[tokio::test]
    async fn cache_is_bounded_by_size_not_count() {
        // ~100 KB estimated per entry, 1 MB budget -> about 10 entries survive
        let cache = ResultCache::new(Duration::from_secs(60), 1024 * 1024);
        for i in 0..100 {
            let key = ("ip".to_owned(), format!("192.0.2.{i}"));
            cache
                .get_or_fetch(key, async { Ok(result(10_000)) })
                .await
                .unwrap();
        }
        let inner = cache.inner.as_ref().unwrap();
        inner.run_pending_tasks().await;
        assert!(
            inner.weighted_size() <= 1024 * 1024,
            "{}",
            inner.weighted_size()
        );
        assert!(inner.entry_count() <= 11, "{}", inner.entry_count());
    }
}
