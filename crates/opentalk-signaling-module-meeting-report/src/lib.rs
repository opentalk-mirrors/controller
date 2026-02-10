// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use bytes::Bytes;
use chrono::{DateTime, Datelike, Local, Utc};
use chrono_tz::Tz;
use either::Either;
use fluent_langneg::{NegotiationStrategy, negotiate_languages};
use futures::{StreamExt as _, TryStreamExt, stream};
use icu_locid::{LanguageIdentifier, langid};
use opentalk_inventory::{Event as InventoryEvent, InventoryProvider};
use opentalk_report_generation::{GenerateOptions, ToReportDateTime};
use opentalk_signaling_core::{
    ChunkFormat, DestroyContext, Event, InitContext, ModuleContext, ObjectStorage,
    ObjectStorageError, SignalingModule, SignalingModuleDescription, SignalingModuleError,
    SignalingModuleFeatureDescription, SignalingModuleInitData, SignalingRoomId, VolatileStorage,
    assets::{AssetError, AssetSaved, NewAssetFileName, save_asset},
    control::{
        self,
        storage::{
            AttributeActions, ControlStorageParticipantAttributes, DISPLAY_NAME, JOINED_AT, KIND,
            LEFT_AT, ROLE, USER_ID,
        },
    },
};
use opentalk_types_common::{
    assets::{AssetFileKind, FileExtension, asset_file_kind},
    modules::ModuleId,
    time::{TimeZone, Timestamp},
    users::UserId,
};
use opentalk_types_signaling::{ParticipantId, ParticipationKind, Role};
use opentalk_types_signaling_meeting_report::{
    MODULE_ID,
    command::MeetingReportCommand,
    event::{Error, MeetingReportEvent, PdfAsset},
};
use snafu::{Report, ResultExt};
use storage::MeetingReportStorage;
use template::{ReportParticipant, ReportTemplateParameter};

pub mod exchange;
mod storage;
mod template;

const TEMPLATE: &str = include_str!("../templates/attendance_report.typ");
const FTL_EN: &str = include_str!("../templates/l10n/en.ftl");
const FTL_DE: &str = include_str!("../templates/l10n/de.ftl");
const AVAILABLE_LANGUAGES: &[LanguageIdentifier] = &[langid!("en"), langid!("de")];

trait MeetingReportStorageProvider {
    fn storage(&mut self) -> &mut dyn MeetingReportStorage;
}

impl MeetingReportStorageProvider for VolatileStorage {
    fn storage(&mut self) -> &mut dyn MeetingReportStorage {
        match self.as_mut() {
            Either::Left(v) => v,
            Either::Right(v) => v,
        }
    }
}

pub struct MeetingReport {
    system_default_language: LanguageIdentifier,
    typst_packages_path: PathBuf,
    room_id: SignalingRoomId,
    inventory_provider: Arc<dyn InventoryProvider>,
    storage: Arc<ObjectStorage>,
}

impl SignalingModuleDescription for MeetingReport {
    const MODULE_ID: ModuleId = MODULE_ID;
    const DESCRIPTION: &'static str =
        "Handles generation of meeting reports, e.g. participant list export";
    const FEATURES: &[SignalingModuleFeatureDescription] = &[];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetingReportParams {
    system_default_language: LanguageIdentifier,
    typst_packages_path: PathBuf,
}

#[async_trait::async_trait(?Send)]
impl SignalingModule for MeetingReport {
    const NAMESPACE: ModuleId = MODULE_ID;

    type Params = MeetingReportParams;

    type Incoming = MeetingReportCommand;

    type Outgoing = MeetingReportEvent;

    type ExchangeMessage = exchange::Event;

    type ExtEvent = ();

    type FrontendData = ();

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
            room_id: ctx.room_id(),
            inventory_provider: ctx.inventory_provider().clone(),
            storage: ctx.storage.clone(),
        }))
    }

    async fn on_event(
        &mut self,
        ctx: ModuleContext<'_, Self>,
        event: Event<'_, Self>,
    ) -> Result<(), SignalingModuleError> {
        match event {
            Event::Joined { .. } => {}
            Event::WsMessage(msg) => {
                self.handle_ws_message(ctx, msg).await?;
            }
            Event::Exchange(..)
            | Event::Ext(..)
            | Event::ParticipantJoined(..)
            | Event::Leaving
            | Event::RaiseHand
            | Event::LowerHand
            | Event::ParticipantUpdated(_, _)
            | Event::ParticipantLeft(_)
            | Event::RoleUpdated(_) => {}
        }

        Ok(())
    }

    async fn on_destroy(self, _ctx: DestroyContext<'_>) {}

    fn build_params(
        init: SignalingModuleInitData,
    ) -> Result<Option<Self::Params>, SignalingModuleError> {
        let Some(settings) = init.startup_settings.signaling.controller() else {
            return Ok(None);
        };

        let typst_packages_path = settings.reports.typst.packages_path.clone();
        let system_default_language = init.startup_settings.defaults.user_language.clone();
        Ok(Some(MeetingReportParams {
            system_default_language,
            typst_packages_path,
        }))
    }
}

impl MeetingReport {
    async fn handle_ws_message(
        &mut self,
        mut ctx: ModuleContext<'_, Self>,
        msg: MeetingReportCommand,
    ) -> Result<(), SignalingModuleError> {
        match msg {
            MeetingReportCommand::GenerateAttendanceReport {
                include_email_addresses,
            } => {
                if ctx.role() != Role::Moderator {
                    ctx.ws_send(Error::InsufficientPermissions);
                    return Ok(());
                }
                self.create_attendance_report(ctx, include_email_addresses)
                    .await?;
            }
        }
        Ok(())
    }

    async fn create_attendance_report(
        &mut self,
        mut ctx: ModuleContext<'_, Self>,
        include_email_addresses: bool,
    ) -> Result<(), SignalingModuleError> {
        let (participants, event, timezone, language) = self
            .collect_report_information(&mut ctx, include_email_addresses)
            .await?;

        let report = Self::generate_pdf_report(
            TEMPLATE.to_string(),
            Vec::from_iter(AVAILABLE_LANGUAGES.iter().cloned()),
            event,
            participants,
            timezone,
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

    async fn collect_report_information(
        &mut self,
        ctx: &mut ModuleContext<'_, Self>,
        include_email_addresses: bool,
    ) -> Result<
        (
            Vec<ReportParticipant>,
            InventoryEvent,
            TimeZone,
            LanguageIdentifier,
        ),
        SignalingModuleError,
    > {
        const CONCURRENT_PARTICIPANT_QUERIES: usize = 10;

        let mut inventory = self.inventory_provider.get_inventory().await?;
        let storage = ctx.volatile.storage();

        let event = inventory
            .get_event_for_room(self.room_id.room_id())
            .await?
            .ok_or(SignalingModuleError::NotFoundError {
                message: "Room not found".to_string(),
            })?;
        let room_owner = inventory.get_user(event.created_by).await?;

        // Query all participant IDs and create and concurrently fetch all participant information.
        let participants = storage.get_all_participants(self.room_id).await?;
        let participants = participants.iter().map(|p| async {
            let mut volatile = ctx.volatile.clone();
            let storage = volatile.storage();
            self.query_participant_report(storage, *p, include_email_addresses, &ctx.timezone)
                .await
        });

        let participants = stream::iter(participants)
            .buffer_unordered(CONCURRENT_PARTICIPANT_QUERIES)
            .try_collect::<Vec<ReportParticipant>>()
            .await
            .map_err(|e| SignalingModuleError::CustomError {
                message: "Failed to query participants".to_string(),
                source: Some(Box::new(e).into()),
            })?;

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

        Ok((participants, event, ctx.timezone, language))
    }

    async fn generate_pdf_report(
        template: String,
        available_languages: Vec<LanguageIdentifier>,
        event: InventoryEvent,
        participants: Vec<ReportParticipant>,
        report_timezone: TimeZone,
        report_language: LanguageIdentifier,
        typst_package_path: &Path,
    ) -> Result<Vec<u8>, SignalingModuleError> {
        let tz = Tz::from(report_timezone);

        let (starts_at, ends_at) = event
            .date()
            .map(|date| {
                let starts_at_dt = DateTime::from(date.starts_at).to_report_date_time(&tz);
                let ends_at_dt = {
                    let dt = DateTime::from(date.ends_at);
                    // Legacy check due to the missuse of the `starts_at` and
                    // `ends_at` fields in recurring events.
                    if date.ends_at.year() - date.starts_at.year() == 100 {
                        dt.with_year(dt.year() - 100).unwrap_or(dt)
                    } else {
                        dt
                    }
                }
                .to_report_date_time(&tz);

                (starts_at_dt, ends_at_dt)
            })
            .unzip();

        let current_time = Local::now();
        let timestamp = current_time.naive_local().format("%Y-%m-%dT%H:%M:%S.%f");
        let report_created_at = current_time.to_report_date_time(&tz);

        Self::generate_pdf_report_from_template(
            template,
            &ReportTemplateParameter {
                available_languages,
                title: event.title,
                description: event.description,
                starts_at,
                ends_at,
                report_created_at,
                report_timezone,
                participants,
                report_language,
            },
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

        let files = vec![
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
        ];

        let pdf = opentalk_report_generation::generate_pdf_report(
            template,
            BTreeMap::from_iter(files),
            &generate_options,
        )
        .whatever_context::<_, SignalingModuleError>("unable to build pdf")?;
        Ok(pdf)
    }

    async fn upload_pdf(&mut self, report: Vec<u8>, mut ctx: ModuleContext<'_, Self>) {
        const ASSET_FILE_KIND: AssetFileKind = asset_file_kind!("meeting_report");
        let file_name =
            NewAssetFileName::new(ASSET_FILE_KIND, Timestamp::now(), FileExtension::pdf());
        let report =
            async_stream::stream!(yield Result::<_, ObjectStorageError>::Ok(Bytes::from(report)));
        let report = Box::pin(report);
        let result = save_asset(
            &self.storage,
            self.inventory_provider.as_ref(),
            self.room_id.room_id(),
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
                log::debug!("Storage exceeded while storing meeting report");
                ctx.ws_send(MeetingReportEvent::Error(Error::StorageExceeded));
                return;
            }
            Err(e) => {
                log::error!(
                    "Error while storing attendance report: {}",
                    Report::from_error(e)
                );
                ctx.ws_send(MeetingReportEvent::Error(Error::Storage));
                return;
            }
        };

        let pdf_asset = PdfAsset { filename, asset_id };
        log::debug!("Generated meeting attendance report: {pdf_asset:?}");
        ctx.exchange_publish(
            control::exchange::current_room_all_participants(self.room_id),
            exchange::Event::PdfAsset(pdf_asset.clone()),
        );
        ctx.ws_send(MeetingReportEvent::PdfAsset(pdf_asset));
    }

    async fn query_participant_report(
        &self,
        storage: &mut dyn MeetingReportStorage,
        participant: ParticipantId,
        include_email_addresses: bool,
        tz: &Tz,
    ) -> Result<ReportParticipant, SignalingModuleError> {
        type UserData = (
            Option<DateTime<Utc>>,
            Option<DateTime<Utc>>,
            String,
            Role,
            ParticipationKind,
            Option<UserId>,
        );

        let (joined_at, left_at, name, role, kind, user_id): UserData = storage
            .bulk_attribute_actions(
                AttributeActions::new(self.room_id, participant)
                    .get_local(JOINED_AT)
                    .get_local(LEFT_AT)
                    .get_global(DISPLAY_NAME)
                    .get_global(ROLE)
                    .get_local(KIND)
                    .get_local(USER_ID),
            )
            .await?;

        let email = match (include_email_addresses, user_id) {
            (true, Some(user_id)) => {
                let user = self
                    .inventory_provider
                    .get_inventory()
                    .await?
                    .get_user(user_id)
                    .await?;

                Some(user.email)
            }
            _ => None,
        };

        // The report generator does not support sub-seconds resolution, so we
        // round the timestamps to seconds
        let joined_at = joined_at.to_report_date_time(tz);
        let left_at = left_at.to_report_date_time(tz);

        Ok(ReportParticipant {
            id: participant,
            name,
            joined_at,
            left_at,
            role,
            email,
            kind,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use insta::assert_snapshot;
    use opentalk_controller_settings::reports_typst_default_packages_path;

    use crate::{MODULE_ID, MeetingReport, TEMPLATE, template::ReportTemplateParameter};

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

        let pdf = MeetingReport::generate_pdf_report_from_template(
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
        assert_snapshot!(generate("small", &crate::template::tests::example_small()), @r"
        Attendance Report
         Meeting : Testmeeting

        Report created at : 2025-02-06 09:16

        Report timezone : Europe/Berlin

        Participants
         Nr Name Role

        1 Alice Adams Moderator
        ");
    }

    #[test]
    fn generate_report_medium() {
        assert_snapshot!(
            generate(
                "medium",
                &crate::template::tests::example_medium()
            ),
            @r"
        Attendance Report
         Meeting : Testmeeting

        Details : A medium sized test meeting

        Planned start : 2025-02-06 08:18

        Planned end : 2025-02-06 11:25

        Report created at : 2025-02-06 09:16

        Report timezone : Europe/Berlin

        Participants
         Nr Name Role

        1 Alice Adams Moderator

        2 Charlie Cooper User

        3 Bob Burton User
        "
        );
    }

    #[test]
    fn generate_report_medium_de() {
        let mut report_template_data = crate::template::tests::example_medium();
        report_template_data.report_language =
            "de".parse().expect("value must be parsable as Language");
        assert_snapshot!(
            generate(
                "medium_de",
                &report_template_data
            ),
            @r"
        Anwesenheitsbericht
         Meeting : Testmeeting

        Details : A medium sized test meeting

        Geplanter Beginn : 2025-02-06 08:18

        Geplantes Ende : 2025-02-06 11:25

        Bericht erstellt um : 2025-02-06 09:16

        Zeitzone des Berichts : Europe/Berlin

        Teilnehmende
         Nr Name Rolle

        1 Alice Adams Moderator

        2 Charlie Cooper Nutzer

        3 Bob Burton Nutzer
        "
        );
    }

    #[test]
    fn generate_report_large() {
        assert_snapshot!(generate("large", &crate::template::tests::example_large()), @r"
        Attendance Report
         Meeting : Large Testmeeting

        Details : The large test meeting

        Planned start : 2025-02-06 08:18

        Planned end : 2025-02-06 11:25

        Report created at : 2025-02-06 09:16

        Report timezone : Europe/Berlin

        Participants
         Nr Name Role

        1 Alice Adams Moderator

        2 Franz Fischer User

        3 Charlie Cooper User

        4 Bob Burton User

        5 Erin Guest

        6 Dave Dunn Guest
        ");
    }

    #[test]
    fn generate_report_large_de() {
        let mut report_template_data = crate::template::tests::example_large();
        report_template_data.report_language =
            "de".parse().expect("value must be parsable as Language");
        assert_snapshot!(
            generate(
                "large_de",
                &report_template_data
            ),
            @r"
        Anwesenheitsbericht
         Meeting : Large Testmeeting

        Details : The large test meeting

        Geplanter Beginn : 2025-02-06 08:18

        Geplantes Ende : 2025-02-06 11:25

        Bericht erstellt um : 2025-02-06 09:16

        Zeitzone des Berichts : Europe/Berlin

        Teilnehmende
         Nr Name Rolle

        1 Alice Adams Moderator

        2 Franz Fischer Nutzer

        3 Charlie Cooper Nutzer

        4 Bob Burton Nutzer

        5 Erin Gast

        6 Dave Dunn Gast
        "
        );
    }
}
