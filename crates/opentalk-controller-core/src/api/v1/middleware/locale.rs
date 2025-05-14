// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use actix_web::dev::ServiceRequest;
use fluent_langneg::{NegotiationStrategy, negotiate_languages, parse_accepted_languages};
use icu_locid::{LanguageIdentifier, langid};

pub(super) fn get_request_locale(req: &ServiceRequest) -> Option<LanguageIdentifier> {
    // These are the languages supported by the frontend at the moment
    const SUPPORTED_LOCALES: &[LanguageIdentifier] = &[langid!("de-DE"), langid!("en-US")];

    let accepted_languages_header_value = req
        .headers()
        .get("Accept-Language")
        .and_then(|hv| hv.to_str().ok())
        .unwrap_or_default();

    let languages_accepted_by_client = parse_accepted_languages(accepted_languages_header_value);
    let languages_available = negotiate_languages(
        &languages_accepted_by_client,
        SUPPORTED_LOCALES,
        None,
        NegotiationStrategy::Filtering,
    );

    languages_available
        .first()
        .and_then(|x| x.to_string().parse().ok())
}
