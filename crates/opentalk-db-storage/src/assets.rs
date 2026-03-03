// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::{ExpressionMethods, NullableExpressionMethods as _, QueryDsl};
use opentalk_database::{DbConnection, Result};
use opentalk_types_common::{
    assets::AssetSorting,
    events::EventId,
    order::Ordering,
    pagination::{ItemCount, Page, PageSize},
    rooms::RoomId,
    users::UserId,
};

pub use crate::tables::assets::{Asset, NewAsset, UpdateAsset};

use crate::{
    paginate::Paginate as _,
    schema::{assets, events, room_assets, rooms},
};

pub type AssetRoomIdEventIdTuple = (Asset, RoomId, Option<EventId>);

#[tracing::instrument(err, skip_all)]
pub async fn get_all_for_room_owner_paginated_ordered(
    conn: &mut DbConnection,
    user_id: UserId,
    limit: PageSize,
    page: Page,
    sorting: AssetSorting,
    order: Ordering,
) -> Result<(Vec<AssetRoomIdEventIdTuple>, ItemCount)> {
    let mut query = room_assets::table
        .inner_join(assets::table)
        .inner_join(rooms::table.left_join(events::table))
        .filter(rooms::created_by.eq(user_id))
        .select((
            assets::all_columns,
            rooms::columns::id,
            events::columns::id.nullable(),
        ))
        .into_boxed();

    // There was no sane approach to move this block to it's own function or call asc/desc on a
    // generalized column.
    query = match (order, sorting) {
        (Ordering::Ascending, AssetSorting::Filename) => query.order(assets::filename.asc()),
        (Ordering::Ascending, AssetSorting::Size) => query.order(assets::size.asc()),
        (Ordering::Ascending, AssetSorting::Namespace) => query.order(assets::namespace.asc()),
        (Ordering::Ascending, AssetSorting::Kind) => query.order(assets::kind.asc()),
        (Ordering::Ascending, AssetSorting::CreatedAt) => query.order(assets::created_at.asc()),

        (Ordering::Descending, AssetSorting::Filename) => query.order(assets::filename.desc()),
        (Ordering::Descending, AssetSorting::Size) => query.order(assets::size.desc()),
        (Ordering::Descending, AssetSorting::Namespace) => query.order(assets::namespace.desc()),
        (Ordering::Descending, AssetSorting::Kind) => query.order(assets::kind.desc()),
        (Ordering::Descending, AssetSorting::CreatedAt) => query.order(assets::created_at.desc()),
    };

    let query = query.paginate_by(limit, page);
    Ok(query.load_and_count(conn).await?)
}
