// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::{fmt::Display, marker::PhantomData, time::Duration};

use bincode::{Decode, Encode};
use redis::AsyncCommands as _;
use serde::de::DeserializeOwned;
use snafu::ResultExt as _;

use super::{Connection, Error};
use crate::{CacheStorage, Result};

pub struct Cache<K, V> {
    connection: Connection,
    prefix: String,
    ttl: Duration,
    hash_key: bool,
    _phantom: PhantomData<(K, V)>,
}

impl<K, V> Cache<K, V>
where
    K: Display + std::hash::Hash + Eq + Send + Sync + 'static,
    V: Encode + Decode<()> + DeserializeOwned + Clone + Send + Sync + 'static,
{
    pub fn new(connection: Connection, prefix: String, ttl: Duration, hash_key: bool) -> Self {
        Self {
            connection,
            prefix,
            ttl,
            hash_key,
            _phantom: PhantomData,
        }
    }

    async fn get_raw(&self, key: &K) -> Result<Option<Vec<u8>>, Error> {
        let v = self
            .connection
            .clone()
            .get(super::CacheKey {
                prefix: &self.prefix,
                key,
                hash_key: self.hash_key,
            })
            .await
            .context(super::error::RedisSnafu)?;
        Ok(v)
    }
}

#[async_trait::async_trait(?Send)]
impl<K, V> CacheStorage<K, V> for Cache<K, V>
where
    K: Display + std::hash::Hash + Eq + Send + Sync + 'static,
    V: Encode + Decode<()> + DeserializeOwned + Clone + Send + Sync + 'static,
{
    fn ttl(&self) -> Duration {
        self.ttl
    }

    async fn get(&self, key: &K) -> Result<Option<V>> {
        let v = self.get_raw(key).await?;

        let Some(v) = v else {
            return Ok(None);
        };

        let (v, _) = bincode::decode_from_slice(&v, bincode::config::standard())
            .context(super::error::DecodeSnafu)?;
        Ok(Some(v))
    }

    async fn insert(&self, key: K, value: V) -> Result<()> {
        self.insert_with_ttl(key, value, self.ttl).await?;
        Ok(())
    }

    async fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) -> Result<()> {
        // Limit the ttl to the ttl of this [`Cache`].
        let ttl = ttl.min(self.ttl);

        self.connection
            .clone()
            .set_ex::<_, _, ()>(
                super::CacheKey {
                    prefix: &self.prefix,
                    key: &key,
                    hash_key: self.hash_key,
                },
                bincode::encode_to_vec(value, bincode::config::standard())
                    .context(super::error::EncodeSnafu)?,
                ttl.as_secs(),
            )
            .await
            .context(super::error::RedisSnafu)?;
        Ok(())
    }
}
