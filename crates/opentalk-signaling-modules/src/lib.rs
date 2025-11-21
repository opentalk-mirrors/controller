// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use async_trait::async_trait;
use opentalk_signaling_core::{ModulesRegistrar, RegisterModules};
use opentalk_signaling_module_asset_storage::AssetStorage;
use opentalk_signaling_module_automod::Automod;
use opentalk_signaling_module_breakout::Breakout;
use opentalk_signaling_module_chat::Chat;
use opentalk_signaling_module_core::Core;
use opentalk_signaling_module_echo::Echo;
use opentalk_signaling_module_legal_vote::LegalVote;
use opentalk_signaling_module_livekit::Livekit;
use opentalk_signaling_module_meeting_notes::MeetingNotes;
use opentalk_signaling_module_meeting_report::MeetingReport;
use opentalk_signaling_module_moderation::Moderation;
use opentalk_signaling_module_polls::Polls;
use opentalk_signaling_module_recording::{Recording, RecordingService};
use opentalk_signaling_module_shared_folder::SharedFolder;
use opentalk_signaling_module_subroom_audio::SubroomAudio;
use opentalk_signaling_module_timer::Timer;
use opentalk_signaling_module_training_participation_report::TrainingParticipationReport;
use opentalk_signaling_module_whiteboard::Whiteboard;

pub struct Modules;

#[async_trait(?Send)]
impl RegisterModules for Modules {
    fn register<E>(registrar: &mut impl ModulesRegistrar<Error = E>) -> Result<(), E> {
        registrar.register::<Core>()?;
        registrar.register::<Breakout>()?;
        registrar.register::<Moderation>()?;
        registrar.register::<Echo>()?;
        registrar.register::<Recording>()?;
        registrar.register::<RecordingService>()?;
        registrar.register::<Chat>()?;
        registrar.register::<LegalVote>()?;
        registrar.register::<Automod>()?;
        registrar.register::<Livekit>()?;
        registrar.register::<Polls>()?;
        registrar.register::<MeetingNotes>()?;
        registrar.register::<SharedFolder>()?;
        registrar.register::<Timer>()?;
        registrar.register::<Whiteboard>()?;
        registrar.register::<MeetingReport>()?;
        registrar.register::<SubroomAudio>()?;
        registrar.register::<TrainingParticipationReport>()?;
        registrar.register::<AssetStorage>()
    }
}
