// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::str::FromStr;

use chrono::{DateTime, Utc};
use diesel::{
    ExpressionMethods, Identifiable, QueryDsl, Queryable, pg::Pg, prelude::*, sql_types::Jsonb,
};
use diesel_async::RunQueryDsl;
use opentalk_database::{DatabaseError, DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::{
    module_resources::ModuleResourceId, rooms::RoomId, tenants::TenantId, users::UserId,
};
use serde::Serialize;
use snafu::Snafu;

use crate::schema::{module_resources, rooms};

#[derive(
    Debug, PartialEq, Serialize, strum::Display, strum::FromRepr, strum::AsRefStr, strum::EnumString,
)]
#[serde(rename_all = "snake_case")]
pub enum JsonPatchErrorCode {
    /// The given path is invalid
    #[strum(serialize = "ot_invalid_path", to_string = "invalid_path")]
    InvalidPath,

    /// Can only be thrown by the
    /// [`Test`](inventory::ModuleResourceOperation::Test) when the
    /// compare returns `false`
    #[strum(serialize = "ot_value_not_equal", to_string = "value_not_equal")]
    ValueNotEqual,

    /// Can only be thrown by a
    /// [`Copy`](inventory::ModuleResourceOperation::Copy) or
    /// [`Move`](inventory::ModuleResourceOperation::Move) operation
    /// when the `from` parameter is invalid.
    #[strum(serialize = "ot_invalid_from_path", to_string = "invalid_from_path")]
    InvalidFromPath,
}

#[derive(Debug, Serialize, Snafu)]
#[snafu(display("Json operation failed at index {at_index}: {error_code} - {details} "))]
pub struct JsonPatchError {
    pub error_code: JsonPatchErrorCode,
    /// Human readable error details
    pub details: String,
    /// The index of the of the operation that failed
    pub at_index: usize,
}

impl JsonPatchError {
    fn try_from_diesel_error(diesel_error: &diesel::result::Error) -> Option<Self> {
        if let diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::Unknown,
            error_info,
        ) = diesel_error
        {
            let error_code = JsonPatchErrorCode::from_str(error_info.message()).ok()?;

            let details = error_info.details()?;

            let at_index = error_info.hint()?.parse().ok()?;

            return Some(JsonPatchError {
                error_code,
                details: details.into(),
                at_index,
            });
        }

        None
    }
}

#[derive(Debug, Snafu)]
pub enum JsonOperationError {
    #[snafu(transparent)]
    JsonPatch { source: JsonPatchError },
    #[snafu(transparent)]
    Database { source: DatabaseError },
}

impl From<diesel::result::Error> for JsonOperationError {
    fn from(value: diesel::result::Error) -> Self {
        match JsonPatchError::try_from_diesel_error(&value) {
            Some(json_patch_error) => Self::JsonPatch {
                source: json_patch_error,
            },
            None => Self::Database {
                source: value.into(),
            },
        }
    }
}

/// Helper struct for filtering module resources
#[derive(Default)]
pub struct Filter {
    /// Filter by the UUID of the module resource
    id: Option<ModuleResourceId>,
    /// Filter by the room id of the module resource
    room_id: Option<RoomId>,
    /// Filter by the namespace of the module resource
    namespace: Option<String>,
    /// Filter by the creator of the module resource
    created_by: Option<UserId>,
    /// Filter by the tag of the module resource
    tag: Option<String>,
    /// Filter by the content of the module resource
    json: Option<serde_json::Value>,
}

impl From<inventory::ModuleResourceFilter> for Filter {
    fn from(value: inventory::ModuleResourceFilter) -> Self {
        let (id, room_id, namespace, created_by, tag, json) = value.into();

        let mut filter = Self::default();
        if let Some(id) = id {
            filter = filter.with_id(id);
        }
        if let Some(room_id) = room_id {
            filter = filter.with_room_id(room_id);
        }
        if let Some(namespace) = namespace {
            filter = filter.with_namespace(namespace);
        }
        if let Some(created_by) = created_by {
            filter = filter.with_created_by(created_by);
        }
        if let Some(tag) = tag {
            filter = filter.with_tag(tag);
        }
        if let Some(json) = json {
            filter = filter.with_json(json);
        }
        filter
    }
}

type ResourceFilter =
    Box<dyn BoxableExpression<module_resources::table, Pg, SqlType = diesel::sql_types::Bool>>;

impl Filter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_id(mut self, id: ModuleResourceId) -> Self {
        self.id = Some(id);

        self
    }

    pub fn with_room_id(mut self, room_id: RoomId) -> Self {
        self.room_id = Some(room_id);

        self
    }

    pub fn with_namespace(mut self, namespace: String) -> Self {
        self.namespace = Some(namespace);

        self
    }

    pub fn with_created_by(mut self, created_by: UserId) -> Self {
        self.created_by = Some(created_by);

        self
    }

    pub fn with_tag(mut self, tag: String) -> Self {
        self.tag = Some(tag);

        self
    }

    pub fn with_json(mut self, json: serde_json::Value) -> Self {
        self.json = Some(json);

        self
    }

    fn into_diesel_filter(self) -> Option<ResourceFilter> {
        fn append(query: &mut Option<ResourceFilter>, filter: ResourceFilter) {
            if let Some(query_) = query.take() {
                *query = Some(Box::new(query_.and(filter)));
            } else {
                *query = Some(filter);
            }
        }

        let mut query: Option<ResourceFilter> = None;

        if let Some(id) = self.id {
            append(&mut query, Box::new(module_resources::id.eq(id)));
        }

        if let Some(room_id) = self.room_id {
            append(&mut query, Box::new(module_resources::room_id.eq(room_id)));
        }

        if let Some(namespace) = self.namespace {
            append(
                &mut query,
                Box::new(module_resources::namespace.eq(namespace)),
            );
        }

        if let Some(created_by) = self.created_by {
            append(
                &mut query,
                Box::new(module_resources::created_by.eq(created_by)),
            );
        }

        if let Some(tag) = self.tag {
            // Unwrapping the Nullable<Bool> here with an if is_not_null then compare, else false
            append(
                &mut query,
                Box::new(
                    module_resources::tag
                        .is_not_null()
                        .and(module_resources::tag.eq(tag))
                        .assume_not_null(),
                ),
            );
        }

        if let Some(json) = self.json {
            append(&mut query, Box::new(module_resources::data.contains(json)));
        }

        query
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Queryable, Identifiable, Insertable)]
pub struct ModuleResource {
    pub id: ModuleResourceId,
    pub tenant_id: TenantId,
    pub room_id: RoomId,
    pub created_by: UserId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub namespace: String,
    pub tag: Option<String>,
    pub data: serde_json::Value,
}

impl From<ModuleResource> for inventory::ModuleResource {
    fn from(
        ModuleResource {
            id,
            tenant_id,
            room_id,
            created_by,
            created_at,
            updated_at,
            namespace,
            tag,
            data,
        }: ModuleResource,
    ) -> Self {
        Self {
            id,
            tenant_id,
            room_id,
            created_by,
            created_at: created_at.into(),
            updated_at: updated_at.into(),
            namespace,
            tag,
            data,
        }
    }
}

impl From<inventory::ModuleResource> for ModuleResource {
    fn from(
        inventory::ModuleResource {
            id,
            tenant_id,
            room_id,
            created_by,
            created_at,
            updated_at,
            namespace,
            tag,
            data,
        }: inventory::ModuleResource,
    ) -> Self {
        Self {
            id,
            tenant_id,
            room_id,
            created_by,
            created_at: created_at.into(),
            updated_at: updated_at.into(),
            namespace,
            tag,
            data,
        }
    }
}

impl ModuleResource {
    #[tracing::instrument(err, skip_all)]
    pub async fn get(conn: &mut DbConnection, filter: Filter) -> Result<Vec<Self>> {
        let filter = filter
            .into_diesel_filter()
            .ok_or_else(|| DatabaseError::Custom {
                message: "Missing filter for get module_resource query".to_owned(),
            })?;

        let query = module_resources::table.filter(filter);

        let module_resources = query.get_results(conn).await?;

        Ok(module_resources)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_ids_for_room(
        conn: &mut DbConnection,
        room_id: RoomId,
    ) -> Result<Vec<ModuleResourceId>> {
        let query = module_resources::table
            .select(module_resources::id)
            .filter(module_resources::room_id.eq(room_id));

        let module_resource_ids = query.load(conn).await?;

        Ok(module_resource_ids)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_with_creator_and_owner(
        conn: &mut DbConnection,
    ) -> Result<Vec<(ModuleResourceId, UserId, UserId)>> {
        let query = module_resources::table.inner_join(rooms::table).select((
            module_resources::id,
            module_resources::created_by,
            rooms::created_by,
        ));

        let module_resources_with_creator = query.load(conn).await?;

        Ok(module_resources_with_creator)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn delete(conn: &mut DbConnection, filter: Filter) -> Result<Vec<ModuleResource>> {
        let filter = filter
            .into_diesel_filter()
            .ok_or_else(|| DatabaseError::Custom {
                message: "Missing filter for delete query".to_owned(),
            })?;

        let query = diesel::delete(module_resources::table).filter(filter);

        Ok(query.get_results(conn).await?)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn delete_by_room(conn: &mut DbConnection, room_id: RoomId) -> Result<()> {
        diesel::delete(module_resources::table)
            .filter(module_resources::room_id.eq(room_id))
            .execute(conn)
            .await?;

        Ok(())
    }

    /// Update the targeted json data by a set of json patch
    /// [ModuleResourceOperations](inventory::ModuleResourceOperation).
    #[tracing::instrument(err, skip_all)]
    pub async fn patch(
        conn: &mut DbConnection,
        filter: Filter,
        operations: Vec<inventory::ModuleResourceOperation>,
    ) -> Result<Vec<ModuleResource>, JsonOperationError> {
        let filter = filter
            .into_diesel_filter()
            .ok_or_else(|| JsonOperationError::Database {
                source: DatabaseError::Custom {
                    message: "Missing filter for patch query".to_owned(),
                },
            })?;

        let query = diesel::update(module_resources::table).filter(filter);

        let query = query.set(module_resources::data.eq(ot_patch_json(
            module_resources::data,
            serde_json::to_value(operations).unwrap(),
        )));

        let module_resources = query.get_results(conn).await?;

        Ok(module_resources)
    }
}

define_sql_function! {
    fn ot_patch_json(target: Jsonb, changeset: Jsonb) -> Jsonb;
}

#[derive(Debug, Insertable)]
#[diesel(table_name = module_resources)]
pub struct NewModuleResource {
    pub tenant_id: TenantId,
    pub room_id: RoomId,
    pub created_by: UserId,
    pub namespace: String,
    pub tag: Option<String>,
    pub data: serde_json::Value,
}

impl From<inventory::NewModuleResource> for NewModuleResource {
    fn from(
        inventory::NewModuleResource {
            tenant_id,
            room_id,
            created_by,
            namespace,
            tag,
            data,
        }: inventory::NewModuleResource,
    ) -> Self {
        Self {
            tenant_id,
            room_id,
            created_by,
            namespace,
            tag,
            data,
        }
    }
}

impl NewModuleResource {
    #[tracing::instrument(err, skip_all)]
    pub async fn insert(self, conn: &mut DbConnection) -> Result<ModuleResource> {
        let module: ModuleResource = diesel::insert_into(module_resources::table)
            .values(self)
            .get_result(conn)
            .await?;

        Ok(module)
    }
}

#[cfg(test)]
mod tests {
    use opentalk_inventory::ModuleResourceOperation;
    use opentalk_test_util::{assert_eq, *};
    use serde_json::{Value, json};
    use serial_test::serial;

    use super::*;
    use crate as db;

    async fn init_resource(json: Value) -> (ModuleResourceId, DbConnection) {
        let db_ctx = opentalk_test_util::database::DatabaseContext::new(false).await;

        let mut db_conn = db_ctx.db.get_conn().await.unwrap();

        let user = db_ctx.create_test_user(0, vec![]).await.unwrap();

        let room = db_ctx
            .create_test_room(RoomId::from_u128(0), user.id, false)
            .await
            .unwrap();

        let tenant = db::queries::tenants::get_or_create_tenant_by_oidc_id(
            &mut db_conn,
            &db::tables::tenants::OidcTenantId::from("OpenTalkDefaultTenant"),
        )
        .await
        .unwrap();

        let m = NewModuleResource {
            tenant_id: tenant.id,
            created_by: user.id,
            room_id: room.id,
            namespace: "test".into(),
            tag: None,
            data: json,
        };

        let resource = m.insert(&mut db_conn).await.unwrap();

        (resource.id, db_conn)
    }

    async fn init_empty_resource() -> (ModuleResourceId, DbConnection) {
        init_resource(json!({})).await
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_add_single() {
        let (_id, mut db_conn) = init_empty_resource().await;

        let operations = vec![ModuleResourceOperation::Add {
            path: "/foo".into(),
            value: Value::String("bar".into()),
        }];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();
        assert_eq!(updated.len(), 1);

        assert_eq!(
            json!(updated.remove(0).data),
            json!({
                "foo": "bar"
            })
        );
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_add_to_non_empty() {
        let (_id, mut db_conn) = init_resource(json!({"foo": "bar"})).await;

        let operations = vec![ModuleResourceOperation::Add {
            path: "/baz".into(),
            value: Value::String("quux".into()),
        }];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();
        assert_eq!(updated.len(), 1);

        assert_eq!(
            json!(updated.remove(0).data),
            json!({
                "foo": "bar",
                "baz": "quux"
            })
        );
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_empty_path() {
        let (_id, mut db_conn) = init_empty_resource().await;

        let operations = vec![ModuleResourceOperation::Add {
            path: "".into(),
            value: Value::String("bar".into()),
        }];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();

        assert_eq!(json!(updated.remove(0).data), json!("bar"));
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_root_path() {
        let (_id, mut db_conn) = init_empty_resource().await;

        let operations = vec![ModuleResourceOperation::Add {
            path: "/".into(),
            value: Value::String("bar".into()),
        }];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();

        assert_eq!(json!(updated.remove(0).data), json!({"": "bar"}));
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_add_invalid() {
        let (_id, mut db_conn) = init_empty_resource().await;

        let operations = vec![ModuleResourceOperation::Add {
            path: "/foo/bar/baz".into(),
            value: Value::String("bar".into()),
        }];

        match ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        {
            Err(JsonOperationError::JsonPatch { source }) => {
                assert_eq!(JsonPatchErrorCode::InvalidPath, source.error_code)
            }
            unexpected => panic!("Expected invalid_path error, got {unexpected:?}"),
        }
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_add_a_lot() {
        let (_id, mut db_conn) = init_empty_resource().await;

        let mut json_compare = json!({});

        let mut operations = vec![];

        for i in 0..100 {
            let field = format!("foo{i}");

            operations.push(ModuleResourceOperation::Add {
                path: format!("/{field}"),
                value: Value::String("bar".into()),
            });

            if let Value::Object(hm) = &mut json_compare {
                hm.insert(field, "bar".into());
            }
        }

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();

        assert_eq!(updated.len(), 1);

        assert_eq!(json!(updated.remove(0).data), json_compare);
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_add_single_empty() {
        let (_id, mut db_conn) = init_empty_resource().await;

        let operations = vec![ModuleResourceOperation::Add {
            path: "".into(),
            value: Value::String("bar".into()),
        }];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();
        assert_eq!(updated.len(), 1);

        assert_eq!(json!(updated.remove(0).data), json!("bar"));
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_add_single_root() {
        let (_id, mut db_conn) = init_empty_resource().await;

        let operations = vec![ModuleResourceOperation::Add {
            path: "/".into(),
            value: Value::String("bar".into()),
        }];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();
        assert_eq!(updated.len(), 1);

        assert_eq!(json!(updated.remove(0).data), json!({"": "bar"}));
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_add_nested() {
        let (_id, mut db_conn) = init_empty_resource().await;

        let operations = vec![
            ModuleResourceOperation::Add {
                path: "/foo".into(),
                value: json!({"bar": 1}),
            },
            ModuleResourceOperation::Add {
                path: "/foo/bar".into(),
                value: Value::Number(42.into()),
            },
        ];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();
        assert_eq!(updated.len(), 1);

        assert_eq!(
            json!(updated.remove(0).data),
            json!({
                "foo": {
                    "bar": 42
                }
            })
        );
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_add_to_array() {
        let initial_json = json!({
            "foo": ["a", "c"],
        });

        let (_id, mut db_conn) = init_resource(initial_json).await;

        let operations = vec![ModuleResourceOperation::Add {
            path: "/foo/1".into(),
            value: Value::String("b".into()),
        }];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();
        assert_eq!(updated.len(), 1);

        assert_eq!(
            json!(updated.remove(0).data),
            json!({
                "foo": ["a", "b", "c"]
            })
        );
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_add_array() {
        let (_id, mut db_conn) = init_empty_resource().await;

        let operations = vec![ModuleResourceOperation::Add {
            path: "/foo".into(),
            value: Value::Array(vec![1.into(), 2.into(), 3.into()]),
        }];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();
        assert_eq!(updated.len(), 1);

        assert_eq!(
            json!(updated.remove(0).data),
            json!({
                "foo": [1, 2 ,3]
            })
        );
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_add_multi() {
        let (_id, mut db_conn) = init_empty_resource().await;

        let operations = vec![
            ModuleResourceOperation::Add {
                path: "/foo".into(),
                value: Value::Number(1.into()),
            },
            ModuleResourceOperation::Add {
                path: "/bar".into(),
                value: Value::Number(2.into()),
            },
        ];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();
        assert_eq!(updated.len(), 1);

        assert_eq!(
            updated.remove(0).data,
            json!({
                "foo": 1,
                "bar": 2,
            })
        );
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_update_multi_add_with_update() {
        let (_id, mut db_conn) = init_empty_resource().await;

        let operations = vec![
            ModuleResourceOperation::Add {
                path: "/foo".into(),
                value: Value::Number(1.into()),
            },
            ModuleResourceOperation::Add {
                path: "/bar".into(),
                value: Value::Number(2.into()),
            },
            ModuleResourceOperation::Add {
                path: "/foo".into(),
                value: Value::Number(42.into()),
            },
        ];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();
        assert_eq!(updated.len(), 1);

        assert_eq!(
            json!(updated.remove(0).data),
            json!({
                "foo": 42,
                "bar": 2,
            })
        );
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_simple_remove() {
        let initial_json = json!({
            "foo": 1,
            "bar": 2
        });

        let (_id, mut db_conn) = init_resource(initial_json).await;

        let operations = vec![ModuleResourceOperation::Remove {
            path: "/foo".into(),
        }];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();
        assert_eq!(updated.len(), 1);

        assert_eq!(
            json!(updated.remove(0).data),
            json!({
                "bar": 2
            })
        );
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_remove_document() {
        let initial_json = json!({
            "foo": 1,
            "bar": 2
        });

        let (_id, mut db_conn) = init_resource(initial_json).await;

        let operations = vec![ModuleResourceOperation::Remove { path: "".into() }];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();
        assert_eq!(updated.len(), 1);

        assert_eq!(json!(updated.remove(0).data), json!({}));
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_remove_by_index() {
        let initial_json = json!({
            "foo": ["a", "b", "c"],
            "bar": {
                "0": "baz",
                "qux": "biz"
            },
        });

        let (_id, mut db_conn) = init_resource(initial_json).await;

        let operations = vec![
            ModuleResourceOperation::Remove {
                path: "/foo/0".into(),
            },
            ModuleResourceOperation::Remove {
                path: "/bar/0".into(),
            },
        ];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();
        assert_eq!(updated.len(), 1);

        assert_eq!(
            json!(updated.remove(0).data),
            json!({
                "foo": ["b", "c"],
                "bar": {
                    "qux": "biz"
                },
            })
        );
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_remove_and_add_with_numbers() {
        let initial_json = json!({
            "foo": ["a", "b", "c"],
            "bar": {
                "0": "baz",
                "qux": "biz"
            },
        });

        let (_id, mut db_conn) = init_resource(initial_json).await;

        let operations = vec![
            ModuleResourceOperation::Remove {
                path: "/foo/0".into(),
            },
            ModuleResourceOperation::Remove {
                path: "/bar/0".into(),
            },
            ModuleResourceOperation::Add {
                path: "/bar/0".into(),
                value: Value::Number(42.into()),
            },
        ];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();
        assert_eq!(updated.len(), 1);

        assert_eq!(
            json!(updated.remove(0).data),
            json!({
                "foo": ["b", "c"],
                "bar": {
                    "0": 42,
                    "qux": "biz"
                },
            })
        );
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_simple_move() {
        let initial_json = json!({
            "foo": { "qux": 1 },
            "bar": {
                "baz": 2,
            },
        });

        let (_id, mut db_conn) = init_resource(initial_json).await;

        let operations = vec![ModuleResourceOperation::Move {
            from: "/foo/qux".into(),
            path: "/bar/qux".into(),
        }];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();
        assert_eq!(updated.len(), 1);

        assert_eq!(
            json!(updated.remove(0).data),
            json!({
                "foo": { },
                "bar": {
                    "baz": 2,
                    "qux": 1
                },
            })
        );
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_move_empty() {
        let initial_json = json!({
            "foo": { "qux": 1 },
            "bar": {
                "baz": 2,
            },
        });

        let (_id, mut db_conn) = init_resource(initial_json).await;

        let operations = vec![ModuleResourceOperation::Move {
            from: "".into(),
            path: "/bar/qux".into(),
        }];

        match ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        {
            Err(JsonOperationError::JsonPatch { source }) => {
                assert_eq!(JsonPatchErrorCode::InvalidFromPath, source.error_code)
            }
            unexpected => panic!("Expected invalid_from_path error, got {unexpected:?}"),
        }
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_move_root() {
        let initial_json = json!({
            "foo": { "qux": 1 },
            "bar": {
                "baz": 2,
            },
        });

        let (_id, mut db_conn) = init_resource(initial_json).await;

        let operations = vec![ModuleResourceOperation::Move {
            from: "/".into(),
            path: "/bar/qux".into(),
        }];

        match ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        {
            Err(JsonOperationError::JsonPatch { source }) => {
                assert_eq!(JsonPatchErrorCode::InvalidFromPath, source.error_code)
            }
            unexpected => panic!("Expected invalid_from_path error, got {unexpected:?}"),
        }
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_move_null_value() {
        let initial_json = json!({
            "foo": null,
            "bar": {
                "baz": 2,
            },
        });

        let (_id, mut db_conn) = init_resource(initial_json).await;

        let operations = vec![ModuleResourceOperation::Move {
            from: "/foo".into(),
            path: "/bar/qux".into(),
        }];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();

        assert_eq!(
            json!(updated.remove(0).data),
            json!({
                "bar": {
                    "baz": 2,
                    "qux": null,
                },
            })
        );
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_move_into_child() {
        let initial_json = json!({
            "foo": {
                "bar": 42
            },
        });

        let (_id, mut db_conn) = init_resource(initial_json).await;

        let from_path: String = "/foo".into();
        let to_path: String = "/foo/bar".into();
        let operations = vec![ModuleResourceOperation::Move {
            from: from_path,
            path: to_path,
        }];

        match ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        {
            Err(JsonOperationError::JsonPatch { source }) => {
                assert_eq!(JsonPatchErrorCode::InvalidFromPath, source.error_code)
            }
            unexpected => panic!("Expected invalid_source_path error, got {unexpected:?}"),
        }
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_move_value_inside_array() {
        let initial_json = json!({ "foo": [ "all", "grass", "cows", "eat" ] });

        let (_id, mut db_conn) = init_resource(initial_json).await;

        let operations = vec![ModuleResourceOperation::Move {
            from: "/foo/1".into(),
            path: "/foo/3".into(),
        }];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();
        assert_eq!(updated.len(), 1);

        assert_eq!(
            json!(updated.remove(0).data),
            json!({ "foo": [ "all", "cows", "eat", "grass" ] })
        );
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_move_array_value() {
        let initial_json = json!({
            "foo": ["a", "b", "c"],
            "bar": {
                "baz": ["d"],
                "qux": 1,
            },
        });

        let (_id, mut db_conn) = init_resource(initial_json).await;

        let operations = vec![ModuleResourceOperation::Move {
            from: "/foo/0".into(),
            path: "/bar/baz/1".into(),
        }];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();
        assert_eq!(updated.len(), 1);

        assert_eq!(
            json!(updated.remove(0).data),
            json!({
                "foo": ["b", "c"],
                "bar": {
                    "baz": ["d", "a"],
                    "qux": 1,
                },
            })
        );
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_simple_copy() {
        let initial_json = json!({
            "foo": { "qux": 1 },
            "bar": {
                "baz": 2,
            },
        });

        let (_id, mut db_conn) = init_resource(initial_json).await;

        let operations = vec![ModuleResourceOperation::Copy {
            from: "/foo/qux".into(),
            path: "/bar/qux".into(),
        }];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();
        assert_eq!(updated.len(), 1);

        assert_eq!(
            json!(updated.remove(0).data),
            json!({
                "foo": { "qux": 1 },
                "bar": {
                    "baz": 2,
                    "qux": 1,
                },
            })
        );
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_simple_test() {
        let initial_json = json!({
            "foo": 1,
            "bar": 2,
        });

        let (_id, mut db_conn) = init_resource(initial_json).await;

        let operations = vec![ModuleResourceOperation::Test {
            path: "/foo".into(),
            value: Value::Number(1.into()),
        }];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();
        assert_eq!(updated.len(), 1);

        assert_eq!(
            json!(updated.remove(0).data),
            json!({
                "foo": 1,
                "bar": 2,
            })
        );
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_simple_test_error() {
        let initial_json = json!({
            "foo": 1,
            "bar": 2,
        });

        let (_id, mut db_conn) = init_resource(initial_json).await;

        let operations = vec![ModuleResourceOperation::Test {
            path: "/foo".into(),
            value: Value::Number(99.into()),
        }];

        match ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        {
            Err(JsonOperationError::JsonPatch { source }) => {
                assert_eq!(JsonPatchErrorCode::ValueNotEqual, source.error_code)
            }
            unexpected => panic!("Expected failed compare error, got {unexpected:?}"),
        }
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_value_is_kept_on_error() {
        let initial_json = json!({
            "foo": 1,
            "bar": 2,
        });

        let (id, mut db_conn) = init_resource(initial_json.clone()).await;

        let operations = vec![
            ModuleResourceOperation::Add {
                path: "/baz".into(),
                value: Value::Number(3.into()),
            },
            ModuleResourceOperation::Remove {
                path: "/bar".into(),
            },
            ModuleResourceOperation::Test {
                path: "/foo".into(),
                value: Value::Number(99.into()),
            },
        ];

        match ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        {
            Err(JsonOperationError::JsonPatch { source }) => {
                assert_eq!(JsonPatchErrorCode::ValueNotEqual, source.error_code)
            }
            unexpected => panic!("Expected failed compare error, got {unexpected:?}"),
        }

        let resource = ModuleResource::get(&mut db_conn, Filter::new().with_id(id))
            .await
            .unwrap()
            .remove(0);

        // resource should not be changed
        assert_eq!(initial_json, resource.data);
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_object_value() {
        let initial_json = json!({
        "foo": {
            "baz": 1,
            "bar": 2,
        }});

        let (_id, mut db_conn) = init_resource(initial_json).await;

        let operations = vec![ModuleResourceOperation::Test {
            path: "/foo".into(),
            value: json!({
                "baz": 1,
                "bar": 2,
            }),
        }];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();
        assert_eq!(updated.len(), 1);

        assert_eq!(
            json!(updated.remove(0).data),
            json!({
                "foo": {
                    "baz": 1,
                    "bar": 2,
                }
            }
            )
        );
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_simple_replace() {
        let initial_json = json!({
            "foo": 1,
            "bar": 2,
        });

        let (_id, mut db_conn) = init_resource(initial_json).await;

        let operations = vec![ModuleResourceOperation::Replace {
            path: "/bar".into(),
            value: Value::Number(99.into()),
        }];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();
        assert_eq!(updated.len(), 1);

        assert_eq!(
            json!(updated.remove(0).data),
            json!({
                "foo": 1,
                "bar": 99,
            })
        );
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_replace_array_index() {
        let initial_json = json!({
            "foo": ["x", "b", "c"],
        });

        let (_id, mut db_conn) = init_resource(initial_json).await;

        let operations = vec![ModuleResourceOperation::Replace {
            path: "/foo/0".into(),
            value: Value::String("a".into()),
        }];

        let mut updated = ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        .unwrap();
        assert_eq!(updated.len(), 1);

        assert_eq!(
            json!(updated.remove(0).data),
            json!({
                "foo": ["a", "b", "c"],
            })
        );
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_replace_invalid_array_index() {
        let initial_json = json!({
            "foo": ["a", "b", "c"],
        });

        let (_id, mut db_conn) = init_resource(initial_json).await;

        let operations = vec![ModuleResourceOperation::Replace {
            path: "/foo/3".into(),
            value: Value::String("d".into()),
        }];

        match ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        {
            Err(JsonOperationError::JsonPatch { source }) => {
                assert_eq!(JsonPatchErrorCode::InvalidPath, source.error_code)
            }
            unexpected => panic!("Expected invalid path error, got {unexpected:?}"),
        }
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_replace_nonexistent_field() {
        let initial_json = json!({
            "foo": 1,
        });

        let (_id, mut db_conn) = init_resource(initial_json).await;

        let operations = vec![ModuleResourceOperation::Replace {
            path: "/bar".into(),
            value: Value::String("2".into()),
        }];

        match ModuleResource::patch(
            &mut db_conn,
            Filter::new().with_namespace("test".into()),
            operations,
        )
        .await
        {
            Err(JsonOperationError::JsonPatch { source }) => {
                assert_eq!(JsonPatchErrorCode::InvalidPath, source.error_code)
            }
            unexpected => panic!("Expected invalid path error, got {unexpected:?}"),
        }
    }

    #[actix_rt::test]
    #[serial]
    async fn serial_test_filter() {
        let db_ctx = opentalk_test_util::database::DatabaseContext::new(false).await;

        let mut db_conn = db_ctx.db.get_conn().await.unwrap();

        let user = db_ctx.create_test_user(0, vec![]).await.unwrap();

        let room = db_ctx
            .create_test_room(RoomId::from_u128(0), user.id, false)
            .await
            .unwrap();

        let tenant = db::queries::tenants::get_or_create_tenant_by_oidc_id(
            &mut db_conn,
            &db::tables::tenants::OidcTenantId::from("OpenTalkDefaultTenant"),
        )
        .await
        .unwrap();

        let new_resource = NewModuleResource {
            tenant_id: tenant.id,
            created_by: user.id,
            room_id: room.id,
            namespace: "test".into(),
            tag: Some("something".into()),
            data: json! {
                {
                    "foo": "a",
                    "bar": "b"
                }
            },
        };

        let module_resource = new_resource.insert(&mut db_conn).await.unwrap();

        let resources = ModuleResource::get(
            &mut db_conn,
            Filter::new().with_namespace("not-test".into()),
        )
        .await
        .unwrap();

        assert!(resources.is_empty());

        let resources = ModuleResource::get(&mut db_conn, Filter::new().with_tag("nothing".into()))
            .await
            .unwrap();

        assert!(resources.is_empty());

        let resources =
            ModuleResource::get(&mut db_conn, Filter::new().with_json(json! {{"baz": "c"}}))
                .await
                .unwrap();

        assert!(resources.is_empty());

        let filter = Filter::new()
            .with_id(module_resource.id)
            .with_created_by(user.id)
            .with_namespace("test".into())
            .with_tag("something".into())
            .with_json(json! {
                {
                    "foo": "a"
                }
            });

        let resources = ModuleResource::get(&mut db_conn, filter).await.unwrap();

        assert!(resources.len() == 1);
        assert_eq!(module_resource, resources[0])
    }
}
