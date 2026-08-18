// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::{ExpressionMethods as _, JoinOnDsl as _, OptionalExtension as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use opentalk_database::{DatabaseError, DbConnection, Result};
use opentalk_types_common::{
    events::invites::InviteRole,
    rooms::{RoomIdOrAlias, invite_codes::InviteCode},
    users::UserId,
};

use crate::{
    queries::room_filter::FilterByRoom as _,
    schema::{event_invites, events, invites, rooms, tariffs, users},
    tables::{invites::Invite, rooms::Room, tariffs::Tariff},
};

pub async fn is_room_owner(
    conn: &mut DbConnection,
    room: &RoomIdOrAlias,
    user_id: UserId,
) -> Result<bool> {
    diesel::select(diesel::dsl::exists(
        rooms::table
            .filter_by_room(room)
            .filter(rooms::created_by.eq(user_id)),
    ))
    .get_result(conn)
    .await
    .map_err(DatabaseError::from)
}

pub async fn get_room_user_role(
    conn: &mut DbConnection,
    room: &RoomIdOrAlias,
    user_id: UserId,
) -> Result<Option<InviteRole>> {
    rooms::table
        .inner_join(events::table.on(events::room.eq(rooms::id)))
        .inner_join(event_invites::table.on(event_invites::event_id.eq(events::id)))
        .filter(event_invites::invitee.eq(user_id))
        .filter_by_room(room)
        .select(event_invites::role)
        .get_result(conn)
        .await
        .optional()
        .map_err(DatabaseError::from)
}

/// Returns the [`Room`], [`Invite`], and [`Tariff`] for a given [`RoomId`] and [`InviteCode`].
///
/// Returns `Ok(None)` if the request succeeds but the `room` or `invite_code` do not match an
/// existing entry, which is semantically clearer than a [`DatabaseError`].
///
/// [`RoomId`]: opentalk_types_common::rooms::RoomId
pub async fn get_room_invite_and_tariff(
    conn: &mut DbConnection,
    room: &RoomIdOrAlias,
    invite_code: InviteCode,
) -> Result<Option<(Room, Invite, Tariff)>> {
    invites::table
        .filter(invites::id.eq(invite_code))
        .inner_join(rooms::table.on(rooms::id.eq(invites::room)))
        .inner_join(users::table.on(users::id.eq(rooms::created_by)))
        .inner_join(tariffs::table.on(tariffs::id.eq(users::tariff_id)))
        .filter_by_room(room)
        .select((
            rooms::all_columns,
            invites::all_columns,
            tariffs::all_columns,
        ))
        .get_result(conn)
        .await
        .optional()
        .map_err(DatabaseError::from)
}
