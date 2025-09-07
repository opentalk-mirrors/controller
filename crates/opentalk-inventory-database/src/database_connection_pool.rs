// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::sync::Arc;

use opentalk_database::Db;
use opentalk_inventory::{Inventory, InventoryProvider};
use opentalk_kustos_inventory::{KustosInventory, KustosInventoryProvider};
use snafu::ResultExt as _;

use crate::{DatabaseConnection, Result, error::DatabaseSnafu};

/// The database connection pool.
#[derive(Debug)]
pub struct DatabaseConnectionPool {
    db: Arc<Db>,
}

#[async_trait::async_trait]
impl InventoryProvider for DatabaseConnectionPool {
    #[tracing::instrument(skip_all)]
    async fn get_inventory(&self) -> Result<Box<dyn Inventory>> {
        let connection = self.get_connection().await?;
        Ok(Box::new(connection))
    }
}

#[async_trait::async_trait]
impl KustosInventoryProvider for DatabaseConnectionPool {
    #[tracing::instrument(skip_all)]
    async fn get_inventory(&self) -> Result<Box<dyn KustosInventory>> {
        let inventory: Box<dyn Inventory> = InventoryProvider::get_inventory(self).await?;
        let kustos_inventory: Box<dyn KustosInventory> = inventory;
        Ok(kustos_inventory)
    }
}

impl DatabaseConnectionPool {
    /// Create a new database connection pool wrapping a [`Db`].
    pub fn new(db: Arc<Db>) -> Self {
        Self { db }
    }

    /// Get a connection from the database pool.
    pub async fn get_connection(&self) -> Result<DatabaseConnection> {
        let inner = self.db.get_conn().await.context(DatabaseSnafu)?;
        Ok(DatabaseConnection { inner })
    }
}
