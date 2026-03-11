// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Contains jobs database queries

use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use opentalk_database::{DatabaseError, DbConnection, Result};

use crate::{
    schema::jobs,
    tables::jobs::{Job, SerialJobId},
};

#[tracing::instrument(err, skip_all)]
pub async fn get_job(conn: &mut DbConnection, id: SerialJobId) -> Result<Job> {
    jobs::table
        .filter(jobs::id.eq(id))
        .get_result(conn)
        .await
        .map_err(DatabaseError::from)
}

#[tracing::instrument(err, skip_all)]
pub async fn get_all_jobs(conn: &mut DbConnection) -> Result<Vec<Job>> {
    jobs::table.load(conn).await.map_err(DatabaseError::from)
}
