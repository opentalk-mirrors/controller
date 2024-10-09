// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Contains rooms database queries

use diesel::{dsl::not, prelude::*};
use diesel_async::RunQueryDsl;
use opentalk_database::{DatabaseError, DbConnection, Result};
use opentalk_types_common::{
    pagination::{ItemCount, Page, PageSize},
    rooms::RoomId,
    users::UserId,
};

use crate::{
    self as db,
    paginate::Paginate,
    schema::{event_invites, events, rooms, users},
    tables::{
        rooms::{NewRoom, Room, UpdateRoom},
        tariffs::Tariff,
        users::User,
    },
};

/// Select a room using the given id
#[tracing::instrument(err(level = "debug"), skip_all)]
pub async fn get_room(conn: &mut DbConnection, id: RoomId) -> Result<Room> {
    rooms::table
        .filter(rooms::id.eq(id))
        .get_result(conn)
        .await
        .map_err(DatabaseError::from)
}

/// Select a room and the creator using the given room id
#[tracing::instrument(err(level = "debug"), skip_all)]
pub async fn get_room_with_creator(conn: &mut DbConnection, id: RoomId) -> Result<(Room, User)> {
    rooms::table
        .filter(rooms::id.eq(id))
        .inner_join(users::table)
        .get_result(conn)
        .await
        .map_err(DatabaseError::from)
}

/// Select all rooms joined with their creator
#[tracing::instrument(err(level = "debug"), skip_all)]
pub async fn get_all_rooms_with_creator(conn: &mut DbConnection) -> Result<Vec<(Room, User)>> {
    rooms::table
        .order_by(rooms::id.desc())
        .inner_join(users::table)
        .load::<(Room, User)>(conn)
        .await
        .map_err(DatabaseError::from)
}

/// Select all rooms paginated
#[tracing::instrument(err(level = "debug"), skip_all)]
pub async fn get_all_rooms_paginated_with_creator(
    conn: &mut DbConnection,
    limit: PageSize,
    page: Page,
) -> Result<(Vec<(Room, User)>, ItemCount)> {
    rooms::table
        .inner_join(users::table)
        .select((rooms::all_columns, users::all_columns))
        .order_by(rooms::id.desc())
        .paginate_by(limit, page)
        .load_and_count(conn)
        .await
        .map_err(DatabaseError::from)
}

/// Select all rooms accessible to a certain user
#[tracing::instrument(err, skip_all)]
pub async fn get_accessible_to_user_with_creator_paginated(
    conn: &mut DbConnection,
    user: UserId,
    limit: PageSize,
    page: Page,
) -> Result<(Vec<(Room, User)>, ItemCount)> {
    let query = rooms::table
        .inner_join(users::table)
        .left_join(events::table)
        .left_join(event_invites::table.on(event_invites::event_id.eq(events::id)))
        .filter(
            rooms::created_by
                .eq(user)
                .or(event_invites::invitee.eq(user)),
        )
        .order_by(rooms::id.desc())
        .select((rooms::all_columns, users::all_columns))
        .paginate_by(limit, page);

    let rooms_with_total = query.load_and_count(conn).await?;

    Ok(rooms_with_total)
}

/// Select all rooms filtered by ids
#[tracing::instrument(err(level = "debug"), skip_all)]
pub async fn get_by_ids_with_creator_paginated(
    conn: &mut DbConnection,
    ids: &[RoomId],
    limit: PageSize,
    page: Page,
) -> Result<(Vec<(Room, User)>, ItemCount)> {
    rooms::table
        .inner_join(users::table)
        .select((rooms::all_columns, users::all_columns))
        .filter(rooms::id.eq_any(ids))
        .order_by(rooms::id.desc())
        .paginate_by(limit, page)
        .load_and_count(conn)
        .await
        .map_err(DatabaseError::from)
}

/// Select all rooms that have no event associated with them
#[tracing::instrument(err(level = "debug"), skip_all)]
pub async fn get_all_orphaned_room_ids(conn: &mut DbConnection) -> Result<Vec<RoomId>> {
    rooms::table
        .select(rooms::id)
        .filter(not(rooms::id.eq_any(events::table.select(events::room))))
        .order_by(rooms::created_at.asc())
        .load(conn)
        .await
        .map_err(DatabaseError::from)
}

/// Get the room's tariff
#[tracing::instrument(err(level = "debug"), skip_all)]
pub async fn get_tariff(conn: &mut DbConnection, room: Room) -> Result<Tariff> {
    let user = db::queries::users::get_user(conn, room.created_by).await?;

    db::queries::tariffs::get_tariff(conn, user.tariff_id).await
}

/// Select all room ids and their creator
#[tracing::instrument(err(level = "debug"), skip_all)]
pub async fn get_all_room_and_creator_ids(
    conn: &mut DbConnection,
) -> Result<Vec<(RoomId, UserId)>> {
    rooms::table
        .select((rooms::id, rooms::created_by))
        .load::<(RoomId, UserId)>(conn)
        .await
        .map_err(DatabaseError::from)
}

/// Delete a room using the given id
#[tracing::instrument(err(level = "debug"), skip_all)]
pub async fn delete_room(conn: &mut DbConnection, room_id: RoomId) -> Result<()> {
    _ = diesel::delete(rooms::table.filter(rooms::id.eq(room_id)))
        .execute(conn)
        .await?;

    Ok(())
}

/// Create new room
#[tracing::instrument(err(level = "debug"), skip_all)]
pub async fn create_room(conn: &mut DbConnection, new_room: NewRoom) -> Result<Room> {
    diesel::insert_into(rooms::table)
        .values(new_room)
        .get_result(conn)
        .await
        .map_err(DatabaseError::from)
}

/// Create room
#[tracing::instrument(err(level = "debug"), skip_all)]
pub async fn update_room(
    conn: &mut DbConnection,
    update_room: UpdateRoom,
    room_id: RoomId,
) -> Result<Room> {
    diesel::update(rooms::table.filter(rooms::id.eq(&room_id)))
        .set(update_room)
        .get_result(conn)
        .await
        .map_err(DatabaseError::from)
}
