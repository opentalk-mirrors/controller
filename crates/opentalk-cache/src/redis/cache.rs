// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::{fmt::Display, marker::PhantomData, time::Duration};

use bincode::{Decode, Encode};
use redis::AsyncCommands as _;
use serde::de::DeserializeOwned;

use super::{Connection, Error};

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

    pub(crate) fn ttl(&self) -> Duration {
        self.ttl
    }

    pub(crate) async fn get(&self, key: &K) -> Result<Option<V>, Error> {
        let v = self.get_raw(key).await?;

        let Some(v) = v else {
            return Ok(None);
        };

        let (v, _) = bincode::decode_from_slice(&v, bincode::config::standard())?;
        Ok(Some(v))
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
            .await?;
        Ok(v)
    }

    pub(crate) async fn insert(&self, key: &K, value: &V) -> Result<(), Error> {
        self.insert_with_ttl(key, value, self.ttl).await?;
        Ok(())
    }

    pub(crate) async fn insert_with_ttl(
        &self,
        key: &K,
        value: &V,
        ttl: Duration,
    ) -> Result<(), Error> {
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
                bincode::encode_to_vec(value, bincode::config::standard())?,
                ttl.as_secs(),
            )
            .await?;
        Ok(())
    }
}
