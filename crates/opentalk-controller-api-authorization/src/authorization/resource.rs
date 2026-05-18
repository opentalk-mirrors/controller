// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_api_v1::events::InstanceId;
use opentalk_types_common::{
    assets::AssetId,
    events::EventId,
    rooms::{RoomId, invite_codes::InviteCode},
    streaming::StreamingTargetId,
    users::UserId,
};

/// Specification of a resource provided by the OpenTalk Controller API.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Resource {
    /// The events list resource.
    ///
    /// Served under `/v1/events`.
    Events,

    /// The event instances list resource.
    ///
    /// Served under `/v1/events/instances`.
    EventsInstances,

    /// An event resource.
    ///
    /// Served under `/v1/events/{event_id}`.
    Event(EventId),

    /// The list of invites for an event resource.
    ///
    /// Served under `/v1/events/{event_id}/invites`.
    EventInvites(EventId),

    /// An event e-mail invite resource.
    ///
    /// Served under `/v1/events/{event_id}/invites/email`.
    EventEmailInvite(EventId),

    /// An event user invite resource.
    ///
    /// Served under `/v1/events/{event_id}/invites/{user_id}`.
    EventUserInvite(EventId, UserId),

    /// An event invite resource.
    ///
    /// Served under `/v1/events/{event_id}/invite`.
    EventInvite(EventId),

    /// The instances of an event.
    ///
    /// Served under `/v1/events/{event_id}/instances`.
    EventInstances(EventId),

    /// The shared folder of an event.
    ///
    /// Served under `/v1/events/{event_id}/shared_folder`.
    EventSharedFolder(EventId),

    /// An event instance resource.
    ///
    /// Served under `/v1/events/{event_id}/instances/{instance_id}`.
    EventInstance(EventId, InstanceId),

    /// The rooms list resource.
    ///
    /// Served under `/v1/rooms`.
    Rooms,

    /// A room resource.
    ///
    /// Served under `/v1/rooms/{room_id}`.
    Room(RoomId),

    /// A room event resource.
    ///
    /// Served under `/v1/rooms/{room_id}/event`.
    RoomEvent(RoomId),

    /// The list of invites to a room.
    ///
    /// Served under `/v1/rooms/{room_id}/invites`.
    RoomInvites(RoomId),

    /// A room invite code.
    ///
    /// Served under `/v1/rooms/{room_id}/invites/{invite_code}`.
    RoomInviteCode(RoomId, InviteCode),

    /// The list of assets for a room.
    ///
    /// Served under `/v1/rooms/{room_id}/assets`.
    RoomAssets(RoomId),

    /// An asset stored with a room.
    ///
    /// Served under `/v1/rooms/{room_id}/assets/{asset_id}`.
    RoomAsset(RoomId, AssetId),

    /// An asset download for a room.
    ///
    /// Served under `/v1/rooms/{room_id}/assets/{asset_id}/download`.
    RoomAssetDownload(RoomId, AssetId),

    /// The list of streaming targets for a room.
    ///
    /// Served under `/v1/rooms/{room_id}/streaming_targets`.
    RoomStreamingTargets(RoomId),

    /// A streaming target for a room.
    ///
    /// Served under `/v1/rooms/{room_id}/streaming_targets/{streaming_target_id}`.
    RoomStreamingTarget(RoomId, StreamingTargetId),

    /// The sip config for a room.
    ///
    /// Served under `/v1/rooms/{room_id}/sip`.
    RoomSip(RoomId),

    /// The room start endpoint.
    ///
    /// Served under `/v1/rooms/{room_id}/start`.
    RoomStart(RoomId),

    /// The room tariff endpoint.
    ///
    /// Served under `/v1/rooms/{room_id}/tariff`.
    RoomTariff(RoomId),

    /// The user search.
    ///
    /// Served under `/v1/users/find`.
    UserFind,

    /// The user profile information.
    ///
    /// Served under `/v1/users/me`.
    UserMe,

    /// The user assets.
    ///
    /// Served under `/v1/users/me/assets`.
    UserMeAssets,

    /// The user event favorite.
    ///
    /// Served under `/v1/users/me/event_favorites/{user_id}`.
    UserMeEventFavorite(EventId),

    /// The pending invites for a user.
    ///
    /// Served under `/v1/users/me/pending_invites`.
    UserMePendingInvites,

    /// The user profile tariff information.
    ///
    /// Served under `/v1/users/me/tariff`.
    UserMeTariff,

    /// The public profile of a user.
    ///
    /// Served under `/v1/users/{user_id}`.
    UserProfile(UserId),
}

#[cfg(feature = "actix-web")]
pub(super) mod actix_web_impls {
    use actix_router::{Path, PathDeserializer, ResourceDef};
    use actix_web::{dev::ServiceRequest, error::PathError};
    use serde::de::DeserializeOwned;
    use snafu::{Snafu, ensure};

    use super::*;

    #[derive(Debug, Snafu)]
    pub enum TryFromResourceError {
        #[snafu(display("Unknown resource path {path:?}"))]
        UnknownResourcePath { path: String },

        #[snafu(display("Unknown resource path pattern {pattern:?}"))]
        UnknownResourcePathPattern { pattern: String },
    }

    impl TryFrom<&ServiceRequest> for Resource {
        type Error = TryFromResourceError;

        fn try_from(req: &ServiceRequest) -> Result<Self, Self::Error> {
            ensure!(
                req.resource_map().has_resource(req.path()),
                UnknownResourcePathSnafu {
                    path: req.path().to_string()
                }
            );
            let Some(pattern) = req.resource_map().match_pattern(req.path()) else {
                return UnknownResourcePathSnafu {
                    path: req.path().to_string(),
                }
                .fail()?;
            };

            match pattern.as_str() {
                "/v1/events" => Ok(Resource::Events),
                "/v1/events/instances" => Ok(Resource::EventsInstances),
                "/v1/events/{event_id}" => {
                    let event_id = extract_path::<EventId>(req.path(), &pattern).expect("invalid");
                    Ok(Resource::Event(event_id))
                }
                "/v1/events/{event_id}/invites" => {
                    let event_id = extract_path::<EventId>(req.path(), &pattern).expect("invalid");
                    Ok(Resource::EventInvites(event_id))
                }
                "/v1/events/{event_id}/invites/email" => {
                    let event_id = extract_path::<EventId>(req.path(), &pattern).expect("invalid");
                    Ok(Resource::EventEmailInvite(event_id))
                }
                "/v1/events/{event_id}/invites/{user_id}" => {
                    let (event_id, user_id) =
                        extract_path::<(EventId, UserId)>(req.path(), &pattern).expect("invalid");
                    Ok(Resource::EventUserInvite(event_id, user_id))
                }
                "/v1/events/{event_id}/invite" => {
                    let event_id = extract_path::<EventId>(req.path(), &pattern).expect("invalid");
                    Ok(Resource::EventInvite(event_id))
                }
                "/v1/events/{event_id}/instances" => {
                    let event_id = extract_path::<EventId>(req.path(), &pattern).expect("invalid");
                    Ok(Resource::EventInstances(event_id))
                }
                "/v1/events/{event_id}/instances/{instance_id}" => {
                    let (event_id, instance_id) =
                        extract_path::<(EventId, InstanceId)>(req.path(), &pattern)
                            .expect("invalid");
                    Ok(Resource::EventInstance(event_id, instance_id))
                }
                "/v1/events/{event_id}/shared_folder" => {
                    let event_id = extract_path::<EventId>(req.path(), &pattern).expect("invalid");
                    Ok(Resource::EventSharedFolder(event_id))
                }
                "/v1/rooms" => Ok(Resource::Rooms),
                "/v1/rooms/{room_id}" => {
                    let room_id = extract_path::<RoomId>(req.path(), &pattern).expect("invalid");
                    Ok(Resource::Room(room_id))
                }
                "/v1/rooms/{room_id}/event" => {
                    let room_id = extract_path::<RoomId>(req.path(), &pattern).expect("invalid");
                    Ok(Resource::RoomEvent(room_id))
                }
                "/v1/rooms/{room_id}/invites" => {
                    let room_id = extract_path::<RoomId>(req.path(), &pattern).expect("invalid");
                    Ok(Resource::RoomInvites(room_id))
                }
                "/v1/rooms/{room_id}/invites/{invite_code}" => {
                    let (room_id, invite_code) =
                        extract_path::<(RoomId, InviteCode)>(req.path(), &pattern)
                            .expect("invalid");
                    Ok(Resource::RoomInviteCode(room_id, invite_code))
                }
                "/v1/rooms/{room_id}/assets" => {
                    let room_id = extract_path::<RoomId>(req.path(), &pattern).expect("invalid");
                    Ok(Resource::RoomAssets(room_id))
                }
                "/v1/rooms/{room_id}/assets/{asset_id}" => {
                    let (room_id, asset_id) =
                        extract_path::<(RoomId, AssetId)>(req.path(), &pattern).expect("invalid");
                    Ok(Resource::RoomAsset(room_id, asset_id))
                }
                "/v1/rooms/{room_id}/assets/{asset_id}/download" => {
                    let (room_id, asset_id) =
                        extract_path::<(RoomId, AssetId)>(req.path(), &pattern).expect("invalid");
                    Ok(Resource::RoomAssetDownload(room_id, asset_id))
                }
                "/v1/rooms/{room_id}/sip" => {
                    let room_id = extract_path::<RoomId>(req.path(), &pattern).expect("invalid");
                    Ok(Resource::RoomSip(room_id))
                }
                "/v1/rooms/{room_id}/streaming_targets" => {
                    let room_id = extract_path::<RoomId>(req.path(), &pattern).expect("invalid");
                    Ok(Resource::RoomStreamingTargets(room_id))
                }
                "/v1/rooms/{room_id}/streaming_targets/{streaming_target_id}" => {
                    let (room_id, streaming_target_id) =
                        extract_path::<(RoomId, StreamingTargetId)>(req.path(), &pattern)
                            .expect("invalid");
                    Ok(Resource::RoomStreamingTarget(room_id, streaming_target_id))
                }
                "/v1/rooms/{room_id}/start" | "/v1/rooms/{room_id}/roomserver/start" => {
                    let room_id = extract_path::<RoomId>(req.path(), &pattern).expect("invalid");
                    Ok(Resource::RoomStart(room_id))
                }
                "/v1/rooms/{room_id}/tariff" => {
                    let room_id = extract_path::<RoomId>(req.path(), &pattern).expect("invalid");
                    Ok(Resource::RoomTariff(room_id))
                }
                "/v1/users/find" => Ok(Resource::UserFind),
                "/v1/users/me" => Ok(Resource::UserMe),
                "/v1/users/me/assets" => Ok(Resource::UserMeAssets),
                "/v1/users/me/event_favorites/{event_id}" => {
                    let event_id = extract_path::<EventId>(req.path(), &pattern).expect("invalid");
                    Ok(Resource::UserMeEventFavorite(event_id))
                }
                "/v1/users/me/pending_invites" => Ok(Resource::UserMePendingInvites),
                "/v1/users/me/tariff" => Ok(Resource::UserMeTariff),
                "/v1/users/{user_id}" => {
                    let user_id = extract_path::<UserId>(req.path(), &pattern).expect("invalid");
                    Ok(Resource::UserProfile(user_id))
                }
                pattern => UnknownResourcePathPatternSnafu {
                    pattern: pattern.to_string(),
                }
                .fail(),
            }
        }
    }

    fn extract_path<T: DeserializeOwned>(path: &str, pattern: &str) -> Result<T, actix_web::Error> {
        let resource = ResourceDef::prefix(pattern);
        let mut path = Path::new(path);

        let matches = resource.capture_match_info(&mut path);
        if !matches {
            tracing::warn!("Path {path:?} didn't match pattern {pattern:?}");
        }

        let path: T = serde::de::Deserialize::deserialize(PathDeserializer::new(&path))
            .map_err(move |err| {
                tracing::debug!(
                    "Failed during Path extractor deserialization. \
                         Request path: {path:?}",
                );

                PathError::Deserialize(err)
            })
            .map_err(actix_web::Error::from)?;
        Ok(path)
    }
}
