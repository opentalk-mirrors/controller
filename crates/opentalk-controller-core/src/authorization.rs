// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::collections::BTreeSet;

use diesel_async::AsyncConnection as _;
use opentalk_controller_api_authorization::authorization::AuthorizationChange;
use opentalk_database::{DatabaseError, Db};

pub(super) async fn load_authorization_changes(
    db: &Db,
) -> Result<Vec<AuthorizationChange>, DatabaseError> {
    let mut conn = db.get_conn().await.unwrap();

    let transaction_result: Result<Vec<AuthorizationChange>, DatabaseError> = conn
        .transaction(async |conn| {
            let mut auth_changes = Vec::new();

            let room_id_and_auth_props =
                opentalk_db_storage::queries::rooms::get_all_room_ids_with_auth_properties(conn)
                    .await?;

            auth_changes.extend(room_id_and_auth_props.into_iter().map(
                |(room, creator, guest_access)| AuthorizationChange::CreateRoom {
                    room,
                    creator,
                    guest_access,
                },
            ));

            let event_and_creator_ids =
                opentalk_db_storage::queries::events::get_all_event_and_creator_ids(conn).await?;

            auth_changes.extend(
                event_and_creator_ids
                    .into_iter()
                    .map(|(event, creator)| AuthorizationChange::CreateEvent { event, creator }),
            );

            let users_with_groups =
                opentalk_db_storage::queries::users::get_all_users_with_groups(conn).await?;

            for (user, groups) in users_with_groups {
                auth_changes.push(AuthorizationChange::CreateUser { user: user.id });
                auth_changes.push(AuthorizationChange::AddUserToGroups {
                    user: user.id,
                    groups: groups.iter().map(|g| g.id).collect(),
                });
            }

            let invites = opentalk_db_storage::queries::invites::get_all_invites(conn).await?;

            auth_changes.extend(
                invites
                    .into_iter()
                    .filter(|invite| invite.active)
                    .map(|invite| AuthorizationChange::AddInviteCodeToRoom {
                        room: invite.room,
                        invite_code: invite.id,
                    }),
            );

            let event_invitees =
                opentalk_db_storage::queries::events::get_all_events_with_invitee(conn).await?;

            for (event_id, room_id, user_id, role) in event_invitees {
                auth_changes.push(AuthorizationChange::AddUserToEvents {
                    user: user_id,
                    role,
                    events: BTreeSet::from([event_id]),
                });
                auth_changes.push(AuthorizationChange::AddUserToRooms {
                    user: user_id,
                    role,
                    rooms: BTreeSet::from([room_id]),
                });
            }

            Ok(auth_changes)
        })
        .await;

    transaction_result
}
