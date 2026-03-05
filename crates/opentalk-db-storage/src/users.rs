// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Contains the user specific database structs amd queries
use std::fmt;

use bigdecimal::{BigDecimal, ToPrimitive};
use chrono::{DateTime, Utc};
use derive_more::{AsRef, Display, From, FromStr, Into};
use diesel::{
    BelongingToDsl, BoolExpressionMethods, ExpressionMethods, GroupedBy, Identifiable, Insertable,
    OptionalExtension, QueryDsl, Queryable, TextExpressionMethods, pg::Pg,
};
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_diesel_newtype::DieselNewtype;
use opentalk_inventory as inventory;
use opentalk_types_common::{
    pagination::{ItemCount, Page, PageSize},
    tariffs::{TariffId, TariffStatus},
    tenants::TenantId,
    time::TimeZone,
    users::{DisplayName, Language, Theme, UserId, UserTitle},
};
use serde::{Deserialize, Serialize};

use super::{
    schema::{assets, groups, room_assets, rooms, users},
    tables::{groups::Group, user_groups::UserGroupRelation},
};
use crate::{levenshtein, lower, newtypes::LanguageIdentifier, paginate::Paginate as _, soundex};

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
pub struct SerialUserId(i64);

const MAX_USER_SEARCH_RESULTS: usize = 50;

/// Diesel user struct
///
/// Is used as a result in various queries. Represents a user column
#[derive(Clone, Queryable, Identifiable, PartialEq, Eq)]
pub struct User {
    pub id: UserId,
    pub id_serial: SerialUserId,
    pub oidc_sub: String,
    pub email: String,
    pub title: UserTitle,
    pub firstname: String,
    pub lastname: String,
    pub language: Option<LanguageIdentifier>,
    pub display_name: DisplayName,
    pub dashboard_theme: Option<Theme>,
    pub conference_theme: Option<Theme>,
    pub phone: Option<String>,
    pub tenant_id: TenantId,
    pub tariff_id: TariffId,
    pub tariff_status: TariffStatus,
    pub disabled_since: Option<DateTime<Utc>>,
    pub avatar_url: Option<String>,
    pub timezone: Option<TimeZone>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_authenticated_at: Option<DateTime<Utc>>,
}

impl From<User> for inventory::User {
    fn from(
        User {
            id,
            id_serial,
            oidc_sub,
            email,
            title,
            firstname,
            lastname,
            language,
            display_name,
            dashboard_theme,
            conference_theme,
            phone,
            tenant_id,
            tariff_id,
            tariff_status,
            disabled_since,
            avatar_url,
            timezone,
            created_at,
            updated_at,
            last_authenticated_at: _,
        }: User,
    ) -> Self {
        Self {
            id,
            id_serial: id_serial.into(),
            oidc_sub,
            email,
            title,
            firstname,
            lastname,
            language: language.map(Into::into),
            display_name,
            dashboard_theme,
            conference_theme,
            phone,
            tenant_id,
            tariff_id,
            tariff_status,
            disabled_since: disabled_since.map(Into::into),
            avatar_url,
            timezone,
            created_at: created_at.into(),
            updated_at: updated_at.into(),
        }
    }
}

impl From<inventory::User> for User {
    fn from(
        inventory::User {
            id,
            id_serial,
            oidc_sub,
            email,
            title,
            firstname,
            lastname,
            language,
            display_name,
            dashboard_theme,
            conference_theme,
            phone,
            tenant_id,
            tariff_id,
            tariff_status,
            disabled_since,
            avatar_url,
            timezone,
            created_at,
            updated_at,
        }: inventory::User,
    ) -> Self {
        Self {
            id,
            id_serial: id_serial.into(),
            oidc_sub,
            email,
            title,
            firstname,
            lastname,
            language: language.map(Into::into),
            last_authenticated_at: None,
            display_name,
            dashboard_theme,
            conference_theme,
            phone,
            tenant_id,
            tariff_id,
            tariff_status,
            disabled_since: disabled_since.map(Into::into),
            avatar_url,
            timezone,
            created_at: created_at.into(),
            updated_at: updated_at.into(),
        }
    }
}

impl fmt::Debug for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .field("first_name", &self.firstname)
            .field("last_name", &self.lastname)
            .finish()
    }
}

impl User {
    /// Helper function to get all active users
    fn active_users_query() -> users::BoxedQuery<'static, Pg> {
        users::table
            .filter(users::disabled_since.is_null())
            .into_boxed()
    }

    /// Get an active user with the given `id`. If the user has a `disabled_since` entry, a NotFound error is returned.
    ///
    /// If no user exists with `user_id` this returns an Error
    #[tracing::instrument(err, skip_all)]
    pub async fn get(conn: &mut DbConnection, user_id: UserId) -> Result<Self> {
        let user = Self::active_users_query()
            .filter(users::id.eq(user_id))
            .get_result(conn)
            .await?;

        Ok(user)
    }

    /// Get a user with the given `id` inside a tenant
    ///
    /// If no user exists with `user_id` this returns an Error
    #[tracing::instrument(err, skip_all)]
    pub async fn get_filtered_by_tenant(
        conn: &mut DbConnection,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> Result<Self> {
        let user = Self::active_users_query()
            .filter(users::id.eq(user_id))
            .filter(users::tenant_id.eq(tenant_id))
            .get_result(conn)
            .await?;

        Ok(user)
    }

    /// Get a user with the given id
    ///
    /// Returns None if no user matches `email`
    #[tracing::instrument(err, skip_all)]
    pub async fn get_by_email(
        conn: &mut DbConnection,
        tenant_id: TenantId,
        email: &str,
    ) -> Result<Option<Self>> {
        let user = Self::active_users_query()
            .filter(users::tenant_id.eq(tenant_id))
            .filter(users::email.eq(email))
            .get_result(conn)
            .await
            .optional()?;

        Ok(user)
    }

    /// Get one or more users with the given phone number
    #[tracing::instrument(err, skip_all)]
    pub async fn get_by_phone(
        conn: &mut DbConnection,
        tenant_id: TenantId,
        phone: &str,
    ) -> Result<Vec<Self>> {
        let users = Self::active_users_query()
            .filter(users::tenant_id.eq(tenant_id))
            .filter(users::phone.eq(phone))
            .get_results(conn)
            .await?;

        Ok(users)
    }

    /// Get all users alongside their current groups
    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_with_groups(conn: &mut DbConnection) -> Result<Vec<(Self, Vec<Group>)>> {
        let users_query = Self::active_users_query().order_by(users::id.desc());
        let users = users_query.load(conn).await?;

        let groups_query = UserGroupRelation::belonging_to(&users).inner_join(groups::table);
        let groups: Vec<Vec<(UserGroupRelation, Group)>> = groups_query
            .load::<(UserGroupRelation, Group)>(conn)
            .await?
            .grouped_by(&users);

        let users_with_groups = users
            .into_iter()
            .zip(groups)
            .map(|(user, groups)| (user, groups.into_iter().map(|(_, group)| group).collect()))
            .collect();

        Ok(users_with_groups)
    }

    /// Get all users paginated
    #[tracing::instrument(err, skip_all, fields(%limit, %page))]
    pub async fn get_all_paginated(
        conn: &mut DbConnection,
        limit: PageSize,
        page: Page,
    ) -> Result<(Vec<Self>, ItemCount)> {
        let query = Self::active_users_query()
            .order_by(users::id.desc())
            .paginate_by(limit, page);

        let users_with_total = query.load_and_count(conn).await?;

        Ok(users_with_total)
    }

    /// Get all users
    #[tracing::instrument(err, skip_all)]
    pub async fn get_all(conn: &mut DbConnection) -> Result<Vec<Self>> {
        let users = users::table.get_results(conn).await?;

        Ok(users)
    }

    /// Get Users paginated and filtered by ids
    #[tracing::instrument(err, skip_all, fields(%limit, %page))]
    pub async fn get_by_ids_paginated(
        conn: &mut DbConnection,
        ids: &[UserId],
        limit: PageSize,
        page: Page,
    ) -> Result<(Vec<Self>, ItemCount)> {
        let query = Self::active_users_query()
            .filter(users::id.eq_any(ids))
            .order_by(users::id.desc())
            .paginate_by(limit, page);

        let users_with_total = query.load_and_count::<Self, _>(conn).await?;

        Ok(users_with_total)
    }

    /// Returns all `User`s filtered by id
    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_by_ids(conn: &mut DbConnection, ids: &[UserId]) -> Result<Vec<Self>> {
        let query = Self::active_users_query().filter(users::id.eq_any(ids));
        let users = query.load(conn).await?;

        Ok(users)
    }

    /// Get all users filtered by the given subs
    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_by_oidc_subs(
        conn: &mut DbConnection,
        tenant_id: TenantId,
        subs: &[&str],
    ) -> Result<Vec<Self>> {
        let users = Self::active_users_query()
            .filter(users::tenant_id.eq(tenant_id))
            .filter(users::oidc_sub.eq_any(subs))
            .load(conn)
            .await?;

        Ok(users)
    }

    /// Find users by search string
    ///
    /// This looks for similarities of the search_str in the display_name, first+lastname and email
    #[tracing::instrument(err, skip_all)]
    pub async fn find(
        conn: &mut DbConnection,
        tenant_id: TenantId,
        search_str: &str,
        max_users: usize,
    ) -> Result<Vec<Self>> {
        // IMPORTANT: lowercase it to match the index of the db and
        // remove all existing % in name and to avoid manipulation of the LIKE query.
        let search_str = search_str.replace('%', "").trim().to_lowercase();

        if search_str.is_empty() {
            return Ok(vec![]);
        }

        let like_query = format!("%{search_str}%");

        let lower_display_name = lower(users::display_name);

        let lower_first_lastname = lower(users::firstname.concat(" ").concat(users::lastname));

        let matches = Self::active_users_query()
            .filter(users::tenant_id.eq(tenant_id))
            .filter(
                // First try LIKE query on display_name
                lower_display_name.like(&like_query).or(
                    // Then try LIKE query with first+last name
                    lower_first_lastname
                        .like(&like_query)
                        // Then try LIKE query on email
                        .or(lower(users::email).like(&like_query))
                        //
                        // Then SOUNDEX on display_name
                        .or(soundex(lower_display_name)
                            .eq(soundex(&search_str))
                            // only take SOUNDEX results with a levenshtein score of lower than 5
                            .and(levenshtein(lower_display_name, &search_str).lt(5)))
                        //
                        // Then SOUNDEX on first+last name
                        .or(soundex(lower_first_lastname)
                            .eq(soundex(&search_str))
                            // only take SOUNDEX results with a levenshtein score of lower than 5
                            .and(levenshtein(lower_first_lastname, &search_str).lt(5))),
                ),
            )
            .order_by(levenshtein(lower_display_name, &search_str))
            .then_order_by(levenshtein(lower_first_lastname, &search_str))
            .then_order_by(users::id)
            .limit(MAX_USER_SEARCH_RESULTS.min(max_users) as i64)
            .load(conn)
            .await?;

        Ok(matches)
    }

    pub async fn get_used_storage(conn: &mut DbConnection, user_id: &UserId) -> Result<BigDecimal> {
        let used_storage: Option<BigDecimal> = assets::table
            .inner_join(room_assets::table.inner_join(rooms::table))
            .filter(rooms::created_by.eq(user_id))
            .select(diesel::dsl::sum(assets::size))
            .first(conn)
            .await?;

        Ok(used_storage.unwrap_or_default())
    }

    pub async fn get_used_storage_u64(conn: &mut DbConnection, user_id: &UserId) -> Result<u64> {
        let used_storage = Self::get_used_storage(conn, user_id).await?;

        Ok(used_storage.to_u64().unwrap_or_else(|| {
            log::warn!("failed to convert used storage: {used_storage} to u64");
            u64::MAX
        }))
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_disabled_before(
        conn: &mut DbConnection,
        date: DateTime<Utc>,
    ) -> Result<Vec<UserId>> {
        users::table
            .select(users::id)
            .filter(users::disabled_since.le(date))
            .load(conn)
            .await
            .map_err(Into::into)
    }

    /// Delete a user using the given id
    #[tracing::instrument(err, skip_all)]
    pub async fn delete_by_id(conn: &mut DbConnection, user_id: UserId) -> Result<()> {
        let query = diesel::delete(users::table.filter(users::id.eq(user_id)));

        query.execute(conn).await?;

        Ok(())
    }

    /// Updates the last_authenticated_ab flag using the given id
    #[tracing::instrument(err, skip_all)]
    pub async fn update_last_authenticated_at_by_id(
        conn: &mut DbConnection,
        user_id: UserId,
    ) -> Result<()> {
        let update_statement = diesel::update(users::table.filter(users::id.eq(user_id))).set((
            crate::users::users::last_authenticated_at.eq(diesel::dsl::now),
            crate::users::users::updated_at.eq(diesel::dsl::now),
        ));

        update_statement.execute(conn).await?;

        Ok(())
    }
}

/// Diesel insertable user struct
///
/// Represents fields that have to be provided on user insertion.
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub oidc_sub: String,
    pub email: String,
    pub title: UserTitle,
    pub firstname: String,
    pub lastname: String,
    pub language: Option<LanguageIdentifier>,
    pub display_name: DisplayName,
    pub phone: Option<String>,
    pub tenant_id: TenantId,
    pub tariff_id: TariffId,
    pub tariff_status: TariffStatus,
    pub avatar_url: Option<String>,
    pub timezone: Option<TimeZone>,
}

impl From<inventory::NewUser> for NewUser {
    fn from(
        inventory::NewUser {
            oidc_sub,
            email,
            title,
            firstname,
            lastname,
            language,
            display_name,
            phone,
            tenant_id,
            tariff_id,
            tariff_status,
            avatar_url,
            timezone,
        }: inventory::NewUser,
    ) -> Self {
        Self {
            oidc_sub,
            email,
            title,
            firstname,
            lastname,
            language: language.map(Into::into),
            display_name,
            phone,
            tenant_id,
            tariff_id,
            tariff_status,
            avatar_url,
            timezone,
        }
    }
}

impl NewUser {
    pub async fn insert(self, conn: &mut DbConnection) -> Result<User> {
        let query = self.insert_into(users::table);
        let user = query.get_result(conn).await?;
        Ok(user)
    }

    pub async fn insert_or_update_by_oidc_sub(
        self,
        conn: &mut DbConnection,
        enforce_display_name_on_update: bool,
    ) -> Result<User> {
        let NewUser {
            oidc_sub: _,
            email,
            title,
            firstname,
            lastname,
            language: _,
            display_name,
            phone,
            tenant_id: _,
            tariff_id,
            tariff_status,
            avatar_url,
            timezone,
        } = self.clone();
        let update_user = UpdateUser {
            title: Some(&title),
            email: Some(email),
            firstname: Some(&firstname),
            lastname: Some(&lastname),
            phone: Some(phone),
            display_name: enforce_display_name_on_update.then_some(&display_name),
            language: None,
            dashboard_theme: None,
            conference_theme: None,
            tariff_id: Some(tariff_id),
            tariff_status: Some(tariff_status),
            disabled_since: None,
            avatar_url: Some(avatar_url.as_deref()),
            timezone: Some(timezone),
            updated_at: Utc::now(),
        };
        let query = self
            .insert_into(users::table)
            .on_conflict((users::oidc_sub, users::tenant_id))
            .do_update()
            .set(update_user);
        let user = query.get_result(conn).await?;
        Ok(user)
    }
}

/// Diesel user struct for updates
///
/// Is used in update queries. None fields will be ignored on update queries
#[derive(Default, AsChangeset)]
#[diesel(table_name = users)]
pub struct UpdateUser<'a> {
    pub title: Option<&'a UserTitle>,
    pub email: Option<String>,
    pub firstname: Option<&'a str>,
    pub lastname: Option<&'a str>,
    pub phone: Option<Option<String>>,
    pub display_name: Option<&'a DisplayName>,
    pub language: Option<Option<LanguageIdentifier>>,
    pub dashboard_theme: Option<Option<Theme>>,
    pub conference_theme: Option<Option<Theme>>,
    // The tenant_id should never be updated!
    //pub tenant_id: Option<TenantId>,
    pub tariff_id: Option<TariffId>,
    pub tariff_status: Option<TariffStatus>,
    pub disabled_since: Option<Option<DateTime<Utc>>>,
    pub avatar_url: Option<Option<&'a str>>,
    pub timezone: Option<Option<TimeZone>>,
    pub updated_at: DateTime<Utc>,
}

impl<'a> From<inventory::UpdateUser<'a>> for UpdateUser<'a> {
    fn from(
        inventory::UpdateUser {
            title,
            email,
            firstname,
            lastname,
            phone,
            display_name,
            language,
            dashboard_theme,
            conference_theme,
            tariff_id,
            tariff_status,
            disabled_since,
            avatar_url,
            timezone,
            updated_at,
        }: inventory::UpdateUser<'a>,
    ) -> UpdateUser<'a> {
        UpdateUser {
            title,
            email,
            firstname,
            lastname,
            phone,
            display_name,
            language: language.map(|l| l.map(Into::into)),
            dashboard_theme: dashboard_theme.map(|t| t.copied()),
            conference_theme: conference_theme.map(|t| t.copied()),
            tariff_id,
            tariff_status,
            disabled_since: disabled_since.map(|d| d.map(Into::into)),
            avatar_url,
            timezone,
            updated_at: updated_at.into(),
        }
    }
}

impl UpdateUser<'_> {
    pub async fn apply(self, conn: &mut DbConnection, user_id: UserId) -> Result<User> {
        let query = diesel::update(users::table.filter(users::id.eq(user_id))).set(self);
        let user: User = query.get_result(conn).await?;
        Ok(user)
    }

    pub fn is_empty(&self) -> bool {
        matches!(
            self,
            Self {
                title: None,
                email: None,
                firstname: None,
                lastname: None,
                phone: None,
                display_name: None,
                language: None,
                dashboard_theme: None,
                conference_theme: None,
                tariff_id: None,
                tariff_status: None,
                disabled_since: None,
                avatar_url: None,
                timezone: None,
                updated_at: _
            }
        )
    }
}

impl User {
    pub fn into_mail_worker_registered_user(
        self,
        default_user_language: Language,
    ) -> opentalk_mail_worker_protocol::v1::RegisteredUser {
        opentalk_mail_worker_protocol::v1::RegisteredUser {
            email: self.email.into(),
            title: self.title,
            first_name: self.firstname,
            last_name: self.lastname,
            language: self
                .language
                .map(Into::into)
                .unwrap_or(default_user_language),
        }
    }
}
