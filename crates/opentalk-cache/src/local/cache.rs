// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::time::Duration;

use super::Entry;
use crate::{CacheStorage, Result};

pub struct Cache<K, V> {
    ttl: Duration,
    inner: moka::future::Cache<K, Entry<V>>,
}

impl<K, V> Cache<K, V>
where
    K: std::hash::Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            inner: moka::future::Cache::builder().time_to_live(ttl).build(),
        }
    }
}

#[async_trait::async_trait(?Send)]
impl<K, V> CacheStorage<K, V> for Cache<K, V>
where
    K: std::hash::Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    fn ttl(&self) -> Duration {
        self.ttl
    }

    async fn get(&self, key: &K) -> Result<Option<V>> {
        Ok(self
            .inner
            .get(key)
            .await
            .filter(|entry| entry.still_valid())
            .map(|entry| entry.into_inner()))
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
}
