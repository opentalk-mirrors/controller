// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Migrates the database schema

use clap::Parser;
use opentalk_controller_settings::Settings;
use snafu::ResultExt as _;

use crate::Result;

#[derive(Debug, Clone, Parser)]
pub(super) struct Command {}

impl Command {
    pub(super) async fn exec(self, settings: &Settings) -> Result<()> {
        let result = opentalk_db_storage::migrations::migrate_from_url(&settings.database.url)
            .await
            .whatever_context("Failed to migrate database")?;
        println!("{result:?}");
        Ok(())
    }
}
