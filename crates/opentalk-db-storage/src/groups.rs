// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::collections::BTreeSet;

use diesel::BoolExpressionMethods;
use diesel::Queryable;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_types_common::{
    tenants::TenantId,
    users::{GroupId, GroupName, UserId},
};

use crate::{
    schema::{groups, user_groups},
    users::User,
};

pub use crate::tables::groups::{Group, NewGroup};

#[derive(Debug, Insertable)]
#[diesel(table_name = user_groups)]
pub struct NewUserGroupRelation {
    pub user_id: UserId,
    pub group_id: GroupId,
}

#[derive(Debug, Queryable, Identifiable, Associations)]
#[diesel(table_name = user_groups)]
#[diesel(belongs_to(User, foreign_key = user_id))]
#[diesel(belongs_to(Group, foreign_key = group_id))]
#[diesel(primary_key(user_id, group_id))]
pub struct UserGroupRelation {
    pub user_id: UserId,
    pub group_id: GroupId,
}

/// Get or create groups in the database by their name and tenant_id
/// If the group is currently not stored, create a new group and returns the ID along the already present ones.
/// Does not preserve the order of groups passed to the function
pub async fn get_or_create_groups_by_name(
    conn: &mut DbConnection,
    groups: &[(TenantId, GroupName)],
) -> Result<Vec<Group>> {
    let new_groups: Vec<NewGroup> = groups
        .iter()
        .map(|&(tenant_id, ref name)| NewGroup { name, tenant_id })
        .collect();
    diesel::insert_into(groups::table)
        .values(&new_groups)
        .on_conflict((groups::tenant_id, groups::name))
        .do_nothing()
        .execute(conn)
        .await?;

    let mut query = groups::table.select(groups::all_columns).into_boxed();

    for (tenant_id, group_name) in groups {
        query = query.or_filter(
            groups::tenant_id
                .eq(tenant_id)
                .and(groups::name.eq(group_name)),
        );
    }

    let groups: Vec<Group> = query.load(conn).await?;

    Ok(groups)
}

/// Add a user to a set of groups
///
/// The result will contain the set of ids for the groups to which the user was
/// effectively added. Any groups passed into the `groups` parameters where the
/// user was already a member anyway will be missing from the returned set.
#[tracing::instrument(err, skip_all)]
pub async fn insert_user_into_groups(
    conn: &mut DbConnection,
    user_id: UserId,
    groups: &[GroupId],
) -> Result<BTreeSet<GroupId>> {
    let new_user_groups = groups
        .iter()
        .map(|&group_id| NewUserGroupRelation { user_id, group_id })
        .collect::<Vec<_>>();

    let inserted_groups = diesel::insert_into(user_groups::table)
        .values(new_user_groups)
        .on_conflict_do_nothing()
        .returning(user_groups::group_id)
        .load(conn)
        .await?;

    Ok(BTreeSet::from_iter(inserted_groups))
}

/// Remove a user from all groups except those given in the `groups_to_keep` parameter.
///
/// The result will contain the set of ids for the groups from which the user
/// was effectively removed. Any groups in which the user remained will not be
/// contained in the returned value.
#[tracing::instrument(err, skip_all)]
pub async fn remove_user_from_all_groups_except(
    conn: &mut DbConnection,
    user_id: UserId,
    group_ids_to_keep: &[GroupId],
) -> Result<BTreeSet<GroupId>> {
    let removed_groups = diesel::delete(user_groups::table)
        .filter(
            user_groups::user_id
                .eq(user_id)
                .and(user_groups::group_id.ne_all(group_ids_to_keep)),
        )
        .returning(user_groups::group_id)
        .load(conn)
        .await?;

    Ok(BTreeSet::from_iter(removed_groups))
}

#[tracing::instrument(err, skip_all)]
pub async fn remove_user_from_all_groups(conn: &mut DbConnection, user_id: UserId) -> Result<()> {
    diesel::delete(user_groups::table)
        .filter(user_groups::user_id.eq(user_id))
        .execute(conn)
        .await?;

    Ok(())
}
