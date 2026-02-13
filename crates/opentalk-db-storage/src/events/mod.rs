// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

pub use crate::tables::{
    event_email_invites as email_invites,
    event_exceptions::{EventException, NewEventException, UpdateEventException},
    event_favorites::{EventFavorite, NewEventFavorite},
    event_invites::{EventInvite, NewEventInvite, UpdateEventInvite},
    event_shared_folders as shared_folders,
    event_training_participation_report_parameter_sets::{
        EventTrainingParticipationReportParameterSet,
        UpdateEventTrainingParticipationReportParameterSet,
    },
    events::{Event, NewEvent, UpdateEvent},
}; // TODO: rm -f
