// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub(super) struct Entry<V> {
    value: V,
    /// Custom expiration value to work around moka's limitation to set a custom ttl for an entry
    expires_at: Option<Instant>,
}

impl<V> Entry<V> {
    pub(super) fn new(value: V) -> Self {
        Self {
            value,
            expires_at: None,
        }
    }

    pub(super) fn new_with_ttl(value: V, ttl: Duration) -> Self {
        Self {
            value,
            expires_at: Some(Instant::now() + ttl),
        }
    }

    // Check if the custom ttl has expired
    pub(super) fn still_valid(&self) -> bool {
        if let Some(exp) = self.expires_at {
            exp.saturating_duration_since(Instant::now()) > Duration::ZERO
        } else {
            true
        }
    }

    pub(super) fn into_inner(self) -> V {
        self.value
    }
}
