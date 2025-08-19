// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! This module contains the error type that is returned form the traits defined in this crate.
//!
//! The module is public so that implementors of the traits can create variants of it
//! using the types defined by [snafu], e.g. [StorageBackendSnafu].
//!
//! [snafu::whatever] can be used with this type as well if necessary.

use std::fmt::Display;

use snafu::Snafu;

/// The error returned from function calls to the storage facade provider.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    /// An error happened in the storage backend.
    StorageBackend {
        /// The cause of the error
        source: InventoryBackendError,
    },

    /// An error occurred when attempting to begin, rollback or finish a transaction.
    BrokenTransactionManager,

    /// A custom error with just a message.
    #[snafu(whatever, display("{message}"))]
    Custom {
        /// The custom error message.
        message: String,

        /// An optional source error.
        #[snafu(source(from(Box<dyn std::error::Error + Send + Sync>,Some)))]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

/// An error that can be returned from the storage backend.
#[derive(Debug)]
pub struct InventoryBackendError(Box<dyn std::error::Error + Send + Sync>);

impl From<Box<dyn std::error::Error + Send + Sync>> for InventoryBackendError {
    fn from(value: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self(value)
    }
}

impl std::error::Error for InventoryBackendError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}
impl Display for InventoryBackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<InventoryBackendError> for Error {
    fn from(source: InventoryBackendError) -> Self {
        Error::StorageBackend { source }
    }
}
