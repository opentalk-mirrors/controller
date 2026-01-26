// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::{marker::PhantomData, time::Duration};

use redis::AsyncCommands as _;
use snafu::ResultExt as _;

use super::{Connection, Error, Key, Value};
use crate::{CacheStorage, Result};

pub struct Cache<K, V> {
    connection: Connection,
    prefix: String,
    ttl: Duration,
    _phantom: PhantomData<(K, V)>,
}

impl<K, V> Cache<K, V>
where
    K: Key + 'static,
    V: Value + 'static,
{
    pub fn new(connection: Connection, prefix: String, ttl: Duration) -> Self {
        Self {
            connection,
            prefix,
            ttl,
            _phantom: PhantomData,
        }
    }

    async fn get_raw(&self, key: &K) -> Result<Option<Vec<u8>>, Error> {
        let v = self
            .connection
            .clone()
            .get(super::RedisCacheKey {
                prefix: &self.prefix,
                key,
            })
            .await
            .context(super::error::RedisSnafu)?;
        Ok(v)
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
        let v = self.get_raw(key).await?;

        let Some(v) = v else {
            return Ok(None);
        };

        let v = V::decode_from_redis(&v)?;
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
                super::RedisCacheKey {
                    prefix: &self.prefix,
                    key: &key,
                },
                value.encode_for_redis()?,
                ttl.as_secs(),
            )
            .await
            .context(super::error::RedisSnafu)?;
        Ok(())
    }

    async fn invalidate(&self, key: &K) -> Result<()> {
        self.connection
            .clone()
            .del::<_, ()>(super::RedisCacheKey {
                prefix: &self.prefix,
                key: &key,
            })
            .await
            .context(super::error::RedisSnafu)?;
        Ok(())
    }
}
