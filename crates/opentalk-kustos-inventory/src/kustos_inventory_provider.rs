// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use crate::{KustosInventory, Result};

/// A provider for connections to the inventory backend. Could for example be a
/// database connection pool.
#[async_trait::async_trait]
pub trait KustosInventoryProvider: Sync + Send + std::fmt::Debug {
    /// Get an instance of the inventory access handle from the provider.
    ///
    /// For inventory providers that are implemented as a database connection
    /// pool, this is typically an available database connection from the pool.
    async fn get_inventory(&self) -> Result<Box<dyn KustosInventory>>;
}
