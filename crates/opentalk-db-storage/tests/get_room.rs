// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileCopyrightText: OpenTalk Team <mail@opentalk.eu>

use opentalk_db_storage::{
    self as db,
    tables::{rooms::NewRoom, users::User},
};
use opentalk_types_common::{
    rooms::{GuestAccess, RoomAlias, RoomName, RoomSuffix},
    utils::ExampleData as _,
};
use pretty_assertions::assert_eq;

use crate::common::make_user;

mod common;

const SUFFIX_LENGTH: u8 = 8;

fn new_room(user: &User, name: Option<RoomName>, suffix: Option<RoomSuffix>) -> NewRoom {
    NewRoom {
        created_by: user.id,
        password: None,
        waiting_room: false,
        guest_access: GuestAccess::default(),
        e2e_encryption: false,
        tenant_id: user.tenant_id,
        name,
        suffix,
    }
}

#[tokio::test]
async fn get_room_by_id_returns_the_room() {
    let db_ctx = opentalk_test_util::database::DatabaseContext::new().await;
    let mut conn = db_ctx.db.get_conn().await.unwrap();

    let user = make_user(&mut conn, "Ruth", "Less", "Ruth").await;

    let name = RoomName::example_data();
    let suffix = RoomSuffix::generate(SUFFIX_LENGTH);
    let created =
        db::queries::rooms::create_room(&mut conn, new_room(&user, Some(name), Some(suffix)))
            .await
            .unwrap();

    let fetched = db::queries::rooms::get_room(&mut conn, created.id.into())
        .await
        .unwrap();

    assert_eq!(fetched.id, created.id);
}

#[tokio::test]
async fn get_room_by_alias_with_suffix_matches_name_and_suffix() {
    let db_ctx = opentalk_test_util::database::DatabaseContext::new().await;
    let mut conn = db_ctx.db.get_conn().await.unwrap();

    let user = make_user(&mut conn, "Ruth", "Less", "Ruth").await;

    let name = RoomName::example_data();
    let suffix = RoomSuffix::generate(SUFFIX_LENGTH);
    let target = db::queries::rooms::create_room(
        &mut conn,
        new_room(&user, Some(name.clone()), Some(suffix.clone())),
    )
    .await
    .unwrap();

    // Create a room with the same name but a different suffix
    db::queries::rooms::create_room(
        &mut conn,
        new_room(
            &user,
            Some(name.clone()),
            Some(RoomSuffix::generate(SUFFIX_LENGTH)),
        ),
    )
    .await
    .unwrap();

    let fetched = db::queries::rooms::get_room(
        &mut conn,
        RoomAlias {
            name,
            suffix: Some(suffix),
        }
        .into(),
    )
    .await
    .unwrap();

    assert_eq!(fetched.id, target.id);
}

#[tokio::test]
async fn get_room_by_alias_without_suffix_matches_null_suffix_only() {
    let db_ctx = opentalk_test_util::database::DatabaseContext::new().await;
    let mut conn = db_ctx.db.get_conn().await.unwrap();

    let user = make_user(&mut conn, "Ruth", "Less", "Ruth").await;

    let name = RoomName::example_data();

    // The suffix-less room (suffix IS NULL) is the one we expect to find.
    let target =
        db::queries::rooms::create_room(&mut conn, new_room(&user, Some(name.clone()), None))
            .await
            .unwrap();

    // A room with the same name but a suffix must not be matched by the `IS NULL` lookup.
    let _suffixed = db::queries::rooms::create_room(
        &mut conn,
        new_room(
            &user,
            Some(name.clone()),
            Some(RoomSuffix::generate(SUFFIX_LENGTH)),
        ),
    )
    .await
    .unwrap();

    let fetched = db::queries::rooms::get_room(&mut conn, RoomAlias { name, suffix: None }.into())
        .await
        .unwrap();

    assert_eq!(fetched.id, target.id);
    assert_eq!(fetched.suffix, None);
}

#[tokio::test]
async fn get_room_by_alias_returns_not_found_when_absent() {
    let db_ctx = opentalk_test_util::database::DatabaseContext::new().await;
    let mut conn = db_ctx.db.get_conn().await.unwrap();

    let err = db::queries::rooms::get_room(
        &mut conn,
        RoomAlias {
            name: RoomName::example_data(),
            suffix: None,
        }
        .into(),
    )
    .await
    .unwrap_err();

    assert!(err.is_not_found());
}

#[tokio::test]
async fn get_room_with_creator_by_alias_returns_room_and_creator() {
    let db_ctx = opentalk_test_util::database::DatabaseContext::new().await;
    let mut conn = db_ctx.db.get_conn().await.unwrap();

    let user = make_user(&mut conn, "Ruth", "Less", "Ruth").await;

    let name = RoomName::example_data();
    let suffix = RoomSuffix::generate(SUFFIX_LENGTH);
    let created = db::queries::rooms::create_room(
        &mut conn,
        new_room(&user, Some(name.clone()), Some(suffix.clone())),
    )
    .await
    .unwrap();

    let (room, creator) = db::queries::rooms::get_room_with_creator(
        &mut conn,
        RoomAlias {
            name,
            suffix: Some(suffix),
        }
        .into(),
    )
    .await
    .unwrap();

    assert_eq!(room.id, created.id);
    assert_eq!(creator.id, user.id);
}
