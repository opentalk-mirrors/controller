// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Synchronization for OpenTalk Controller WebAPI endpoint authorization

#![deny(
    bad_style,
    missing_debug_implementations,
    missing_docs,
    overflowing_literals,
    patterns_in_fns_without_body,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_results
)]

mod opentalk_synchronized_authorizer_backend;
mod synchronization_error;
mod synchronizer;
mod synchronizer_backend;

pub use opentalk_synchronized_authorizer_backend::OpenTalkSynchronizedAuthorizerBackend;
pub use synchronization_error::SynchronizationError;
pub use synchronizer::Synchronizer;
pub use synchronizer_backend::SynchronizerBackend;
