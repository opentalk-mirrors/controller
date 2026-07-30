// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::collections::{BTreeMap, BTreeSet};

use opentalk_types_common::{
    events::EventId,
    features::{FeatureId, ModuleFeatureId},
    modules::ModuleId,
    rooms::{RoomIdOrAlias, invite_codes::InviteCode},
    users::UserId,
};

use crate::{
    Result,
    authorization::types::{AuthorizationInviteCodeValidity, AuthorizationUserRole},
};

/// A trait for retrieving and storing event entities.
#[cfg_attr(feature = "mockall", mockall::automock)]
#[async_trait::async_trait]
pub trait AuthorizationInventory: Send {
    /// Returns the [`AuthorizationUserRole`] associated with the `user_id`.
    async fn get_event_user_role(
        &mut self,
        event_id: EventId,
        user_id: UserId,
    ) -> Result<AuthorizationUserRole>;

    /// Return the [`AuthorizationInviteCodeValidity`] of the invite code.
    async fn get_event_invite_code_validity(
        &mut self,
        event_id: EventId,
        invite_code: InviteCode,
        disabled_features: BTreeSet<ModuleFeatureId>,
        module_features: BTreeMap<ModuleId, BTreeSet<FeatureId>>,
    ) -> Result<AuthorizationInviteCodeValidity>;

    /// Returns the [`AuthorizationUserRole`] associated with the `user_id` on the room.
    async fn get_room_user_role(
        &mut self,
        room_id_or_alias: RoomIdOrAlias,
        user_id: UserId,
    ) -> Result<AuthorizationUserRole>;

    /// Return the [`AuthorizationInviteCodeValidity`] of the room invite code.
    async fn get_room_invite_code_validity(
        &mut self,
        room_id_or_alias: RoomIdOrAlias,
        invite_code: InviteCode,
        disabled_features: BTreeSet<ModuleFeatureId>,
        module_features: BTreeMap<ModuleId, BTreeSet<FeatureId>>,
    ) -> Result<AuthorizationInviteCodeValidity>;
}
