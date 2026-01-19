// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::time::Duration;

use super::{Entry, EntryExpiry, Key, Value};
use crate::{CacheStorage, Result};

pub struct Cache<K, V> {
    ttl: Duration,
    inner: moka::future::Cache<K, Entry<V>>,
}

impl<K, V> Cache<K, V>
where
    K: Key + 'static,
    V: Value + 'static,
{
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            inner: moka::future::Cache::builder()
                .time_to_live(ttl)
                .expire_after(EntryExpiry)
                .build(),
        }
    }
}

#[async_trait::async_trait(?Send)]
impl<K, V> CacheStorage<K, V> for Cache<K, V>
where
    K: Key + 'static,
    V: Value + 'static,
{
    fn ttl(&self) -> Duration {
        self.ttl
    }

    async fn get(&self, key: &K) -> Result<Option<V>> {
        Ok(self.inner.get(key).await.map(|entry| entry.into_inner()))
    }

    async fn insert(&self, key: K, value: V) -> Result<()> {
        self.inner.insert(key, Entry::new(value)).await;
        Ok(())
    }

    async fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) -> Result<()> {
        self.inner
            .insert(key, Entry::new_with_ttl(value, ttl))
            .await;
        Ok(())
    }

    async fn invalidate(&self, key: &K) -> Result<()> {
        self.inner.invalidate(key).await;
        Ok(())
    }
}

/// TODO: We could make these tests more generic and move them up to the lib level
///       to test all CacheStorage implementations (redis, local)
#[cfg(test)]
mod tests {
    use tokio;

    use super::*;

    const DEFAULT_TTL: Duration = Duration::from_millis(300);
    const CUSTOM_SHORTER_TTL: Duration = Duration::from_millis(100);
    const CUSTOM_LONGER_TTL: Duration = Duration::from_millis(500);

    async fn setup() -> (Cache<String, String>, String, String) {
        let cache = Cache::new(DEFAULT_TTL);
        let key = String::from("key");
        let value = String::from("value");

        cache.insert(key.clone(), value.clone()).await.unwrap();
        (cache, key, value)
    }

    async fn setup_with_entry_ttl(ttl: Duration) -> (Cache<String, String>, String, String) {
        let cache = Cache::new(DEFAULT_TTL);
        let key = String::from("key");
        let value = String::from("value");

        cache
            .insert_with_ttl(key.clone(), value.clone(), ttl)
            .await
            .unwrap();
        (cache, key, value)
    }

    #[tokio::test]
    async fn insertion() {
        let (cache, original_key, original_value) = setup().await;

        let cached_value = cache.get(&original_key).await.unwrap();
        assert_eq!(Some(original_value), cached_value);
    }

    #[tokio::test]
    async fn invalidation() {
        let (cache, original_key, _original_value) = setup().await;

        cache.invalidate(&original_key).await.unwrap();
        let cached_value = cache.get(&original_key).await.unwrap();
        assert_eq!(cached_value, None);
    }

    #[tokio::test]
    async fn expiration_with_default_ttl() {
        let (cache, original_key, _original_value) = setup().await;

        tokio::time::sleep(DEFAULT_TTL).await;

        let cached_value = cache.get(&original_key).await.unwrap();
        assert_eq!(cached_value, None);
    }

    #[tokio::test]
    async fn expiration_with_shorter_ttl() {
        let (cache, original_key, _original_value) = setup_with_entry_ttl(CUSTOM_SHORTER_TTL).await;

        tokio::time::sleep(CUSTOM_SHORTER_TTL).await;

        let cached_value = cache.get(&original_key).await.unwrap();
        assert_eq!(cached_value, None);
    }

    #[tokio::test]
    async fn expiration_with_longer_ttl() {
        let (cache, original_key, _original_value) = setup_with_entry_ttl(CUSTOM_LONGER_TTL).await;

        tokio::time::sleep(DEFAULT_TTL).await;

        let cached_value = cache.get(&original_key).await.unwrap();
        assert_eq!(cached_value, None);
    }
}
