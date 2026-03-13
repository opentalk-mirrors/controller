// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
// SPDX-License-Identifier: EUPL-1.2

use crate::{
    Etherpad, LiveKit, Reports, SettingsError, SettingsRaw, Spacedeck, SubroomAudio,
    settings_file::RoomServer,
};

/// Signaling configuration for either Controller or RoomServer signaling
#[derive(Debug, Clone, PartialEq)]
pub enum Signaling {
    /// RoomServer is configured as signaling backend
    RoomServer(RoomServer),
    /// Controller is configured as signaling backend
    Controller(ControllerSignaling),
}

/// Configuration for controller based signaling modules
#[derive(Debug, Clone, PartialEq)]
pub struct ControllerSignaling {
    /// The SubroomAudio settings.
    pub subroom_audio: SubroomAudio,

    /// The Etherpad settings.
    pub etherpad: Option<Etherpad>,

    /// The Spacedeck settings.
    pub spacedeck: Option<Spacedeck>,

    /// The Reports settings.
    pub reports: Reports,

    /// The LiveKit settings.
    pub livekit: LiveKit,
}

impl TryFrom<&SettingsRaw> for Signaling {
    type Error = SettingsError;

    fn try_from(raw: &SettingsRaw) -> Result<Self, Self::Error> {
        if let Some(roomserver) = &raw.roomserver {
            return Ok(Self::RoomServer(roomserver.clone()));
        }

        let livekit = raw
            .livekit
            .clone()
            .ok_or(SettingsError::LiveKitMissing)?
            .into();
        let subroom_audio = raw
            .subroom_audio
            .clone()
            .map(Into::into)
            .unwrap_or_default();
        let etherpad = raw.etherpad.clone().map(Into::into);
        let spacedeck = raw.spacedeck.clone().map(Into::into);
        let reports = raw.reports.clone().map(Into::into).unwrap_or_default();

        Ok(Self::Controller(ControllerSignaling {
            subroom_audio,
            etherpad,
            spacedeck,
            reports,
            livekit,
        }))
    }
}

impl Signaling {
    /// Returns `Some(&ControllerSignaling)` if the signaling is configured for the controller, [`None`] otherwise.
    pub fn controller(&self) -> Option<&ControllerSignaling> {
        match self {
            Signaling::Controller(settings) => Some(settings),
            _ => None,
        }
    }
}
