// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

pub use crate::tables::{
    event_email_invites as email_invites,
    event_exceptions::{EventException, NewEventException, UpdateEventException},
    event_favorites::{EventFavorite, NewEventFavorite},
}; // TODO: rm -f
