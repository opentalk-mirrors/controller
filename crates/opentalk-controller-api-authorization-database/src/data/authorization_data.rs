// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::collections::{BTreeMap, BTreeSet};

use opentalk_controller_api_authorization::authorization::{
    AccessMethod, Admission, AuthorizationChange, AuthorizationTarget, Resource, SubjectCollection,
};
use opentalk_types_common::{
    assets::AssetId,
    events::{EventId, invites::InviteRole},
    rooms::{GuestAccess, RoomId, invite_codes::InviteCode},
    streaming::StreamingTargetId,
    time::Timestamp,
    users::{GroupId, UserId},
};

use super::{Event, Group, Room};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuthorizationData {
    pub rooms: BTreeMap<RoomId, Room>,
    pub events: BTreeMap<EventId, Event>,
    pub groups: BTreeMap<GroupId, Group>,
    pub users: BTreeSet<UserId>,
}

impl AuthorizationData {
    pub fn build_from_changeset(changeset: &[AuthorizationChange]) -> Self {
        let mut result = Self::default();
        result.apply_changes(changeset);
        result
    }

    pub fn authorize(
        &self,
        AuthorizationTarget {
            authenticated_subjects,
            resource,
            access_method,
        }: AuthorizationTarget,
    ) -> Admission {
        tracing::debug!(
            "Authorizing resource {resource:?} with access method {access_method:?} for authenticated subjects {authenticated_subjects:?}"
        );
        match resource {
            Resource::Events => self.authorize_events(authenticated_subjects, access_method),
            Resource::EventsInstances => {
                self.authorize_events_instances(authenticated_subjects, access_method)
            }
            Resource::Event(event_id) => {
                self.authorize_event(authenticated_subjects, access_method, event_id)
            }
            Resource::EventInvites(event_id) => {
                self.authorize_event_invites(authenticated_subjects, access_method, event_id)
            }
            Resource::EventEmailInvite(event_id) => {
                self.authorize_event(authenticated_subjects, access_method, event_id)
            }
            Resource::EventUserInvite(event_id, _user_id) => {
                self.authorize_event(authenticated_subjects, access_method, event_id)
            }
            Resource::EventInstances(event_id) => {
                self.authorize_event_instances(authenticated_subjects, access_method, event_id)
            }
            Resource::EventInstance(event_id, _instance_id) => {
                self.authorize_event(authenticated_subjects, access_method, event_id)
            }
            Resource::EventInvite(event_id) => {
                self.authorize_event_invite(authenticated_subjects, access_method, event_id)
            }
            Resource::EventSharedFolder(event_id) => {
                self.authorize_event_shared_folder(authenticated_subjects, access_method, event_id)
            }
            Resource::Rooms => self.authorize_rooms(authenticated_subjects, access_method),
            Resource::Room(room_id) => {
                self.authorize_room(authenticated_subjects, access_method, room_id)
            }
            Resource::RoomInviteCode(room_id, invite_code) => self.authorize_room_invite_code(
                authenticated_subjects,
                access_method,
                room_id,
                invite_code,
            ),
            Resource::RoomAssets(room_id) => {
                self.authorize_room_assets(authenticated_subjects, access_method, room_id)
            }
            Resource::RoomAsset(room_id, asset_id) => {
                self.authorize_room_asset(authenticated_subjects, access_method, room_id, asset_id)
            }
            Resource::RoomAssetDownload(room_id, asset_id) => {
                self.authorize_room_asset(authenticated_subjects, access_method, room_id, asset_id)
            }
            Resource::RoomStreamingTargets(room_id) => self.authorize_room_streaming_targets(
                authenticated_subjects,
                access_method,
                room_id,
            ),
            Resource::RoomStreamingTarget(room_id, streaming_target_id) => self
                .authorize_room_streaming_target(
                    authenticated_subjects,
                    access_method,
                    room_id,
                    streaming_target_id,
                ),
            Resource::RoomSip(room_id) => {
                self.authorize_room_sip(authenticated_subjects, access_method, room_id)
            }
            Resource::RoomEvent(room_id) => {
                self.authorize_room_event(authenticated_subjects, access_method, room_id)
            }
            Resource::RoomInvites(room_id) => {
                self.authorize_room_invites(authenticated_subjects, access_method, room_id)
            }
            Resource::RoomStart(room_id) => {
                self.authorize_room_start(authenticated_subjects, access_method, room_id)
            }
            Resource::RoomTariff(room_id) => {
                self.authorize_room_tariff(authenticated_subjects, access_method, room_id)
            }
            Resource::UserMe => self.authorize_user_me(authenticated_subjects),
            Resource::UserMeAssets => self.authorize_user_me(authenticated_subjects),
            Resource::UserMeEventFavorite(event_id) => self.authorize_user_me_event_favorite(
                authenticated_subjects,
                access_method,
                event_id,
            ),
            Resource::UserFind => self.authorize_user_find(authenticated_subjects),
            Resource::UserMeTariff => self.authorize_user_me_tariff(authenticated_subjects),
            Resource::UserMePendingInvites => {
                self.authorize_user_me_pending_invites(authenticated_subjects)
            }
            Resource::UserProfile(user_id) => {
                self.authorize_user_profile(authenticated_subjects, access_method, user_id)
            }
        }
    }

    pub fn apply_changes(&mut self, changeset: &[AuthorizationChange]) {
        for change in changeset {
            match change {
                AuthorizationChange::AddUserToGroups { user, groups } => {
                    self.add_user_to_groups(user, groups);
                }
                AuthorizationChange::RemoveUserFromGroups { user, groups } => {
                    self.remove_user_from_groups(user, groups);
                }
                AuthorizationChange::CreateEvent { event, creator } => {
                    self.create_event(event, creator);
                }
                AuthorizationChange::DeleteEvent { event } => {
                    self.delete_event(event);
                }
                AuthorizationChange::CreateRoom {
                    room,
                    creator,
                    guest_access,
                    e2e_encryption,
                } => {
                    self.create_room(room, creator, guest_access, e2e_encryption);
                }
                AuthorizationChange::DeleteRoom { room } => {
                    self.delete_room(room);
                }
                AuthorizationChange::CreateUser { user } => {
                    self.create_user(user);
                }
                AuthorizationChange::DeleteUser { user } => {
                    self.delete_user(user);
                }
                AuthorizationChange::AddUserToRooms { user, role, rooms } => {
                    self.add_user_to_rooms(user, rooms, role);
                }
                AuthorizationChange::RemoveUserFromRooms { user, rooms } => {
                    self.remove_user_from_rooms(user, rooms);
                }
                AuthorizationChange::AddUserToEvents { user, role, events } => {
                    self.add_user_to_events(user, events, role);
                }
                AuthorizationChange::RemoveUserFromEvents { user, events } => {
                    self.remove_user_from_events(user, events);
                }
                AuthorizationChange::AddInviteCodeToRoom {
                    room,
                    invite_code: code,
                    expiration,
                } => {
                    self.add_invite_code_to_room(room, code, expiration);
                }
                AuthorizationChange::RemoveInviteCodeFromRoom { room, invite_code } => {
                    self.remove_invite_code_from_room(room, invite_code);
                }
                AuthorizationChange::UpdateRoomConfiguration {
                    room,
                    guest_access,
                    e2e_encryption,
                } => {
                    self.update_room_configuration(room, guest_access, e2e_encryption);
                }
            }
        }
    }

    /// Authorize for the `/events` endpoint
    fn authorize_events(
        &self,
        authenticated_subjects: SubjectCollection,
        _method: AccessMethod,
    ) -> Admission {
        Admission::from(authenticated_subjects.contains_any_user())
    }

    /// Authorize for the `/events/instances` endpoint
    fn authorize_events_instances(
        &self,
        authenticated_subjects: SubjectCollection,
        _method: AccessMethod,
    ) -> Admission {
        Admission::from(authenticated_subjects.contains_any_user())
    }

    /// Authorize for the `/events/{event_id}` endpoint
    fn authorize_event(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
        event_id: EventId,
    ) -> Admission {
        let Some(event) = self.events.get(&event_id) else {
            return Admission::Denied;
        };
        event.authorize(authenticated_subjects, method)
    }

    /// Authorize for the `/events/{event_id}/instances` endpoint
    fn authorize_event_instances(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
        event_id: EventId,
    ) -> Admission {
        self.authorize_event(authenticated_subjects, method, event_id)
    }

    /// Authorize for the `/events/{event_id}/invites` endpoint
    fn authorize_event_invites(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
        event_id: EventId,
    ) -> Admission {
        self.authorize_event(authenticated_subjects, method, event_id)
    }

    /// Authorize for the `/events/{event_id}/invite` endpoint
    fn authorize_event_invite(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
        event_id: EventId,
    ) -> Admission {
        let Some(event) = self.events.get(&event_id) else {
            return Admission::Denied;
        };
        event.authorize_invite(authenticated_subjects, method)
    }

    /// Authorize for the `/events/{event_id}/shared_folder` endpoint
    fn authorize_event_shared_folder(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
        event_id: EventId,
    ) -> Admission {
        let Some(event) = self.events.get(&event_id) else {
            return Admission::Denied;
        };
        event.authorize_shared_folder(authenticated_subjects, method)
    }

    /// Authorize for the `/rooms` endpoint
    fn authorize_rooms(
        &self,
        authenticated_subjects: SubjectCollection,
        _method: AccessMethod,
    ) -> Admission {
        Admission::from(authenticated_subjects.contains_any_user())
    }

    /// Authorize for the `/rooms/{room}` endpoint
    fn authorize_room(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
        room_id: RoomId,
    ) -> Admission {
        let Some(room) = self.rooms.get(&room_id) else {
            return Admission::Denied;
        };

        room.authorize(authenticated_subjects, method)
    }

    /// Authorize for the `/rooms/{room_id}/event` endpoint
    fn authorize_room_event(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
        room_id: RoomId,
    ) -> Admission {
        // Write access to this endpoint is not allowed at all.
        if method.requires_write_access() {
            return Admission::Denied;
        }

        // Read access is identical to the `/rooms/{room_id}` endpoint.
        self.authorize_room(authenticated_subjects, method, room_id)
    }

    /// Authorize for the `/rooms/{room_id}/invites` endpoint
    fn authorize_room_invites(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
        room_id: RoomId,
    ) -> Admission {
        // Access is identical to the `/rooms/{room_id}` endpoint.
        self.authorize_room(authenticated_subjects, method, room_id)
    }

    /// Authorize for the `/rooms/{room_id}/start` endpoint
    fn authorize_room_start(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
        room_id: RoomId,
    ) -> Admission {
        let Some(room) = self.rooms.get(&room_id) else {
            return Admission::Denied;
        };

        room.authorize_start(authenticated_subjects, method)
    }

    /// Authorize for the `/rooms/{room_id}/tariff` endpoint
    fn authorize_room_tariff(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
        room_id: RoomId,
    ) -> Admission {
        let Some(room) = self.rooms.get(&room_id) else {
            return Admission::Denied;
        };

        room.authorize_tariff(authenticated_subjects, method)
    }

    fn authorize_room_invite_code(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
        room_id: RoomId,
        _invite_code: InviteCode,
    ) -> Admission {
        let Some(room) = self.rooms.get(&room_id) else {
            return Admission::Denied;
        };
        room.authorize_invite_code(authenticated_subjects, method)
    }

    /// Authorize for the `/rooms/{room_id}/assets` endpoint
    fn authorize_room_assets(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
        room_id: RoomId,
    ) -> Admission {
        let Some(room) = self.rooms.get(&room_id) else {
            return Admission::Denied;
        };

        room.authorize_assets(authenticated_subjects, method)
    }

    fn authorize_room_asset(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
        room_id: RoomId,
        _asset_id: AssetId,
    ) -> Admission {
        let Some(room) = self.rooms.get(&room_id) else {
            return Admission::Denied;
        };
        room.authorize_asset(authenticated_subjects, method)
    }

    /// Authorize for the `/rooms/{room_id}/streaming_targets` endpoint
    fn authorize_room_streaming_targets(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
        room_id: RoomId,
    ) -> Admission {
        let Some(room) = self.rooms.get(&room_id) else {
            return Admission::Denied;
        };

        room.authorize_streaming_targets(authenticated_subjects, method)
    }

    fn authorize_room_streaming_target(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
        room_id: RoomId,
        _streaming_target_id: StreamingTargetId,
    ) -> Admission {
        let Some(room) = self.rooms.get(&room_id) else {
            return Admission::Denied;
        };
        room.authorize_streaming_target(authenticated_subjects, method)
    }

    /// Authorize for the `/rooms/{room_id}/sip` endpoint
    fn authorize_room_sip(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
        room_id: RoomId,
    ) -> Admission {
        let Some(room) = self.rooms.get(&room_id) else {
            return Admission::Denied;
        };

        room.authorize_sip(authenticated_subjects, method)
    }

    /// Authorize for the `/users/find` endpoint
    fn authorize_user_find(&self, authenticated_subjects: SubjectCollection) -> Admission {
        Admission::from(authenticated_subjects.contains_any_user())
    }

    /// Authorize for the `/users/me` endpoint
    fn authorize_user_me(&self, authenticated_subjects: SubjectCollection) -> Admission {
        Admission::from(authenticated_subjects.contains_any_user())
    }

    fn authorize_user_me_event_favorite(
        &self,
        authenticated_subjects: SubjectCollection,
        access_method: AccessMethod,
        event_id: EventId,
    ) -> Admission {
        let Some(event) = self.events.get(&event_id) else {
            return Admission::Denied;
        };
        event.authorize_favorite(authenticated_subjects, access_method)
    }

    /// Authorize for the `/users/me/tariff` endpoint
    fn authorize_user_me_tariff(&self, authenticated_subjects: SubjectCollection) -> Admission {
        self.authorize_user_me(authenticated_subjects)
    }

    /// Authorize for the `/users/me/tariff/pending_invites` endpoint
    fn authorize_user_me_pending_invites(
        &self,
        authenticated_subjects: SubjectCollection,
    ) -> Admission {
        self.authorize_user_me(authenticated_subjects)
    }

    fn authorize_user_profile(
        &self,
        authenticated_subjects: SubjectCollection,
        _method: AccessMethod,
        _user_id: UserId,
    ) -> Admission {
        Admission::from(authenticated_subjects.contains_any_user())
    }

    fn add_user_to_groups(&mut self, user: &UserId, groups: &BTreeSet<GroupId>) {
        tracing::debug!("Adding user {user} to groups {groups:?} in authorization cache");
        for group in groups {
            self.groups.entry(*group).or_default().add_user(*user);
        }
    }

    fn remove_user_from_groups(&mut self, user: &UserId, groups: &BTreeSet<GroupId>) {
        for group in groups {
            let _ = self.groups.get_mut(group).map(|g| {
                g.remove_user(user);
            });
        }
    }

    fn remove_user_from_all_groups(&mut self, user: &UserId) {
        for group in self.groups.values_mut() {
            group.remove_user(user);
        }
    }

    fn create_event(&mut self, event: &EventId, creator: &UserId) {
        tracing::debug!("Adding event {event} with creator {creator} to authorization cache");
        let entry = self
            .events
            .entry(*event)
            .or_insert_with(|| Event::new(*creator));

        if &entry.owner != creator {
            tracing::warn!(
                "New event {event} with creator {creator} was already present with creator {}.",
                entry.owner
            );
        }
    }

    fn delete_event(&mut self, event: &EventId) {
        tracing::debug!("Removing event {event} from authorization cache");
        let _ = self.events.remove(event);
    }

    fn create_room(
        &mut self,
        room: &RoomId,
        creator: &UserId,
        guest_access: &GuestAccess,
        e2e_encryption: &bool,
    ) {
        tracing::debug!(%room, %creator, ?guest_access, e2e_encryption, "Adding room to authorization cache");
        let entry = self
            .rooms
            .entry(*room)
            .or_insert_with(|| Room::new(*creator, *guest_access, *e2e_encryption));

        if &entry.owner != creator {
            tracing::warn!(
                "New room {room} with creator {creator} was already present with creator {}.",
                entry.owner
            );
        }
    }

    fn delete_room(&mut self, room: &RoomId) {
        let _ = self.rooms.remove(room);
    }

    fn create_user(&mut self, user: &UserId) {
        let _ = self.users.insert(*user);
    }

    fn delete_user(&mut self, user: &UserId) {
        let _ = self.users.remove(user);
        self.remove_user_from_all_groups(user);
        self.remove_user_from_all_events(user);
        self.remove_user_from_all_rooms(user);
    }

    fn add_user_to_rooms(&mut self, user: &UserId, rooms: &BTreeSet<RoomId>, role: &InviteRole) {
        for room in rooms {
            if let Some(room) = self.rooms.get_mut(room) {
                room.add_invited_user(user, role);
            } else {
                tracing::warn!("Atttempted to add user to room {room} which does not exist");
            }
        }
    }

    fn remove_user_from_rooms(&mut self, user: &UserId, rooms: &BTreeSet<RoomId>) {
        for room in rooms {
            if let Some(room) = self.rooms.get_mut(room) {
                room.remove_invited_user(user);
            }
        }
    }

    fn remove_user_from_all_rooms(&mut self, user: &UserId) {
        for room in self.rooms.values_mut() {
            room.remove_invited_user(user);
        }
    }

    fn add_user_to_events(&mut self, user: &UserId, events: &BTreeSet<EventId>, role: &InviteRole) {
        for event in events {
            if let Some(event) = self.events.get_mut(event) {
                event.add_invited_user(user, role);
            } else {
                tracing::warn!("Atttempted to add user to event {event} which does not exist");
            }
        }
    }

    fn remove_user_from_events(&mut self, user: &UserId, events: &BTreeSet<EventId>) {
        for event in events {
            if let Some(event) = self.events.get_mut(event) {
                event.remove_invited_user(user);
            }
        }
    }

    fn remove_user_from_all_events(&mut self, user: &UserId) {
        for event in self.events.values_mut() {
            event.remove_invited_user(user);
        }
    }

    fn add_invite_code_to_room(
        &mut self,
        room: &RoomId,
        invite_code: &InviteCode,
        expiration: &Option<Timestamp>,
    ) {
        if let Some(room) = self.rooms.get_mut(room) {
            room.add_invite_code(invite_code, expiration);
        } else {
            tracing::warn!("Atttempted to add invite code to room {room} which does not exist");
        }
    }

    fn remove_invite_code_from_room(&mut self, room: &RoomId, invite_code: &InviteCode) {
        if let Some(room) = self.rooms.get_mut(room) {
            room.remove_invite_code(invite_code);
        }
    }

    fn update_room_configuration(
        &mut self,
        room: &RoomId,
        guest_access: &Option<GuestAccess>,
        e2e_encryption: &Option<bool>,
    ) {
        if let Some(room) = self.rooms.get_mut(room) {
            room.update_room_configuration(guest_access, e2e_encryption);
        } else {
            tracing::warn!(
                %room,
                ?guest_access,
                e2e_encryption,
                "Attempted to update configuration for a room which does not exist"
            );
        }
    }
}
