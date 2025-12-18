// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_database::DatabaseError;
use opentalk_db_storage::module_resources::JsonOperationError;
use opentalk_inventory_common::error::InventoryBackendError;
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum Error {
    /// An error happened in the database backend.
    Database { source: DatabaseError },

    /// An error happened during a JSON operation.
    JsonOperation { source: JsonOperationError },
}

impl Error {
    pub fn is_not_found(&self) -> bool {
        matches!(
            self,
            Error::Database {
                source: DatabaseError::NotFound,
                ..
            }
        )
    }
}

impl From<Error> for InventoryBackendError {
    fn from(e: Error) -> Self {
        let e: Box<dyn std::error::Error + Send + Sync> = Box::new(e);
        e.into()
    }
}

impl From<Error> for opentalk_inventory::Error {
    fn from(e: Error) -> Self {
        if e.is_not_found() {
            return Self::NotFound;
        }
        Self::InventoryBackend {
            source: InventoryBackendError::from(e),
        }
    }
}
