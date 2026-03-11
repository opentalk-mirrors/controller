// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
// SPDX-License-Identifier: EUPL-1.2

use core::fmt;
use std::time::Duration;

use actix_http::{StatusCode, header::AUTHORIZATION};
use actix_web::{
    HttpRequest, HttpResponse, ResponseError,
    body::BoxBody,
    http::header::{HeaderName as ActixHeaderName, HeaderValue as ActixHeaderValue},
    routes,
    web::{self, Data, Query},
};
use actix_ws::AggregatedMessage;
use bytestring::ByteString;
use futures::{SinkExt, StreamExt};
use jsonwebtoken::{DecodingKey, Validation};
use livekit_api::access_token::Claims;
use opentalk_controller_settings::{ControllerSignaling, SettingsProvider};
use opentalk_signaling_core::{
    SignalingModuleError, SignalingRoomId, VolatileStorage,
    control::{
        ControlStorageProvider,
        storage::{ControlStorageParticipantAttributes, JOINED_AT, LEFT_AT, LocalRoomAttributeId},
    },
};
use opentalk_signaling_module_subroom_audio::SubroomAudioStorageProvider;
use opentalk_types_common::time::Timestamp;
use opentalk_types_signaling::ParticipantId;
use opentalk_types_signaling_subroom_audio::whisper_id::WhisperId;
use reqwest::header::{
    HeaderMap as ReqwestHeaderMap, HeaderName as ReqwestHeaderName,
    HeaderValue as ReqwestHeaderValue,
};
use serde::Deserialize;
use snafu::Report;
use tokio::time::{Instant, interval_at};
use tokio_tungstenite::tungstenite::{self, Utf8Bytes, client::IntoClientRequest};
use url::Url;

#[derive(Debug)]
pub struct ProxyError(StatusCode);

impl fmt::Display for ProxyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ResponseError for ProxyError {
    fn status_code(&self) -> StatusCode {
        self.0
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        HttpResponse::new(self.status_code())
    }
}

#[derive(Deserialize)]
struct LiveKitQuery {
    access_token: Option<String>,
}

#[routes]
#[get("/rtc/validate")]
#[get("/rtc/v1/validate")]
pub async fn proxy_validate(
    settings: Data<SettingsProvider>,
    req: HttpRequest,
    body: web::Payload,
    http_client: Data<reqwest::Client>,
) -> actix_web::Result<HttpResponse> {
    let settings = settings.get();
    let Some(signaling) = settings.signaling.controller() else {
        return Err(ProxyError(StatusCode::UNPROCESSABLE_ENTITY).into());
    };

    let path = if req.uri().path().ends_with("v1/validate") {
        "rtc/v1/validate"
    } else {
        "rtc/validate"
    };

    let mut livekit_url = build_livekit_url(signaling, path)?;
    livekit_url.set_query(req.uri().query());

    let mut headers = ReqwestHeaderMap::new();
    for (header_name, header_value) in req.headers() {
        let name = ReqwestHeaderName::from_bytes(header_name.as_str().as_bytes())
            .map_err(|_| ProxyError(StatusCode::BAD_REQUEST))?;
        let value = ReqwestHeaderValue::from_bytes(header_value.as_bytes())
            .map_err(|_| ProxyError(StatusCode::BAD_REQUEST))?;

        headers.append(name, value);
    }

    let body = body
        .to_bytes()
        .await
        .map_err(|_| ProxyError(StatusCode::BAD_REQUEST))?;

    let response = http_client
        .get(livekit_url)
        .headers(headers)
        .body(body)
        .send()
        .await
        .map_err(|_| ProxyError(StatusCode::INTERNAL_SERVER_ERROR))?;

    reqwest_response_to_actix(response).await
}

async fn reqwest_response_to_actix(
    reqwest_response: reqwest::Response,
) -> Result<HttpResponse, actix_web::Error> {
    let status = reqwest_response.status().as_u16();
    let status =
        StatusCode::from_u16(status).map_err(|_| ProxyError(StatusCode::INTERNAL_SERVER_ERROR))?;

    let mut actix_response = HttpResponse::build(status);

    for (header_name, header_value) in reqwest_response.headers() {
        let name = ActixHeaderName::from_bytes(header_name.as_str().as_bytes())
            .map_err(|_| ProxyError(StatusCode::BAD_REQUEST))?;
        let value = ActixHeaderValue::from_bytes(header_value.as_bytes())
            .map_err(|_| ProxyError(StatusCode::BAD_REQUEST))?;

        actix_response.append_header((name, value));
    }

    let response_body = reqwest_response
        .bytes()
        .await
        .map_err(|_| ProxyError(StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(actix_response.body(response_body))
}

#[routes]
#[get("/rtc")]
#[get("/rtc/v1")]
pub async fn proxy_signaling(
    settings: Data<SettingsProvider>,
    req: HttpRequest,
    body: web::Payload,
    Query(query): Query<LiveKitQuery>,
    volatile: Data<VolatileStorage>,
) -> actix_web::Result<HttpResponse> {
    let mut volatile = (**volatile).clone();

    let settings = settings.get();
    let Some(signaling) = settings.signaling.controller() else {
        return Err(ProxyError(StatusCode::UNPROCESSABLE_ENTITY).into());
    };

    let access_token_inside_query = query.access_token.is_some();
    let access_token = if let Some(access_token) = query.access_token {
        access_token
    } else {
        let Some(access_token) = req
            .headers()
            .get(&AUTHORIZATION)
            .and_then(|header_value| header_value.to_str().ok())
            .and_then(|header_value| {
                let (scheme, token) = header_value.split_once(' ')?;

                if scheme.eq_ignore_ascii_case("bearer") {
                    Some(token.to_string())
                } else {
                    None
                }
            })
        else {
            return Err(ProxyError(StatusCode::FORBIDDEN).into());
        };

        access_token
    };

    let decoding_key = DecodingKey::from_secret(signaling.livekit.api_secret.as_bytes());
    let decoded = match jsonwebtoken::decode::<Claims>(
        &access_token,
        &decoding_key,
        &Validation::new(jsonwebtoken::Algorithm::HS256),
    ) {
        Ok(decoded) => decoded,
        Err(e) => {
            log::debug!("Failed to verify & decode access_token, {e}");
            return Err(ProxyError(StatusCode::FORBIDDEN).into());
        }
    };

    let identity = decoded.claims.sub;
    let room = decoded.claims.video.room;

    let (room, whisper_id) = parse_livekit_room(room)?;

    let participant_id = identity
        .parse()
        .map_err(|_| ProxyError(StatusCode::BAD_REQUEST))?;

    assert_user_is_in_room(&mut volatile, room, participant_id, whisper_id).await?;

    let mut livekit_url = build_livekit_url(signaling, "rtc")?;

    match livekit_url.scheme() {
        "https" => livekit_url
            .set_scheme("wss")
            .expect("wss is a valid scheme"),
        "http" => livekit_url.set_scheme("ws").expect("ws is a valid scheme"),
        scheme => {
            log::error!("Invalid scheme in livekit.service_url, {scheme:?}");
            return Err(ProxyError(StatusCode::INTERNAL_SERVER_ERROR).into());
        }
    }

    livekit_url.set_query(Some(req.query_string()));

    let mut websocket_request = livekit_url.into_client_request().map_err(|e| {
        log::error!("Failed to build websocket request for livekit, {e:?}");
        ProxyError(StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    if !access_token_inside_query {
        let bearer_token =
            tungstenite::http::HeaderValue::from_str(&format!("Bearer {}", access_token)).map_err(
                |e| {
                    log::error!("Failed to build bearer header value, {e:?}");
                    ProxyError(StatusCode::INTERNAL_SERVER_ERROR)
                },
            )?;

        websocket_request
            .headers_mut()
            .append(tungstenite::http::header::AUTHORIZATION, bearer_token);
    }

    let (stream, _response) = tokio_tungstenite::connect_async(websocket_request)
        .await
        .map_err(|e| {
            log::warn!("Failed to connect to livekit signaling, {e:?}");
            ProxyError(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

    let (response, to_client, from_client) = actix_ws::handle(&req, body)?;

    actix_rt::spawn(connection_task(
        volatile,
        room,
        participant_id,
        whisper_id,
        to_client,
        from_client,
        stream,
    ));

    Ok(response)
}

fn build_livekit_url(signaling: &ControllerSignaling, path: &str) -> Result<Url, actix_web::Error> {
    let livekit_url = signaling.livekit.service_url.parse::<Url>().map_err(|e| {
        log::error!("Failed to parse livekit.service_url, {e:?}");
        ProxyError(StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    let livekit_url = livekit_url.join(path).map_err(|e| {
        log::error!("Failed to join livekit.service_url with rtc/validate, {e:?}");
        ProxyError(StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    Ok(livekit_url)
}

/// OpenTalk creates the livekit room name from the following pattern: `{room_id}:{breakout_room_id}#{whisper_id}`,
/// where `breakout_room_id` and `whisper_id` are optional
fn parse_livekit_room(
    room: String,
) -> Result<(SignalingRoomId, Option<WhisperId>), actix_web::Error> {
    let bad_request = |_| ProxyError(StatusCode::BAD_REQUEST);

    // Split room-id & whisper-id at #
    let (room, whisper_id) = match room.split_once('#') {
        Some((room, whisper_id)) => {
            let whisper_id = whisper_id.parse().map_err(bad_request)?;

            (room, Some(whisper_id))
        }
        None => (room.as_str(), None),
    };

    // Split signaling-room-id into room and breakout
    let (room, breakout) = match room.split_once(':') {
        Some((room, breakout)) => {
            let breakout = breakout.parse().map_err(bad_request)?;

            (room, Some(breakout))
        }
        None => (room, None),
    };

    let room = room.parse().map_err(bad_request)?;

    Ok((SignalingRoomId::new(room, breakout), whisper_id))
}

async fn connection_task(
    mut volatile: VolatileStorage,
    room: SignalingRoomId,
    participant_id: ParticipantId,
    whisper_id: Option<WhisperId>,
    mut to_client: actix_ws::Session,
    from_client: actix_ws::MessageStream,
    mut livekit_connection: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) {
    let recheck_period = Duration::from_secs(5);
    let mut recheck_interval = interval_at(Instant::now() + recheck_period, recheck_period);
    let mut from_client = from_client.aggregate_continuations();

    loop {
        tokio::select! {
            from_client = from_client.next() => {
                let Some(Ok(message)) = from_client else {
                    return;
                };

                let message = match message {
                    AggregatedMessage::Text(byte_string) => tungstenite::Message::Text(unsafe {
                        Utf8Bytes::from_bytes_unchecked(byte_string.into_bytes())
                    }),
                    AggregatedMessage::Binary(bytes) => tungstenite::Message::Binary(bytes),
                    AggregatedMessage::Close(_close_reason) => tungstenite::Message::Close(None),
                    AggregatedMessage::Ping(ping) => {
                        if let Err(e) = to_client.send(actix_ws::Message::Pong(ping)).await {
                            log::warn!("Failed to send Pong, {e}");
                            return;
                        }

                        continue;
                    }
                    AggregatedMessage::Pong(..) => {
                        continue;
                    }
                };

                if let Err(e) = livekit_connection.send(message).await {
                    log::warn!("Failed to forward message to livekit: {e}");
                    return;
                }
            }
            from_livekit = livekit_connection.next() => {
                let Some(Ok(message)) = from_livekit else {
                    return;
                };

                let message = match message {
                    tungstenite::Message::Text(utf8_bytes) => actix_ws::Message::Text(unsafe {
                        ByteString::from_bytes_unchecked(utf8_bytes.into())
                    }),
                    tungstenite::Message::Binary(bytes) => actix_ws::Message::Binary(bytes),
                    tungstenite::Message::Close(_close_frame) => actix_ws::Message::Close(None),
                    tungstenite::Message::Ping(..)
                    | tungstenite::Message::Pong(..)
                    | tungstenite::Message::Frame(..) => {
                        // Handled by tungstenite
                        continue;
                    }
                };

                if let Err(e) = to_client.send(message).await {
                    log::warn!("Failed to forward message from livekit: {e}");
                    return;
                }
            }
            _ = recheck_interval.tick() => {
                if assert_user_is_in_room(&mut volatile, room, participant_id, whisper_id).await.is_err() {
                    return;
                }
            }
        }
    }
}

async fn assert_user_is_in_room(
    volatile: &mut VolatileStorage,
    room: SignalingRoomId,
    participant_id: ParticipantId,
    whisper_id: Option<WhisperId>,
) -> Result<(), actix_web::Error> {
    let (joined_at, left_at) = get_participant_attributes(volatile, room, participant_id)
        .await
        .map_err(|e| {
            log::warn!(
                "Failed to fetch participant attributes, {:?}",
                Report::from_error(e)
            );
            ProxyError(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

    if !(joined_at.is_some() && left_at.is_none()) {
        log::warn!(
            "Closing livekit connection due to identity ({participant_id}) not being present in the requested room ({room}) on the controller"
        );

        Err(ProxyError(StatusCode::FORBIDDEN).into())
    } else {
        // Verify that the user is in the whisper group
        if let Some(whisper_id) = whisper_id {
            let is_in_whisper_group = volatile
                .subroom_audio_storage()
                .is_participant_in_whisper_group(room, whisper_id, participant_id)
                .await
                .map_err(|e| {
                    log::warn!(
                        "Failed to test if participant is in whisper group, {:?}",
                        Report::from_error(e)
                    );
                    ProxyError(StatusCode::INTERNAL_SERVER_ERROR)
                })?;

            if !is_in_whisper_group {
                return Err(ProxyError(StatusCode::FORBIDDEN).into());
            }
        }

        Ok(())
    }
}

async fn get_participant_attributes(
    volatile: &mut VolatileStorage,
    room: SignalingRoomId,
    participant_id: ParticipantId,
) -> Result<(Option<Timestamp>, Option<Timestamp>), SignalingModuleError> {
    let joined_at = volatile
        .control_storage()
        .get_attribute::<Timestamp>(
            participant_id,
            opentalk_signaling_core::control::storage::RoomAttributeId::Local(
                LocalRoomAttributeId {
                    room,
                    attribute: JOINED_AT,
                },
            ),
        )
        .await?;

    let left_at = volatile
        .control_storage()
        .get_attribute::<Timestamp>(
            participant_id,
            opentalk_signaling_core::control::storage::RoomAttributeId::Local(
                LocalRoomAttributeId {
                    room,
                    attribute: LEFT_AT,
                },
            ),
        )
        .await?;

    Ok((joined_at, left_at))
}
