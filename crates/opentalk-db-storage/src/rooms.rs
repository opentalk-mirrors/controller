// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Contains the room specific database structs and queries

use chrono::{DateTime, Utc};
use derive_more::{AsRef, Display, From, FromStr, Into};
use diesel::{ExpressionMethods, Identifiable, QueryDsl, Queryable, dsl::not, prelude::*};
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_diesel_newtype::DieselNewtype;
use opentalk_inventory as inventory;
use opentalk_types_common::{
    pagination::{ItemCount, Page, PageSize},
    rooms::{RoomId, RoomPassword},
    tenants::TenantId,
    users::UserId,
};
use serde::{Deserialize, Serialize};

use crate::{
    paginate::Paginate as _,
    schema::{events, rooms, users},
    tariffs::Tariff,
    users::User,
};

#[derive(
    AsRef,
    Display,
    From,
    FromStr,
    Into,
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    AsExpression,
    FromSqlRow,
    DieselNewtype,
)]
#[diesel(sql_type = diesel::sql_types::BigInt)]
pub struct SerialRoomId(i64);

/// Diesel room struct
///
/// Is used as a result in various queries. Represents a room column
#[derive(Debug, Clone, Queryable, Identifiable)]
#[diesel(table_name = rooms)]
pub struct Room {
    pub id: RoomId,
    pub id_serial: SerialRoomId,
    pub created_by: UserId,
    pub created_at: DateTime<Utc>,
    pub password: Option<RoomPassword>,
    pub waiting_room: bool,
    pub tenant_id: TenantId,
    pub e2e_encryption: bool,
}

impl From<Room> for inventory::Room {
    fn from(
        Room {
            id,
            id_serial,
            created_by,
            created_at,
            password,
            waiting_room,
            tenant_id,
            e2e_encryption,
        }: Room,
    ) -> Self {
        Self {
            id,
            id_serial: id_serial.into(),
            created_by,
            created_at: created_at.into(),
            password,
            waiting_room,
            tenant_id,
            e2e_encryption,
        }
    }
}

impl From<inventory::Room> for Room {
    fn from(
        inventory::Room {
            id,
            id_serial,
            created_by,
            created_at,
            password,
            waiting_room,
            tenant_id,
            e2e_encryption,
        }: inventory::Room,
    ) -> Self {
        Self {
            id,
            id_serial: id_serial.into(),
            created_by,
            created_at: created_at.into(),
            password,
            waiting_room,
            tenant_id,
            e2e_encryption,
        }
    }
}

impl Room {
    /// Select a room using the given id
    #[tracing::instrument(err, skip_all)]
    pub async fn get(conn: &mut DbConnection, id: RoomId) -> Result<Self> {
        let query = rooms::table.filter(rooms::id.eq(id));

        let room: Room = query.get_result(conn).await?;

        Ok(room)
    }

    /// Select a room and the creator using the given room id
    #[tracing::instrument(err, skip_all)]
    pub async fn get_with_user(conn: &mut DbConnection, id: RoomId) -> Result<(Self, User)> {
        let query = rooms::table
            .filter(rooms::id.eq(id))
            .inner_join(users::table);

        let result: (Room, User) = query.get_result(conn).await?;

        Ok(result)
    }

    /// Select all rooms joined with their creator
    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_with_creator(conn: &mut DbConnection) -> Result<Vec<(Room, User)>> {
        let query = rooms::table
            .order_by(rooms::id.desc())
            .inner_join(users::table);

        let room_with_creator = query.load::<(Room, User)>(conn).await?;

        Ok(room_with_creator)
    }

    /// Select all rooms paginated
    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_with_creator_paginated(
        conn: &mut DbConnection,
        limit: PageSize,
        page: Page,
    ) -> Result<(Vec<(Room, User)>, ItemCount)> {
        let query = rooms::table
            .inner_join(users::table)
            .select((rooms::all_columns, users::all_columns))
            .order_by(rooms::id.desc())
            .paginate_by(limit, page);

        let rooms_with_total = query.load_and_count(conn).await?;

        Ok(rooms_with_total)
    }

    /// Select all rooms filtered by ids
    #[tracing::instrument(err, skip_all)]
    pub async fn get_by_ids_with_creator_paginated(
        conn: &mut DbConnection,
        ids: &[RoomId],
        limit: PageSize,
        page: Page,
    ) -> Result<(Vec<(Room, User)>, ItemCount)> {
        let query = rooms::table
            .inner_join(users::table)
            .select((rooms::all_columns, users::all_columns))
            .filter(rooms::id.eq_any(ids))
            .order_by(rooms::id.desc())
            .paginate_by(limit, page);

        let rooms_with_total = query.load_and_count(conn).await?;

        Ok(rooms_with_total)
    }

    /// Select all rooms that have no event associated with them
    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_orphaned_ids(conn: &mut DbConnection) -> Result<Vec<RoomId>> {
        let query = rooms::table
            .select(rooms::id)
            .filter(not(rooms::id.eq_any(events::table.select(events::room))))
            .order_by(rooms::created_at.asc());

        let room_ids = query.load(conn).await?;

        Ok(room_ids)
    }

    /// Get the room's tariff
    #[tracing::instrument(err, skip_all)]
    pub async fn get_tariff(&self, conn: &mut DbConnection) -> Result<Tariff> {
        let user = User::get(conn, self.created_by).await?;
        Tariff::get(conn, user.tariff_id).await
    }

    /// Delete a room using the given id
    #[tracing::instrument(err, skip_all)]
    pub async fn delete_by_id(conn: &mut DbConnection, room_id: RoomId) -> Result<()> {
        let query = diesel::delete(rooms::table.filter(rooms::id.eq(room_id)));

        query.execute(conn).await?;

        Ok(())
    }

    /// Delete the room from the database
    pub async fn delete(self, conn: &mut DbConnection) -> Result<()> {
        Self::delete_by_id(conn, self.id).await
    }
}
/// Diesel insertable room struct
///
/// Represents fields that have to be provided on room insertion.
#[derive(Debug, Insertable)]
#[diesel(table_name = rooms)]
pub struct NewRoom {
    pub created_by: UserId,
    pub password: Option<RoomPassword>,
    pub waiting_room: bool,
    pub tenant_id: TenantId,
    pub e2e_encryption: bool,
}

impl From<inventory::NewRoom> for NewRoom {
    fn from(
        inventory::NewRoom {
            created_by,
            password,
            waiting_room,
            tenant_id,
            e2e_encryption,
        }: inventory::NewRoom,
    ) -> Self {
        Self {
            created_by,
            password,
            waiting_room,
            tenant_id,
            e2e_encryption,
        }
    }
}

impl NewRoom {
    #[tracing::instrument(err, skip_all)]
    pub async fn insert(self, conn: &mut DbConnection) -> Result<Room> {
        let room = self.insert_into(rooms::table).get_result(conn).await?;

        Ok(room)
    }
}

/// Diesel room struct for updates
///
/// Is used in update queries. None fields will be ignored on update queries
#[derive(Debug, AsChangeset)]
#[diesel(table_name = rooms)]
pub struct UpdateRoom {
    pub password: Option<Option<RoomPassword>>,
    pub waiting_room: Option<bool>,
    pub e2e_encryption: Option<bool>,
}

impl From<inventory::UpdateRoom> for UpdateRoom {
    fn from(
        inventory::UpdateRoom {
            password,
            waiting_room,
            e2e_encryption,
        }: inventory::UpdateRoom,
    ) -> Self {
        Self {
            password,
            waiting_room,
            e2e_encryption,
        }
    }
}

impl UpdateRoom {
    #[tracing::instrument(err, skip_all)]
    pub async fn apply(self, conn: &mut DbConnection, room_id: RoomId) -> Result<Room> {
        let target = rooms::table.filter(rooms::id.eq(&room_id));
        let room = diesel::update(target).set(self).get_result(conn).await?;

        Ok(room)
    }
}
