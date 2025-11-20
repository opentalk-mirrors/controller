// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Allows to manipulate the acls
//! Currently supported is enabling/disabling room access for all users.
use std::{path::Path, sync::Arc};

use clap::{Parser, Subcommand};
use kustos::prelude::AccessMethod;
use opentalk_controller_core::{
    acl::{check_or_create_kustos_role_policy, maybe_remove_kustos_role_policy},
    load_settings_provider,
};
use opentalk_controller_settings::Settings;
use opentalk_database::Db;
use opentalk_inventory_database::DatabaseConnectionPool;
use opentalk_kustos_inventory::KustosInventoryProvider;
use snafu::ResultExt;

use crate::Result;

#[derive(Subcommand, Debug, Clone)]
#[clap(rename_all = "kebab_case")]
pub enum Command {
    /// Allows all users access to all rooms
    UsersHaveAccessToAllRooms {
        /// Enable/Disable
        #[clap(subcommand)]
        action: EnableDisable,
    },
}

impl Command {
    pub async fn exec(self, optional_config_path: Option<&Path>) -> Result<()> {
        let settings = load_settings_provider(optional_config_path)?.get();
        match self {
            Command::UsersHaveAccessToAllRooms { action } => match action {
                EnableDisable::Enable => enable_user_access_to_all_rooms(&settings).await,
                EnableDisable::Disable => disable_user_access_to_all_rooms(&settings).await,
            },
        }
    }
}

#[derive(Parser, Debug, Clone)]
#[clap(rename_all = "kebab_case")]
pub enum EnableDisable {
    /// enable
    Enable,
    /// disable
    Disable,
}

async fn enable_user_access_to_all_rooms(settings: &Settings) -> Result<()> {
    let db = Arc::new(
        Db::connect(&settings.database).whatever_context("Failed to connect to database")?,
    );
    let inventory: Arc<dyn KustosInventoryProvider> = Arc::new(DatabaseConnectionPool::new(db));
    let authz = kustos::Authz::new(inventory)
        .await
        .whatever_context("Failed to initialize kustos/authz")?;

    check_or_create_kustos_role_policy(&authz, "user", "/rooms/*/start", AccessMethod::POST)
        .await?;
    check_or_create_kustos_role_policy(&authz, "user", "/rooms/*", AccessMethod::GET).await?;
    println!("Enabled access for all users to all rooms");
    Ok(())
}

async fn disable_user_access_to_all_rooms(settings: &Settings) -> Result<()> {
    let db = Arc::new(
        Db::connect(&settings.database).whatever_context("Failed to connect to database")?,
    );
    let inventory: Arc<dyn KustosInventoryProvider> = Arc::new(DatabaseConnectionPool::new(db));
    let authz = kustos::Authz::new(inventory)
        .await
        .whatever_context("Failed to initialize kustos/authz")?;

    maybe_remove_kustos_role_policy(&authz, "user", "/rooms/*/start", AccessMethod::POST).await?;
    maybe_remove_kustos_role_policy(&authz, "user", "/rooms/*", AccessMethod::GET).await?;
    println!("Disabled access for all users to all rooms");
    Ok(())
}
