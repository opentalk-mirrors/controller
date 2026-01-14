// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Caching functionality used by the controller service.

pub mod cacheable;
mod caches;

pub use caches::Caches;
