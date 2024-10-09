// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Interfaces and types for implementing authorization checks.

mod access;
mod admission;
mod authorizer;
mod resource;
mod subject;

pub use access::Access;
pub use admission::Admission;
pub use authorizer::Authorizer;
pub use resource::Resource;
pub use subject::Subject;
