// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::collections::BTreeMap;

use chrono_tz::Tz;
use icu_locid::LanguageIdentifier;
use opentalk_report_generation::{ReportDateTime, ToReportDateTime};
use opentalk_types_common::{
    events::{EventDescription, EventTitle},
    time::Timestamp,
    users::DisplayName,
};
use opentalk_types_signaling::ParticipantId;

use super::Checkpoint;
use crate::{AVAILABLE_LANGUAGES, storage::RoomState};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct ReportTemplateParameter {
    pub available_languages: Vec<LanguageIdentifier>,
    pub title: EventTitle,
    pub description: EventDescription,
    pub start: ReportDateTime,
    pub end: ReportDateTime,
    pub report_timezone: Tz,
    pub report_language: LanguageIdentifier,
    pub participants: BTreeMap<ParticipantId, Option<DisplayName>>,
    pub checkpoints: Vec<Checkpoint>,
}

impl ReportTemplateParameter {
    pub(crate) fn build(
        room_state: &RoomState,
        report_tz: &Tz,
        report_language: LanguageIdentifier,
        participants: BTreeMap<ParticipantId, Option<DisplayName>>,
        title: EventTitle,
        description: EventDescription,
        end: Timestamp,
    ) -> Self {
        let checkpoints = room_state
            .history
            .iter()
            .map(|storage_checkpoint| {
                Checkpoint::from_storage_checkpoint(storage_checkpoint, report_tz)
            })
            .collect();
        Self {
            available_languages: Vec::from_iter(AVAILABLE_LANGUAGES.iter().cloned()),
            title,
            description,
            start: room_state.start.to_report_date_time(report_tz),
            end: end.to_report_date_time(report_tz),
            report_timezone: *report_tz,
            report_language,
            participants,
            checkpoints,
        }
    }
}
