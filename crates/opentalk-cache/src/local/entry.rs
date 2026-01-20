// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::time::{Duration, Instant};

use moka::Expiry;

#[derive(Debug, Clone, Copy)]
pub(super) struct Entry<V> {
    value: V,
    ttl: Option<Duration>,
}

impl<V> Entry<V> {
    pub(super) fn new(value: V) -> Self {
        Self { value, ttl: None }
    }

    pub(super) fn new_with_ttl(value: V, ttl: Duration) -> Self {
        Self {
            value,
            ttl: Some(ttl),
        }
    }

    pub(super) fn into_inner(self) -> V {
        self.value
    }
}

pub(super) struct EntryExpiry;

impl<K, V> Expiry<K, Entry<V>> for EntryExpiry {
    fn expire_after_create(
        &self,
        _key: &K,
        value: &Entry<V>,
        _created_at: Instant,
    ) -> Option<Duration> {
        value.ttl
    }
}
