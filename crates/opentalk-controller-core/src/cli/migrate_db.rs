// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Migrates the database schema

use std::path::Path;

use clap::Parser;
use snafu::ResultExt as _;

use crate::{Result, load_settings_provider};

#[derive(Debug, Clone, Parser)]
pub(super) struct Command {}

impl Command {
    pub(super) async fn exec(self, optional_config_path: Option<&Path>) -> Result<()> {
        let settings = load_settings_provider(optional_config_path)?.get();
        let result = opentalk_db_storage::migrations::migrate_from_url(&settings.database.url)
            .await
            .whatever_context("Failed to migrate database")?;
        println!("{result:?}");
        Ok(())
    }
}
