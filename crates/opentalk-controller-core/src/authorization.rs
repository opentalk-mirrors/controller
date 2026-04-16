// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel_async::{AsyncConnection as _, scoped_futures::ScopedFutureExt as _};
use opentalk_controller_api_authorization::authorization::AuthorizationChange;
use opentalk_database::{DatabaseError, Db};

pub(super) async fn load_authorization_changes(
    db: &Db,
) -> Result<Vec<AuthorizationChange>, DatabaseError> {
    let mut conn = db.get_conn().await.unwrap();

    let transaction_result: Result<Vec<AuthorizationChange>, DatabaseError> = conn
        .transaction(|conn| {
            async move {
                let room_and_creator_ids =
                    opentalk_db_storage::queries::rooms::get_all_room_and_creator_ids(conn).await?;

                let rooms = room_and_creator_ids
                    .into_iter()
                    .map(|(room, creator)| AuthorizationChange::CreateRoom { room, creator });

                let event_and_creator_ids =
                    opentalk_db_storage::queries::events::get_all_event_and_creator_ids(conn)
                        .await?;

                let events = event_and_creator_ids
                    .into_iter()
                    .map(|(event, creator)| AuthorizationChange::CreateEvent { event, creator });

                Ok(rooms.chain(events).collect::<Vec<_>>())
            }
            .scope_boxed()
        })
        .await;

    transaction_result
}
