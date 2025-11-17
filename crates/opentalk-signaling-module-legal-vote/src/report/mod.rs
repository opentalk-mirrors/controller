// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

pub mod data;

use data::ReportData;
pub use error::Error;
use fluent_langneg::{NegotiationStrategy, negotiate_languages};
use icu_locid::{LanguageIdentifier, langid};
use opentalk_report_generation::GenerateOptions;
use opentalk_types_common::users::{DisplayName, UserId};

mod error;
mod report_data_builder;

use std::{collections::BTreeMap, path::Path};

use chrono_tz::Tz;
use error::ReportGenerationSnafu;
use report_data_builder::Builder;
use snafu::ResultExt as _;

use crate::storage::v1::ProtocolEntry;

const TEMPLATE: &str = include_str!("../../templates/legal_vote_report.typ");
const FTL_EN: &str = include_str!("../../templates/l10n/en.ftl");
const FTL_DE: &str = include_str!("../../templates/l10n/de.ftl");
const AVAILABLE_LANGUAGES: &[LanguageIdentifier] = &[langid!("en"), langid!("de")];

pub(crate) fn generate(
    user_names: BTreeMap<UserId, DisplayName>,
    protocol: Vec<ProtocolEntry>,
    timezone: &Tz,
    room_owner_language: &LanguageIdentifier,
    system_default_language: &LanguageIdentifier,
    dump_to_relative_path: &Path,
    typst_package_path: &Path,
) -> Result<Vec<u8>, Error> {
    let builder = Builder::new(user_names);

    let report_language = {
        let fallback = AVAILABLE_LANGUAGES
            .iter()
            .next()
            .expect("AVAILABLE_LANGUAGES is not empty");
        let system_default = system_default_language;

        negotiate_languages(
                &[room_owner_language, system_default],
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

    let report_data = builder.build_report_data(protocol, timezone, report_language)?;

    generate_from_template(
        TEMPLATE.to_string(),
        &report_data,
        dump_to_relative_path,
        typst_package_path,
    )
}

fn generate_from_template(
    template: String,
    parameter: &ReportData,
    dump_to_relative_path: &Path,
    typst_package_path: &Path,
) -> Result<Vec<u8>, Error> {
    let dump_to_path = std::env::var("OPENTALK_REPORT_DUMP_PATH")
        .map(|p| Path::new(&p).join(dump_to_relative_path))
        .ok();

    let mut generate_options = GenerateOptions::default();
    generate_options.dump_to_path = dump_to_path.as_deref();
    generate_options.packages_path = Some(typst_package_path);

    opentalk_report_generation::generate_pdf_report(
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
    .context(ReportGenerationSnafu)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use icu_locid::langid;
    use insta::assert_snapshot;
    use opentalk_controller_settings::reports_typst_default_packages_path;

    use super::{
        TEMPLATE,
        data::{
            ReportData,
            report_data::tests::{example_live_roll_call, example_pseudonymous, example_roll_call},
        },
        generate_from_template,
    };
    use crate::{MODULE_ID, report::data::report_data::tests::canceled_live_roll_call};

    fn generate(sample_name: &str, parameter: &ReportData) -> String {
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

        let pdf = generate_from_template(
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
    fn generate_report_live_roll_call() {
        assert_snapshot!(
            generate("live_roll_call", &example_live_roll_call()),
            @r"
        OpenTalk Vote Report
         Title : Weather Vote

        Subtitle : Another one of these weather votes

        Topic : Is the weather good today?

        Vote kind : Live roll call

        Referendum leader : Alice Adams

        Vote id : ee621ab4-72f6-4d39-bbc4-dc1b96a606cf

        Start : 2025-01-02 03:04:05

        End : 2025-01-02 03:09:05

        Report timezone : Europe/Berlin

        Participant count : 8

        Scheduled duration : 300 s

        Abstention : Disallowed

        Automatic close : Enabled

        Vote ended due to : All users voted

        Number of votes : 6

        Results
         Vote Count

        Approval 4

        Disapproval 2

        Abstention 5

        Recorded votes
         Name Token Vote Timestamp

        Alice Adams aaaaaaaa Approval 2025-01-02 03:04:24

        Bob Burton bbbbbbbb Disapproval 2025-01-02 03:04:20

        Charlie Cooper cccccccc Disapproval 2025-01-02 03:04:21

        Dave Dunn dddddddd Approval 2025-01-02 03:04:19

        Erin Eaton eeeeeeee Approval 2025-01-02 03:06:00

        George Grump gggggggg Approval 2025-01-02 03:06:00

        Event log
         Name Timestamp Event

        Charlie Cooper 2025-01-02 03:04:18 Reports a screenshare issue
        "
        );
    }

    #[test]
    fn generate_report_live_roll_call_de() {
        let mut data = example_live_roll_call();
        data.report_language = langid!("de");

        assert_snapshot!(
            generate("live_roll_call_de", &data),
            @r"
        OpenTalk Abstimmungsbericht
         Titel : Weather Vote

        Untertitel : Another one of these weather votes

        Thema : Is the weather good today?

        Abstimmungstyp : Offene Abstimmung - namentlich

        Abstimmungsleitung : Alice Adams

        Abstimmungs-ID : ee621ab4-72f6-4d39-bbc4-dc1b96a606cf

        Beginn : 2025-01-02 03:04:05

        Ende : 2025-01-02 03:09:05

        Zeitzone des Berichts : Europe/Berlin

        Anzahl Teilnehmende : 8

        Geplante Dauer : 300 s

        Enthaltung : nicht zulässig

        Automatisches Ende : aktiviert

        Abstimmung endete auf Grund von : Alle Stimmen abgegeben

        Anzahl abgegebener Stimmen : 6

        Ergebnisse
         Stimme Anzahl

        Zustimmung 4

        Ablehnung 2

        Enthaltung 5

        Recorded votes
         Name Token Stimme Zeitpunkt

        Alice Adams aaaaaaaa Zustimmung 2025-01-02 03:04:24

        Bob Burton bbbbbbbb Ablehnung 2025-01-02 03:04:20

        Charlie Cooper cccccccc Ablehnung 2025-01-02 03:04:21

        Dave Dunn dddddddd Zustimmung 2025-01-02 03:04:19

        Erin Eaton eeeeeeee Zustimmung 2025-01-02 03:06:00

        George Grump gggggggg Zustimmung 2025-01-02 03:06:00

        Ereignisprotokoll
         Name Zeitpunkt Ereignis

        Charlie Cooper 2025-01-02 03:04:18 Meldet ein Screenshare-Problem
        "
        );
    }

    #[test]
    fn generate_report_roll_call() {
        assert_snapshot!(
            generate("roll_call", &example_roll_call()),
            @r"
        OpenTalk Vote Report
         Title : End meeting early

        Subtitle : Should we end today's meeting earlier?

        Vote kind : Roll call

        Referendum leader : Alice Adams

        Vote id : 21ca8797-915a-4255-86d7-23ad0508905f

        Start : 2025-02-09 08:41:50

        End : 2025-02-09 08:42:50

        Report timezone : Europe/Vienna

        Participant count : 4

        Scheduled duration : 60 s

        Abstention : Disallowed

        Automatic close : Disabled

        Vote ended due to : Expired

        Number of votes : 3

        Results
         Vote Count

        Approval 2

        Disapproval 1

        Recorded votes
         Name Token Vote Timestamp

        Bob Burton WPFPHL6RH7Y Disapproval 2025-02-09 08:41:56

        Alice Adams FmrpkqvtHN8 Approval 2025-02-09 08:42:00

        Dave Dunn 538ks7CrBET Approval 2025-02-09 08:42:35

        Event log
         Name Timestamp Event

        Alice Adams 2025-02-09 08:42:16 Reports a problem : Misunderstood the question :-(

        Charlie Cooper 2025-02-09 08:42:26 User left

        Charlie Cooper 2025-02-09 08:42:28 User joined
        "
        );
    }

    #[test]
    fn generate_report_roll_call_de() {
        let mut data = example_roll_call();
        data.report_language = langid!("de");
        assert_snapshot!(
            generate("roll_call_de", &data),
            @r"
        OpenTalk Abstimmungsbericht
         Titel : End meeting early

        Untertitel : Should we end today's meeting earlier?

        Abstimmungstyp : Offene Abstimmung

        Abstimmungsleitung : Alice Adams

        Abstimmungs-ID : 21ca8797-915a-4255-86d7-23ad0508905f

        Beginn : 2025-02-09 08:41:50

        Ende : 2025-02-09 08:42:50

        Zeitzone des Berichts : Europe/Vienna

        Anzahl Teilnehmende : 4

        Geplante Dauer : 60 s

        Enthaltung : nicht zulässig

        Automatisches Ende : deakitivert

        Abstimmung endete auf Grund von : Abstimmungsdauer abgelaufen

        Anzahl abgegebener Stimmen : 3

        Ergebnisse
         Stimme Anzahl

        Zustimmung 2

        Ablehnung 1

        Recorded votes
         Name Token Stimme Zeitpunkt

        Bob Burton WPFPHL6RH7Y Ablehnung 2025-02-09 08:41:56

        Alice Adams FmrpkqvtHN8 Zustimmung 2025-02-09 08:42:00

        Dave Dunn 538ks7CrBET Zustimmung 2025-02-09 08:42:35

        Ereignisprotokoll
         Name Zeitpunkt Ereignis

        Alice Adams 2025-02-09 08:42:16 Meldet ein Problem : Misunderstood the question :-(

        Charlie Cooper 2025-02-09 08:42:26 Verlassen der Videokonferenz

        Charlie Cooper 2025-02-09 08:42:28 Beitritt zur Videokonferenz
        "
        );
    }

    #[test]
    fn generate_pseudonymous() {
        assert_snapshot!(generate("pseudonymous",&example_pseudonymous()),
        @r"
        OpenTalk Vote Report
         Title : Example Pseudonymous Vote

        Vote kind : Pseudonymous vote

        Referendum leader : Alice Adams

        Vote id : 6a3525fc-aeef-4d7e-9d76-e41ab2cbe469

        Start : 2025-02-08 12:32:09

        End : 2025-02-08 12:32:22

        Report timezone : Europe/Vienna

        Participant count : 4

        Scheduled duration : 60 s

        Abstention : Allowed

        Automatic close : Enabled

        Vote ended due to : All users voted

        Number of votes : 4

        Results
         Vote Count

        Approval 1

        Disapproval 2

        Abstention 1

        Recorded votes
         Name Token Vote Timestamp

        Hidden LPwNXJWs7b1 Approval —

        Hidden K5SMSt98f11 Disapproval —

        Hidden B1yWM5eWQQi Abstention —

        Hidden 8PCkuJ9NGoY Disapproval —

        Event log
         Name Timestamp Event
        ");
    }

    #[test]
    fn generate_pseudonymous_de() {
        let mut data = example_pseudonymous();
        data.report_language = langid!("de");
        assert_snapshot!(generate("pseudonymous_de",&data),
        @r"
        OpenTalk Abstimmungsbericht
         Titel : Example Pseudonymous Vote

        Abstimmungstyp : Geheime Abstimmung

        Abstimmungsleitung : Alice Adams

        Abstimmungs-ID : 6a3525fc-aeef-4d7e-9d76-e41ab2cbe469

        Beginn : 2025-02-08 12:32:09

        Ende : 2025-02-08 12:32:22

        Zeitzone des Berichts : Europe/Vienna

        Anzahl Teilnehmende : 4

        Geplante Dauer : 60 s

        Enthaltung : zulässig

        Automatisches Ende : aktiviert

        Abstimmung endete auf Grund von : Alle Stimmen abgegeben

        Anzahl abgegebener Stimmen : 4

        Ergebnisse
         Stimme Anzahl

        Zustimmung 1

        Ablehnung 2

        Enthaltung 1

        Recorded votes
         Name Token Stimme Zeitpunkt

        Versteckt LPwNXJWs7b1 Zustimmung —

        Versteckt K5SMSt98f11 Ablehnung —

        Versteckt B1yWM5eWQQi Enthaltung —

        Versteckt 8PCkuJ9NGoY Ablehnung —

        Ereignisprotokoll
         Name Zeitpunkt Ereignis
        ");
    }

    #[test]
    fn generate_canceled_live_roll_call() {
        assert_snapshot!(
            generate("canceled_live_roll_call", &canceled_live_roll_call()),
            @r"
        OpenTalk Vote Report
         Title : Weather Vote

        Subtitle : Another one of these weather votes

        Topic : Is the weather good today?

        Vote kind : Live roll call

        Referendum leader : Alice Adams

        Vote id : ee621ab4-72f6-4d39-bbc4-dc1b96a606cf

        Start : 2025-01-02 03:04:05

        End : 2025-01-02 03:09:05

        Report timezone : Europe/Berlin

        Participant count : 8

        Scheduled duration : 300 s

        Abstention : Disallowed

        Automatic close : Enabled

        Vote ended due to : Aborted for custom reason:   test

        Number of votes : 6

        Recorded votes
         Name Token Vote Timestamp

        Alice Adams aaaaaaaa Approval 2025-01-02 03:04:24

        Bob Burton bbbbbbbb Disapproval 2025-01-02 03:04:20

        Charlie Cooper cccccccc Disapproval 2025-01-02 03:04:21

        Dave Dunn dddddddd Approval 2025-01-02 03:04:19

        Erin Eaton eeeeeeee Approval 2025-01-02 03:06:00

        George Grump gggggggg Approval 2025-01-02 03:06:00

        Event log
         Name Timestamp Event

        Charlie Cooper 2025-01-02 03:04:18 Reports a screenshare issue
        "
        );
    }

    #[test]
    fn generate_canceled_live_roll_call_de() {
        let mut data = canceled_live_roll_call();
        data.report_language = langid!("de");
        assert_snapshot!(
            generate("canceled_live_roll_call_de", &data),
            @r"
        OpenTalk Abstimmungsbericht
         Titel : Weather Vote

        Untertitel : Another one of these weather votes

        Thema : Is the weather good today?

        Abstimmungstyp : Offene Abstimmung - namentlich

        Abstimmungsleitung : Alice Adams

        Abstimmungs-ID : ee621ab4-72f6-4d39-bbc4-dc1b96a606cf

        Beginn : 2025-01-02 03:04:05

        Ende : 2025-01-02 03:09:05

        Zeitzone des Berichts : Europe/Berlin

        Anzahl Teilnehmende : 8

        Geplante Dauer : 300 s

        Enthaltung : nicht zulässig

        Automatisches Ende : aktiviert

        Abstimmung endete auf Grund von : Abgebrochen mit benutzerdefiniertem Grund:   test

        Anzahl abgegebener Stimmen : 6

        Recorded votes
         Name Token Stimme Zeitpunkt

        Alice Adams aaaaaaaa Zustimmung 2025-01-02 03:04:24

        Bob Burton bbbbbbbb Ablehnung 2025-01-02 03:04:20

        Charlie Cooper cccccccc Ablehnung 2025-01-02 03:04:21

        Dave Dunn dddddddd Zustimmung 2025-01-02 03:04:19

        Erin Eaton eeeeeeee Zustimmung 2025-01-02 03:06:00

        George Grump gggggggg Zustimmung 2025-01-02 03:06:00

        Ereignisprotokoll
         Name Zeitpunkt Ereignis

        Charlie Cooper 2025-01-02 03:04:18 Meldet ein Screenshare-Problem
        "
        );
    }
}
