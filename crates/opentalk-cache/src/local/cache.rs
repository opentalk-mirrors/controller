// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::time::Duration;

use super::Entry;

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

    pub(crate) fn ttl(&self) -> Duration {
        self.ttl
    }

    pub(crate) async fn get(&self, key: &K) -> Option<V> {
        self.inner
            .get(key)
            .await
            .filter(|entry| entry.still_valid())
            .map(|entry| entry.into_inner())
    }

    pub(crate) async fn insert(&self, key: K, value: V) {
        self.inner.insert(key, Entry::new(value)).await
    }

    pub(crate) async fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) {
        self.inner
            .insert(key, Entry::new_with_ttl(value, ttl))
            .await
    }
}
