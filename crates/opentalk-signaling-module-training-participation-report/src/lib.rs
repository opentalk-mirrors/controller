// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Signaling module for tracking participant presence during a training session

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

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use bytes::Bytes;
use chrono::{Local, Utc};
use chrono_tz::Tz;
use either::Either;
use fluent_langneg::{NegotiationStrategy, negotiate_languages};
use futures::{FutureExt as _, stream::once};
use icu_locid::{LanguageIdentifier, langid};
use opentalk_inventory::InventoryProvider;
use opentalk_report_generation::GenerateOptions;
use opentalk_signaling_core::{
    ChunkFormat, CleanupScope, DestroyContext, Event, InitContext, ModuleContext, ObjectStorage,
    ObjectStorageError, SignalingModule, SignalingModuleDescription, SignalingModuleError,
    SignalingModuleFeatureDescription, SignalingModuleInitData, SignalingRoomId, VolatileStorage,
    assets::{AssetError, AssetSaved, NewAssetFileName, save_asset},
    control::{
        self, ControlStorageProvider,
        storage::{
            ControlStorage, ControlStorageParticipantAttributes as _, DISPLAY_NAME, IS_PRESENT,
        },
    },
};
use opentalk_types_common::{
    assets::{AssetFileKind, FileExtension, asset_file_kind},
    events::{EventDescription, EventTitle},
    modules::ModuleId,
    rooms::RoomId,
    time::{TimeZone, Timestamp},
    training_participation_report::{TimeRange, TrainingParticipationReportParameterSet},
    users::{DisplayName, UserId},
};
use opentalk_types_signaling::ParticipantId;
use opentalk_types_signaling_control::state::ControlState;
use opentalk_types_signaling_training_participation_report::{
    MODULE_ID,
    command::TrainingParticipationReportCommand,
    event::{
        Error, PdfAsset, PresenceLoggingEnded, PresenceLoggingEndedReason, PresenceLoggingStarted,
        PresenceLoggingStartedReason, TrainingParticipationReportEvent,
    },
    state::{ParticipationLoggingState, TrainingParticipationReportState},
};
use rand::RngExt as _;
use snafu::{Report, ResultExt};
use storage::{RoomState, TrainingParticipationReportStorage, TrainingReportState};
use template::ReportTemplateParameter;
use tokio::time::sleep;

pub mod exchange;
mod storage;
mod template;

const TEMPLATE: &str = include_str!("../templates/training_participation_report.typ");
const FTL_EN: &str = include_str!("../templates/l10n/en.ftl");
const FTL_DE: &str = include_str!("../templates/l10n/de.ftl");
const AVAILABLE_LANGUAGES: &[LanguageIdentifier] = &[langid!("en"), langid!("de")];

fn default_initial_checkpoint_delay() -> TimeRange {
    TimeRange::new_with_clamped_durations(Duration::from_mins(10), Duration::from_mins(20))
}

fn default_checkpoint_interval() -> TimeRange {
    TimeRange::new_with_clamped_durations(Duration::from_mins(60 + 45), Duration::from_mins(30))
}

/// An event queued by the runner for itself to handle a timeout
#[derive(Debug, PartialEq, Eq)]
pub struct TimeoutEvent(u32);

/// Signaling module for tracking participant presence during a training session
#[derive(Debug)]
pub struct TrainingParticipationReport {
    system_default_language: LanguageIdentifier,
    typst_packages_path: PathBuf,
    room: RoomId,
    owner: UserId,
    participant: ParticipantId,
    inventory_provider: Arc<dyn InventoryProvider>,
    storage: Arc<ObjectStorage>,
    timeout_id: Option<u32>,
    is_room_owner: bool,
}

trait TrainingParticipationReportStorageProvider {
    fn storage(&mut self) -> &mut dyn TrainingParticipationReportStorage;
}

impl TrainingParticipationReportStorageProvider for VolatileStorage {
    fn storage(&mut self) -> &mut dyn TrainingParticipationReportStorage {
        match self.as_mut() {
            Either::Left(v) => v,
            Either::Right(v) => v,
        }
    }
}

impl SignalingModuleDescription for TrainingParticipationReport {
    const MODULE_ID: ModuleId = MODULE_ID;
    const DESCRIPTION: &'static str = "Handles training participation report functionality. Participants are asked to confirm their presence repeatedly at pre-configured time intervals. These confirmations are documented in the training participation report which is created automatically at the end of the meeting.";
    const FEATURES: &[SignalingModuleFeatureDescription] = &[];
}

/// The initialization parameters for [`TrainingParticipationReport`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainingParticipationReportParams {
    system_default_language: LanguageIdentifier,
    typst_packages_path: PathBuf,
}

#[async_trait::async_trait(?Send)]
impl SignalingModule for TrainingParticipationReport {
    const NAMESPACE: ModuleId = MODULE_ID;

    type Params = TrainingParticipationReportParams;

    type Incoming = TrainingParticipationReportCommand;

    type Outgoing = TrainingParticipationReportEvent;

    type ExchangeMessage = exchange::Event;

    type ExtEvent = TimeoutEvent;

    type FrontendData = TrainingParticipationReportState;

    type PeerFrontendData = ();

    async fn init(
        ctx: InitContext<'_, Self>,
        Self::Params {
            system_default_language,
            typst_packages_path,
        }: &Self::Params,
        _protocol: &'static str,
    ) -> Result<Option<Self>, SignalingModuleError> {
        Ok(Some(Self {
            system_default_language: system_default_language.clone(),
            typst_packages_path: typst_packages_path.clone(),
            room: ctx.room_id().room_id(),
            owner: ctx.room().created_by,
            participant: ctx.participant_id(),
            inventory_provider: ctx.inventory_provider().clone(),
            storage: ctx.storage().clone(),
            // The data required for the checkpoint runner  is only available on join,
            // so we will store it when the join is handled.
            timeout_id: None,
            is_room_owner: false,
        }))
    }

    async fn on_event(
        &mut self,
        mut ctx: ModuleContext<'_, Self>,
        event: Event<'_, Self>,
    ) -> Result<(), SignalingModuleError> {
        match event {
            Event::Joined {
                control_data,
                participants,
                frontend_data,
            } => {
                self.handle_joined(&mut ctx, control_data, participants, frontend_data)
                    .await?;
            }
            Event::WsMessage(msg) => {
                self.handle_ws_message(&mut ctx, msg).await?;
            }
            Event::Ext(TimeoutEvent(timeout_id)) => {
                self.handle_timeout(&mut ctx, timeout_id).await?
            }
            Event::Exchange(event) => self.handle_exchange_event(&mut ctx, event).await?,
            Event::Leaving => self.handle_leaving(&mut ctx).await?,
            Event::RaiseHand
            | Event::LowerHand
            | Event::ParticipantJoined(_, _)
            | Event::ParticipantLeft(_)
            | Event::ParticipantUpdated(_, _)
            | Event::RoleUpdated(_) => {}
        }

        Ok(())
    }

    async fn on_destroy(self, ctx: DestroyContext<'_>) {
        if matches!(ctx.cleanup_scope, CleanupScope::Global) {
            self.clean_up_parameter_set_storage(ctx.volatile.storage())
                .await;
        }
    }

    fn build_params(
        init: SignalingModuleInitData,
    ) -> Result<Option<Self::Params>, SignalingModuleError> {
        let Some(signaling_settings) = init.startup_settings.signaling.controller() else {
            return Ok(None);
        };

        let typst_packages_path = signaling_settings.reports.typst.packages_path.clone();
        let system_default_language = init.startup_settings.defaults.user_language.clone();
        Ok(Some(TrainingParticipationReportParams {
            system_default_language,
            typst_packages_path,
        }))
    }
}

impl TrainingParticipationReport {
    async fn handle_joined(
        &mut self,
        ctx: &mut ModuleContext<'_, Self>,
        control_data: &ControlState,
        participants: &HashMap<ParticipantId, Option<()>>,
        frontend_data: &mut Option<TrainingParticipationReportState>,
    ) -> Result<(), SignalingModuleError> {
        let parameter_set = if ctx
            .volatile
            .storage()
            .is_parameter_set_initialized(self.room)
            .await?
        {
            ctx.volatile.storage().get_parameter_set(self.room).await?
        } else {
            self.initialize_parameter_set(ctx.volatile.storage())
                .await?
        };

        let mut participation_logging_state = ctx
            .volatile
            .storage()
            .get_participation_logging_state(self.room, self.participant)
            .await?;

        let participants = participants.keys().cloned().collect();
        let other_present_participants = self
            .filter_other_present_participants(participants, ctx.volatile.control_storage())
            .await?;

        self.is_room_owner = control_data.is_room_owner;

        if other_present_participants.is_empty() {
            // This is the first checkpoint runner in the room, use it for generating checkpoints

            if let Some(TrainingParticipationReportParameterSet {
                initial_checkpoint_delay,
                checkpoint_interval,
            }) = parameter_set.clone()
            {
                // Initialize the room with all other present participants
                ctx.volatile
                    .storage()
                    .initialize_room(
                        self.room,
                        ctx.timestamp,
                        TrainingReportState::WaitingForInitialTimeout,
                        initial_checkpoint_delay.clone(),
                        checkpoint_interval.clone(),
                        other_present_participants.clone(),
                    )
                    .await?;

                self.start_presence_logging(
                    ctx,
                    initial_checkpoint_delay.clone(),
                    PresenceLoggingStartedReason::Autostart,
                )
                .await?;

                participation_logging_state = ParticipationLoggingState::Enabled;
            }
        }

        // If participation reporting is active, add myself to the known participants
        let training_report_state = ctx
            .volatile
            .storage()
            .get_training_report_state(self.room)
            .await?;
        if training_report_state.is_some() {
            ctx.volatile
                .storage()
                .add_known_participant(self.room, self.participant)
                .await?;
        }

        *frontend_data = Some(TrainingParticipationReportState {
            state: participation_logging_state,
            parameter_set: if control_data.is_room_owner {
                parameter_set
            } else {
                None
            },
        });

        Ok(())
    }

    async fn initialize_parameter_set(
        &mut self,
        storage: &mut dyn TrainingParticipationReportStorage,
    ) -> Result<Option<TrainingParticipationReportParameterSet>, SignalingModuleError> {
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let Some(event) = storage.get_event(self.room).await? else {
            return Ok(None);
        };
        let parameter_set = inventory
            .get_event_training_participation_report_parameter_set(event.id)
            .await?
            .map(TrainingParticipationReportParameterSet::from);

        if let Some(parameter_set) = &parameter_set {
            storage
                .set_parameter_set(self.room, parameter_set.clone())
                .await?;
        }

        storage.set_parameter_set_initialized(self.room).await?;

        Ok(parameter_set)
    }

    async fn get_other_present_participants(
        &self,
        control_storage: &mut dyn ControlStorage,
    ) -> Result<BTreeSet<ParticipantId>, SignalingModuleError> {
        let participants = control_storage
            .get_all_participants(SignalingRoomId::new_for_room(self.room))
            .await?
            .iter()
            .cloned()
            .collect::<Vec<_>>();

        self.filter_other_present_participants(participants, control_storage)
            .await
    }

    async fn filter_other_present_participants(
        &self,
        participants: Vec<ParticipantId>,
        control_storage: &mut dyn ControlStorage,
    ) -> Result<BTreeSet<ParticipantId>, SignalingModuleError> {
        let is_present: Vec<Option<bool>> = control_storage
            .get_global_attribute_for_participants(&participants, self.room, IS_PRESENT)
            .await?;

        let is_present = is_present.into_iter().map(|p| p.unwrap_or_default());
        let mut other_present_participants = BTreeSet::new();
        for participant in participants
            .into_iter()
            .zip(is_present)
            .filter_map(|(participant, is_present)| is_present.then_some(participant))
            .filter(|participant| *participant != self.participant)
        {
            _ = other_present_participants.insert(participant);
        }

        Ok(other_present_participants)
    }

    async fn handle_ws_message(
        &mut self,
        ctx: &mut ModuleContext<'_, Self>,
        msg: TrainingParticipationReportCommand,
    ) -> Result<(), SignalingModuleError> {
        match msg {
            TrainingParticipationReportCommand::EnablePresenceLogging {
                initial_checkpoint_delay,
                checkpoint_interval,
            } => {
                self.handle_command_enable_presence_logging(
                    ctx,
                    initial_checkpoint_delay,
                    checkpoint_interval,
                )
                .await
            }
            TrainingParticipationReportCommand::DisablePresenceLogging => {
                self.handle_command_disable_presence_logging(ctx).await
            }
            TrainingParticipationReportCommand::ConfirmPresence => {
                self.handle_command_confirm_presence(ctx).await
            }
        }
    }

    async fn handle_command_enable_presence_logging(
        &mut self,
        ctx: &mut ModuleContext<'_, Self>,
        initial_checkpoint_delay: Option<TimeRange>,
        checkpoint_interval: Option<TimeRange>,
    ) -> Result<(), SignalingModuleError> {
        if !self.is_room_owner {
            ctx.ws_send(TrainingParticipationReportEvent::Error(
                Error::InsufficientPermissions,
            ));
            return Ok(());
        }

        let other_present_participants = self
            .get_other_present_participants(ctx.volatile.control_storage())
            .await?;

        let storage = ctx.volatile.storage();

        if storage
            .get_training_report_state(self.room)
            .await?
            .is_some()
        {
            ctx.ws_send(TrainingParticipationReportEvent::Error(
                Error::PresenceLoggingAlreadyEnabled,
            ));
            return Ok(());
        }
        let (parameter_set_initial_checkpoint_delay, parameter_set_checkpoint_interval) = storage
            .get_parameter_set(self.room)
            .await?
            .map(
                |TrainingParticipationReportParameterSet {
                     initial_checkpoint_delay,
                     checkpoint_interval,
                 }| { (Some(initial_checkpoint_delay), Some(checkpoint_interval)) },
            )
            .unwrap_or_default();
        let initial_checkpoint_delay = initial_checkpoint_delay
            .or(parameter_set_initial_checkpoint_delay)
            .unwrap_or_else(default_initial_checkpoint_delay);
        let checkpoint_interval = checkpoint_interval
            .or(parameter_set_checkpoint_interval)
            .unwrap_or_else(default_checkpoint_interval);

        // Initialize the room with all other present participants
        storage
            .initialize_room(
                self.room,
                ctx.timestamp,
                TrainingReportState::WaitingForInitialTimeout,
                initial_checkpoint_delay.clone(),
                checkpoint_interval,
                other_present_participants,
            )
            .await?;

        // Add myself to the known participants
        storage
            .add_known_participant(self.room, self.participant)
            .await?;

        ctx.exchange_publish(
            control::exchange::global_room_by_user_id(self.room, self.owner),
            exchange::Event::PresenceLoggingEnabled,
        );

        self.start_presence_logging(
            ctx,
            initial_checkpoint_delay,
            PresenceLoggingStartedReason::StartedManually,
        )
        .await
    }

    async fn start_presence_logging(
        &mut self,
        ctx: &mut ModuleContext<'_, Self>,
        initial_checkpoint_delay: TimeRange,
        reason: PresenceLoggingStartedReason,
    ) -> Result<(), SignalingModuleError> {
        let first_checkpoint = self
            .switch_to_next_checkpoint(ctx, &initial_checkpoint_delay)
            .await?;

        ctx.exchange_publish(
            control::exchange::global_room_all_participants(self.room),
            exchange::Event::PresenceLoggingStarted {
                first_checkpoint,
                reason,
            },
        );

        Ok(())
    }

    async fn switch_to_next_checkpoint(
        &mut self,
        ctx: &mut ModuleContext<'_, Self>,
        time_range: &TimeRange,
    ) -> Result<Timestamp, SignalingModuleError> {
        let wait_duration = Self::random_waiting_duration(time_range);
        let checkpoint = ctx.timestamp
            + chrono::Duration::from_std(wait_duration)
                .with_whatever_context::<_, _, SignalingModuleError>(|e| {
                    format!("Duration out of range: {e}")
                })?;
        self.start_checkpoint_timer(ctx, checkpoint);

        ctx.volatile
            .storage()
            .switch_to_next_checkpoint(self.room, checkpoint)
            .await?;

        Ok(checkpoint)
    }

    fn start_checkpoint_timer(&mut self, ctx: &mut ModuleContext<'_, Self>, checkpoint: Timestamp) {
        let timeout_id = rand::rng().random();
        self.timeout_id = Some(timeout_id);

        let duration = checkpoint
            .signed_duration_since(Utc::now())
            .to_std()
            .unwrap_or_default();
        let event = TimeoutEvent(timeout_id);

        ctx.add_event_stream(once(sleep(duration).map(move |_| event)));
    }

    async fn handle_command_disable_presence_logging(
        &mut self,
        ctx: &mut ModuleContext<'_, Self>,
    ) -> Result<(), SignalingModuleError> {
        if !self.is_room_owner {
            ctx.ws_send(TrainingParticipationReportEvent::Error(
                Error::InsufficientPermissions,
            ));
            return Ok(());
        }

        let Some(room_state) = ctx.volatile.storage().cleanup_room(self.room).await? else {
            ctx.ws_send(TrainingParticipationReportEvent::Error(
                Error::PresenceLoggingNotEnabled,
            ));
            return Ok(());
        };

        let report_state = room_state.report_state;

        if matches!(report_state, TrainingReportState::TrackingPresence) {
            self.create_training_participation_report(ctx, room_state)
                .await?;
        }

        if matches!(
            report_state,
            TrainingReportState::TrackingPresence | TrainingReportState::WaitingForInitialTimeout
        ) {
            let reason = PresenceLoggingEndedReason::StoppedManually;
            ctx.exchange_publish(
                control::exchange::global_room_all_participants(self.room),
                exchange::Event::PresenceLoggingEnded { reason },
            );
        }

        ctx.exchange_publish(
            control::exchange::global_room_by_user_id(self.room, self.owner),
            exchange::Event::PresenceLoggingDisabled,
        );

        Ok(())
    }

    async fn handle_command_confirm_presence(
        &mut self,
        ctx: &mut ModuleContext<'_, Self>,
    ) -> Result<(), SignalingModuleError> {
        let storage = ctx.volatile.storage();
        if storage.get_training_report_state(self.room).await?
            != Some(TrainingReportState::TrackingPresence)
        {
            ctx.ws_send(TrainingParticipationReportEvent::Error(
                Error::PresenceLoggingNotEnabled,
            ));
            return Ok(());
        }

        storage
            .record_presence_confirmation(self.room, self.participant, ctx.timestamp)
            .await?;
        ctx.ws_send(TrainingParticipationReportEvent::PresenceConfirmationLogged);

        Ok(())
    }

    fn random_waiting_duration(range: &TimeRange) -> Duration {
        let within = range.within();
        let timeframe = if within.is_zero() {
            within
        } else {
            let mut rng = rand::rng();
            rng.random_range(Duration::ZERO..within)
        };
        range.after().saturating_add(timeframe)
    }

    async fn handle_timeout(
        &mut self,
        ctx: &mut ModuleContext<'_, Self>,
        timeout_id: u32,
    ) -> Result<(), SignalingModuleError> {
        if self.timeout_id != Some(timeout_id) {
            // Timeout has been canceled or another timeout has been started
            // after the one we're currently handling, so this one is obsolete
            // and we just ignore it.
            return Ok(());
        }

        if ctx
            .volatile
            .storage()
            .get_training_report_state(self.room)
            .await?
            .is_none()
        {
            // Presence logging has stopped, no need to handle the timer.
            return Ok(());
        }

        let time_range = ctx
            .volatile
            .storage()
            .get_checkpoint_interval(self.room)
            .await?;

        ctx.volatile
            .storage()
            .set_training_report_state(self.room, TrainingReportState::TrackingPresence)
            .await?;
        let _checkpoint = self.switch_to_next_checkpoint(ctx, &time_range).await?;

        ctx.exchange_publish(
            control::exchange::global_room_all_participants(self.room),
            exchange::Event::PresenceConfirmationRequested,
        );
        Ok(())
    }

    async fn handle_exchange_event(
        &mut self,
        ctx: &mut ModuleContext<'_, Self>,
        event: exchange::Event,
    ) -> Result<(), SignalingModuleError> {
        match event {
            exchange::Event::PresenceLoggingStarted {
                first_checkpoint,
                reason,
            } => {
                let message = if self.is_room_owner {
                    PresenceLoggingStarted {
                        first_checkpoint: Some(first_checkpoint),
                        reason: Some(reason),
                    }
                } else {
                    PresenceLoggingStarted {
                        first_checkpoint: None,
                        reason: None,
                    }
                };
                ctx.ws_send(message);
                Ok(())
            }
            exchange::Event::PresenceLoggingEnded { reason } => {
                ctx.ws_send(PresenceLoggingEnded { reason });
                Ok(())
            }
            exchange::Event::PresenceLoggingEnabled => {
                ctx.ws_send(TrainingParticipationReportEvent::PresenceLoggingEnabled);
                Ok(())
            }
            exchange::Event::PresenceLoggingDisabled => {
                ctx.ws_send(TrainingParticipationReportEvent::PresenceLoggingDisabled);
                Ok(())
            }
            exchange::Event::PresenceConfirmationRequested => {
                ctx.ws_send(TrainingParticipationReportEvent::PresenceConfirmationRequested);
                Ok(())
            }
            exchange::Event::RoomOwnerHandOver { next_checkpoint } => {
                self.start_checkpoint_timer(ctx, next_checkpoint);
                Ok(())
            }
            exchange::Event::PdfAsset(pdf_asset) => {
                ctx.ws_send(pdf_asset);
                Ok(())
            }
        }
    }

    async fn handle_leaving(
        &mut self,
        ctx: &mut ModuleContext<'_, Self>,
    ) -> Result<(), SignalingModuleError> {
        if ctx
            .volatile
            .storage()
            .get_training_report_state(self.room)
            .await?
            .is_none()
        {
            return Ok(());
        }

        let other_present_participants = self
            .get_other_present_participants(ctx.volatile.control_storage())
            .await?;

        match other_present_participants.iter().next() {
            None => {
                let reason = PresenceLoggingEndedReason::LastParticipantLeft;
                ctx.exchange_publish(
                    control::exchange::global_room_all_participants(self.room),
                    exchange::Event::PresenceLoggingEnded { reason },
                );

                // Abort waiting for the initial timeout.
                let Some(room_state) = ctx.volatile.storage().cleanup_room(self.room).await? else {
                    return Ok(());
                };

                if matches!(
                    room_state.report_state,
                    TrainingReportState::TrackingPresence
                ) {
                    self.create_training_participation_report(ctx, room_state)
                        .await?;
                }
                return Ok(());
            }
            Some(other_present_participant) => {
                // At least one other potential checkpoint runner is present.

                if self.timeout_id.is_none() {
                    // This potential checkpoint runner was not responsible for organizing the checkpoints.
                    return Ok(());
                };

                // Let's hand over the responsibility for checkpoint handling to the next runner.

                let Some(next_checkpoint) = ctx
                    .volatile
                    .storage()
                    .get_next_checkpoint(self.room)
                    .await?
                else {
                    return Ok(());
                };

                ctx.exchange_publish(
                    control::exchange::global_room_by_participant_id(
                        self.room,
                        *other_present_participant,
                    ),
                    exchange::Event::RoomOwnerHandOver { next_checkpoint },
                );
            }
        }

        Ok(())
    }

    async fn create_training_participation_report(
        &mut self,
        ctx: &mut ModuleContext<'_, Self>,
        room_state: RoomState,
    ) -> Result<(), SignalingModuleError> {
        let (event, room_owner) = {
            let mut inventory = self.inventory_provider.get_inventory().await?;
            let event = inventory.get_event_for_room(self.room).await?.ok_or(
                SignalingModuleError::NotFoundError {
                    message: "Event for room not found".to_string(),
                },
            )?;
            let (_room, room_owner) = inventory.get_room_with_creator(self.room).await?;
            (event, room_owner)
        };

        let required_participants = Vec::from_iter(room_state.known_participants.clone());

        let display_names: Vec<Option<DisplayName>> = ctx
            .volatile
            .control_storage()
            .get_global_attribute_for_participants(&required_participants, self.room, DISPLAY_NAME)
            .await?;
        let participants = required_participants
            .into_iter()
            .zip(display_names)
            .collect();

        let language = {
            let fallback = AVAILABLE_LANGUAGES
                .iter()
                .next()
                .expect("AVAILABLE_LANGUAGES is not empty");
            let system_default = &self.system_default_language;

            let requested_languages: Vec<_> = room_owner
                .language
                .iter()
                .map(|v| v.as_ref())
                .chain([system_default])
                .collect();

            negotiate_languages(
                &requested_languages,
                AVAILABLE_LANGUAGES,
                None,
                NegotiationStrategy::Lookup,
            )
            .into_iter()
            .next()
            .unwrap_or_else(|| {
                log::warn!("Could not find a valid report language. System default: {system_default}, available: {AVAILABLE_LANGUAGES:?}.");
                fallback
            }).clone()
        };

        let report = Self::generate_pdf_report(
            TEMPLATE.to_string(),
            room_state,
            ctx.timezone,
            participants,
            event.title,
            event.description,
            ctx.timestamp,
            language,
            &self.typst_packages_path,
        )
        .await
        .with_whatever_context::<_, _, SignalingModuleError>(|_| {
            ctx.ws_send(Error::Generate);
            "Failed to create pdf"
        })?;
        self.upload_pdf(report, ctx).await;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn generate_pdf_report(
        template: String,
        room_state: RoomState,
        report_timezone: TimeZone,
        participants: BTreeMap<ParticipantId, Option<DisplayName>>,
        title: EventTitle,
        description: EventDescription,
        end: Timestamp,
        report_language: LanguageIdentifier,
        typst_package_path: &Path,
    ) -> Result<Vec<u8>, SignalingModuleError> {
        let timestamp = Local::now().naive_local().format("%Y-%m-%dT%H:%M:%S.%f");
        let report_tz = Tz::from(report_timezone);

        Self::generate_pdf_report_from_template(
            template,
            &ReportTemplateParameter::build(
                &room_state,
                &report_tz,
                report_language,
                participants,
                title,
                description,
                end,
            ),
            Path::new(&format!("{MODULE_ID}/{timestamp}")),
            typst_package_path,
        )
    }

    fn generate_pdf_report_from_template(
        template: String,
        parameter: &ReportTemplateParameter,
        dump_to_relative_path: &Path,
        typst_package_path: &Path,
    ) -> Result<Vec<u8>, SignalingModuleError> {
        let dump_to_path = std::env::var("OPENTALK_REPORT_DUMP_PATH")
            .map(|p| Path::new(&p).join(dump_to_relative_path))
            .ok();

        let mut generate_options = GenerateOptions::default();
        generate_options.dump_to_path = dump_to_path.as_deref();
        generate_options.packages_path = Some(typst_package_path);

        let pdf = opentalk_report_generation::generate_pdf_report(
            template,
            BTreeMap::from_iter([
                (
                    Path::new("data.json"),
                    (
                        None,
                        serde_json::to_string_pretty(parameter)
                            .unwrap()
                            .into_bytes()
                            .into(),
                    ),
                ),
                (Path::new("l10n/de.ftl"), (None, FTL_DE.as_bytes().into())),
                (Path::new("l10n/en.ftl"), (None, FTL_EN.as_bytes().into())),
            ]),
            &generate_options,
        )
        .whatever_context::<_, SignalingModuleError>("unable to build pdf")?;
        Ok(pdf)
    }

    async fn upload_pdf(&mut self, report: Vec<u8>, ctx: &mut ModuleContext<'_, Self>) {
        const ASSET_FILE_KIND: AssetFileKind = asset_file_kind!("training_participation_report");
        let file_name =
            NewAssetFileName::new(ASSET_FILE_KIND, Timestamp::now(), FileExtension::pdf());
        let report =
            async_stream::stream!(yield Result::<_, ObjectStorageError>::Ok(Bytes::from(report)));
        let report = Box::pin(report);
        let result = save_asset(
            &self.storage,
            self.inventory_provider.as_ref(),
            self.room,
            Some(Self::NAMESPACE),
            file_name,
            report,
            ChunkFormat::Data,
        )
        .await;

        // If storing the asset failed, we report the error and silently return.
        let AssetSaved {
            asset_id, filename, ..
        } = match result {
            Ok(inner) => inner,
            Err(AssetError::AssetStorageExceeded) => {
                log::debug!("Storage exceeded while storing training participation report");
                ctx.ws_send(Error::StorageExceeded);
                return;
            }
            Err(e) => {
                log::error!(
                    "Error while storing attendance report: {}",
                    Report::from_error(e)
                );
                ctx.ws_send(Error::Storage);
                return;
            }
        };

        let pdf_asset = PdfAsset { filename, asset_id };
        log::debug!("Generated meeting attendance report: {pdf_asset:?}");
        ctx.exchange_publish(
            control::exchange::global_room_by_user_id(self.room, self.owner),
            exchange::Event::PdfAsset(pdf_asset.clone()),
        );
    }

    async fn clean_up_parameter_set_storage(
        self,
        storage: &mut dyn TrainingParticipationReportStorage,
    ) {
        match storage.is_parameter_set_initialized(self.room).await {
            Ok(is_initialized) => {
                if is_initialized {
                    Self::clean_up_parameter_set(self.room, storage).await;
                }
                Self::clean_up_parameter_set_initialized(self.room, storage).await;
            }
            Err(e) => {
                log::error!(
                    "Failed to read training participation report parameter set initialized flag for room {} cleanup: {}",
                    self.room,
                    e
                );
            }
        }
    }

    async fn clean_up_parameter_set_initialized(
        room: RoomId,
        storage: &mut dyn TrainingParticipationReportStorage,
    ) {
        if let Err(e) = storage.delete_parameter_set_initialized(room).await {
            log::error!(
                "Failed to clean up training participation report parameter set initialized flag for room {room}: {e}"
            );
        }
    }

    async fn clean_up_parameter_set(
        room: RoomId,
        storage: &mut dyn TrainingParticipationReportStorage,
    ) {
        if let Err(e) = storage.delete_parameter_set(room).await {
            log::error!(
                "Failed to clean up training participation report parameter set {room}: {e}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use icu_locid::langid;
    use insta::assert_snapshot;
    use opentalk_controller_settings::reports_typst_default_packages_path;

    use crate::{
        MODULE_ID, TEMPLATE, TrainingParticipationReport, template::ReportTemplateParameter,
    };

    fn generate(sample_name: &str, parameter: &ReportTemplateParameter) -> String {
        const TYPST_PACKAGE_CACHE_PATH_ENV_VARIABLE: &str = "TYPST_PACKAGE_CACHE_PATH";

        let typst_packages_path =
            if let Some(env_variable) = std::env::var_os(TYPST_PACKAGE_CACHE_PATH_ENV_VARIABLE) {
                Path::new(&env_variable).to_path_buf()
            } else {
                dirs::cache_dir()
                    .map(|d| d.join("typst/packages"))
                    .unwrap_or_else(|| reports_typst_default_packages_path().to_path_buf())
            };

        assert!(
            typst_packages_path.exists(),
            "Please make sure that the typst packages path {typst_packages_path:?} exists and contains the required typst packages, or point the {TYPST_PACKAGE_CACHE_PATH_ENV_VARIABLE:?} environment variable to the path with the typst packages"
        );

        let pdf = TrainingParticipationReport::generate_pdf_report_from_template(
            TEMPLATE.to_string(),
            parameter,
            Path::new(&format!("{MODULE_ID}/{sample_name}")),
            &typst_packages_path,
        )
        .expect("generation should work");
        pdf_extract::extract_text_from_mem(&pdf)
            .expect("text should be extractable from generated pdf")
    }

    #[test]
    fn generate_report_small() {
        assert_snapshot!(
            generate(
                "small",
                &crate::template::tests::example_small()
            ),
            @r"
        Training participation report
         Meeting: OpenTalk introduction training

        Description: —

        Report timezone: Europe/Berlin

        Training start: 2025-02-18 09:01

        Training end: 2025-02-18 13:32

        Participation checkpoints
         № Person 09:22 11:22 13:19

        1 Bob Burton 09:22 11:25 —

        2 Charlie Cooper 09:22 11:25 13:19
        "
        );
    }

    #[test]
    fn generate_report_small_de() {
        let mut data = crate::template::tests::example_small();
        data.report_language = langid!("de");

        assert_snapshot!(
            generate(
                "small_de",
                &data
            ),
            @r"
        Schulungs-Teilnahmebericht
         Meeting: OpenTalk introduction training

        Beschreibung: —

        Zeitzone des Berichts: Europe/Berlin

        Beginn der Schulung: 2025-02-18 09:01

        Ende der Schulung: 2025-02-18 13:32

        Teilnahme-Kontrollpunkte
         № Person 09:22 11:22 13:19

        1 Bob Burton 09:22 11:25 —

        2 Charlie Cooper 09:22 11:25 13:19
        "
        );
    }

    #[test]
    fn generate_report_medium() {
        assert_snapshot!(
            generate(
                "medium",
                &crate::template::tests::example_medium()
            ),
            @r"
        Training participation report
         Meeting: OpenTalk introduction training

        Description: —

        Report timezone: Europe/Berlin

        Training start: 2025-02-18 09:01

        Training end: 2025-02-19 03:32

        Participation checkpoints
         № Person 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        1 Bob Burton 09:22 11:25 — — 17:21 19:31 21:31 —

        2 Charlie Cooper 09:22 — 13:19 — — — — 23:36

        № Person 01:37 03:27

        1 Bob Burton 01:37 03:27

        2 Charlie Cooper 01:55 —
        "
        );
    }

    #[test]
    fn generate_report_medium_de() {
        let mut data = crate::template::tests::example_medium();
        data.report_language = langid!("de");

        assert_snapshot!(
            generate(
                "medium_de",
                &data
            ),
            @r"
        Schulungs-Teilnahmebericht
         Meeting: OpenTalk introduction training

        Beschreibung: —

        Zeitzone des Berichts: Europe/Berlin

        Beginn der Schulung: 2025-02-18 09:01

        Ende der Schulung: 2025-02-19 03:32

        Teilnahme-Kontrollpunkte
         № Person 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        1 Bob Burton 09:22 11:25 — — 17:21 19:31 21:31 —

        2 Charlie Cooper 09:22 — 13:19 — — — — 23:36

        № Person 01:37 03:27

        1 Bob Burton 01:37 03:27

        2 Charlie Cooper 01:55 —
        "
        );
    }

    #[test]
    fn generate_report_large() {
        assert_snapshot!(
            generate(
                "large",
                &crate::template::tests::example_large()
            ),
            @r"
        Training participation report
         Meeting: OpenTalk introduction training

        Description: —

        Report timezone: Europe/Berlin

        Training start: 2025-02-18 09:01

        Training end: 2025-02-19 03:32

        Participation checkpoints
         № Person 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        1 Bob Burton 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        2 Charlie Cooper 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        3 Dave Dunn 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        4 Erin Eaton 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        5 Frank Floyd 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        6 George Garvis 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        7 Hannah Händl 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        8 Isaac Ivens (Northwind Ltd.) 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        9 Jack Jilbert 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        10 Karl Keating 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        11 Leann Larn 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        12 Marlene M. Maine 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        13 Neil Neugraten 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        14 Ofelia Ollivander 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        15 Patrick Peterson 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        16 Quinton Quintana 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        17 Roger Richard 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        18 Sophie Stanton 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        19 Thalia Tyler 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        20 Ulises Underwood 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        21 Valentina Villalobos 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        22 Wallace Winters 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        23 Xiomara Xiong 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        24 Yousef Yu 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        25 Zainab Zavala 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        26    09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        № Person 01:37 03:27

        1 Bob Burton 01:37 03:27

        2 Charlie Cooper 01:37 03:27

        3 Dave Dunn 01:37 03:27

        4 Erin Eaton 01:37 03:27

        5 Frank Floyd 01:37 03:27

        № Person 01:37 03:27

        6 George Garvis 01:37 03:27

        7 Hannah Händl 01:37 03:27

        8 Isaac Ivens (Northwind Ltd.) 01:37 03:27

        9 Jack Jilbert 01:37 03:27

        10 Karl Keating 01:37 03:27

        11 Leann Larn 01:37 03:27

        12 Marlene M. Maine 01:37 03:27

        13 Neil Neugraten 01:37 03:27

        14 Ofelia Ollivander 01:37 03:27

        15 Patrick Peterson 01:37 03:27

        16 Quinton Quintana 01:37 03:27

        17 Roger Richard 01:37 03:27

        18 Sophie Stanton 01:37 03:27

        19 Thalia Tyler 01:37 03:27

        20 Ulises Underwood 01:37 03:27

        21 Valentina Villalobos 01:37 03:27

        22 Wallace Winters 01:37 03:27

        23 Xiomara Xiong 01:37 03:27

        24 Yousef Yu 01:37 03:27

        25 Zainab Zavala 01:37 03:27

        26    01:37 03:27
        "
        );
    }

    #[test]
    fn generate_report_large_de() {
        let mut data = crate::template::tests::example_large();
        data.report_language = langid!("de");

        assert_snapshot!(
            generate(
                "large_de",
                &data
            ),
            @r"
        Schulungs-Teilnahmebericht
         Meeting: OpenTalk introduction training

        Beschreibung: —

        Zeitzone des Berichts: Europe/Berlin

        Beginn der Schulung: 2025-02-18 09:01

        Ende der Schulung: 2025-02-19 03:32

        Teilnahme-Kontrollpunkte
         № Person 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        1 Bob Burton 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        2 Charlie Cooper 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        3 Dave Dunn 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        4 Erin Eaton 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        5 Frank Floyd 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        6 George Garvis 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        7 Hannah Händl 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        8 Isaac Ivens (Northwind Ltd.) 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        9 Jack Jilbert 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        10 Karl Keating 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        11 Leann Larn 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        12 Marlene M. Maine 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        13 Neil Neugraten 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        14 Ofelia Ollivander 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        15 Patrick Peterson 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        16 Quinton Quintana 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        17 Roger Richard 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        18 Sophie Stanton 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        19 Thalia Tyler 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        20 Ulises Underwood 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        21 Valentina Villalobos 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        22 Wallace Winters 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        23 Xiomara Xiong 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        24 Yousef Yu 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        25 Zainab Zavala 09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        26    09:22 11:22 13:19 15:08 17:21 19:31 21:31 23:36

        № Person 01:37 03:27

        1 Bob Burton 01:37 03:27

        2 Charlie Cooper 01:37 03:27

        3 Dave Dunn 01:37 03:27

        4 Erin Eaton 01:37 03:27

        5 Frank Floyd 01:37 03:27

        № Person 01:37 03:27

        6 George Garvis 01:37 03:27

        7 Hannah Händl 01:37 03:27

        8 Isaac Ivens (Northwind Ltd.) 01:37 03:27

        9 Jack Jilbert 01:37 03:27

        10 Karl Keating 01:37 03:27

        11 Leann Larn 01:37 03:27

        12 Marlene M. Maine 01:37 03:27

        13 Neil Neugraten 01:37 03:27

        14 Ofelia Ollivander 01:37 03:27

        15 Patrick Peterson 01:37 03:27

        16 Quinton Quintana 01:37 03:27

        17 Roger Richard 01:37 03:27

        18 Sophie Stanton 01:37 03:27

        19 Thalia Tyler 01:37 03:27

        20 Ulises Underwood 01:37 03:27

        21 Valentina Villalobos 01:37 03:27

        22 Wallace Winters 01:37 03:27

        23 Xiomara Xiong 01:37 03:27

        24 Yousef Yu 01:37 03:27

        25 Zainab Zavala 01:37 03:27

        26    01:37 03:27
        "
        );
    }
}
