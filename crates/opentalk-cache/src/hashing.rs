// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2
use std::{fmt::Display, hash::Hash, marker::PhantomData, time::Duration};

use siphasher::sip128::{Hasher128, SipHasher24};

use crate::{CacheStorage, Result};

pub struct HashedCache<K, V, C>
where
    K: Display + Hash + From<String>,
    C: CacheStorage<K, V>,
{
    inner: C,
    _phantom: PhantomData<(K, V)>,
}

impl<K, V, C> HashedCache<K, V, C>
where
    K: Display + Hash + From<String>,
    C: CacheStorage<K, V>,
{
    pub fn new(inner: C) -> Self {
        Self {
            inner,
            _phantom: PhantomData,
        }
    }

    pub fn hash_key(&self, raw: &K) -> K {
        let mut h = SipHasher24::new_with_keys(!0x113, 0x311);
        raw.hash(&mut h);
        let hashed_string = format!("{:x}", h.finish128().as_u128());
        K::from(hashed_string)
    }
}

pub trait WithHashing<K, V>: CacheStorage<K, V> + Sized {
    fn with_hashing(self) -> HashedCache<K, V, Self>
    where
        K: Display + Hash + From<String>;
}

impl<K, V, CACHE: CacheStorage<K, V>> WithHashing<K, V> for CACHE
where
    K: Display + Hash + From<String>,
{
    fn with_hashing(self) -> HashedCache<K, V, Self> {
        HashedCache::new(self)
    }
}

#[async_trait::async_trait(?Send)]
impl<K, V, C> CacheStorage<K, V> for HashedCache<K, V, C>
where
    K: Display + Hash + From<String> + 'static,
    V: Clone + 'static,
    C: CacheStorage<K, V>,
{
    fn ttl(&self) -> Duration {
        self.inner.ttl()
    }

    async fn get(&self, key: &K) -> Result<Option<V>> {
        let hashed_key = self.hash_key(key);
        self.inner.get(&hashed_key).await
    }

    async fn insert(&self, key: K, value: V) -> Result<()> {
        let hashed_key = self.hash_key(&key);
        self.inner.insert(hashed_key, value).await
    }

    async fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) -> Result<()> {
        let hashed_key = self.hash_key(&key);
        self.inner.insert_with_ttl(hashed_key, value, ttl).await
    }

    async fn invalidate(&self, key: &K) -> Result<()> {
        let hashed_key = self.hash_key(key);
        self.inner.invalidate(&hashed_key).await
    }
}
