// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::{fmt::Display, hash::Hash};

use redis::{ToRedisArgs, ToSingleRedisArg};
use siphasher::sip128::{Hasher128, SipHasher24};

/// A Cache key with a [`ToRedisArgs`] implementation
///
/// Takes the prefix and cache key to turn them into a redis key
pub(super) struct CacheKey<'a, K> {
    pub(super) hash_key: bool,
    pub(super) prefix: &'a str,
    pub(super) key: &'a K,
}

impl<K: Display + Hash> Display for CacheKey<'_, K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.hash_key {
            let mut h = SipHasher24::new_with_keys(!0x113, 0x311);
            self.key.hash(&mut h);
            let hash = h.finish128().as_u128();

            write!(f, "opentalk-cache:{}:{:x}", self.prefix, hash)
        } else {
            write!(f, "opentalk-cache:{}:{}", self.prefix, self.key)
        }
    }
}

impl<D: Display + Hash> ToSingleRedisArg for CacheKey<'_, D> {}

impl<D: Display + Hash> ToRedisArgs for CacheKey<'_, D> {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        out.write_arg_fmt(self)
    }
}
