// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::path::Path;

use clap::Subcommand;
use opentalk_signaling_modules::Modules;

use crate::Result;

mod acl;
mod fix_acl;
mod health;
mod jobs;
mod migrate_db;
mod modules;
mod openapi;
pub(super) mod reload;
mod tariffs;
mod tenants;

#[derive(Subcommand, Debug, Clone)]
#[clap(rename_all = "kebab_case")]
#[allow(clippy::large_enum_variant)]
pub enum Command {
    /// Recreate all ACL entries from the current database content. Existing entries will not be touched unless the
    /// command is told to delete them all beforehand.
    FixAcl(fix_acl::Command),

    /// Modify the ACLs.
    #[clap(subcommand)]
    Acl(acl::Command),

    /// Migrate the db. This is done automatically during start of the controller,
    /// but can be done without starting the controller using this command.
    MigrateDb(migrate_db::Command),

    /// Manage existing tenants
    #[clap(subcommand)]
    Tenants(tenants::Command),

    /// Manage tariffs
    #[clap(subcommand)]
    Tariffs(tariffs::Command),

    /// Manage and execute maintenance jobs
    #[clap(subcommand)]
    Jobs(jobs::Command),

    /// Manage modules
    #[clap(subcommand)]
    Modules(modules::Command),

    /// Get information on the OpenAPI specification
    #[clap(subcommand)]
    Openapi(openapi::Command),

    /// Return the readiness state
    Health(health::Command),

    /// Triggers a reload of reloadable configuration options for already
    /// running opentalk-controller processes
    Reload(reload::Command),
}

impl Command {
    pub async fn exec(self, optional_config_path: Option<&Path>) -> Result<()> {
        match self {
            Command::FixAcl(command) => {
                command.exec(optional_config_path).await?;
            }
            Command::Acl(command) => {
                command.exec(optional_config_path).await?;
            }
            Command::MigrateDb(command) => {
                command.exec(optional_config_path).await?;
            }
            Command::Tenants(command) => {
                command.exec(optional_config_path).await?;
            }
            Command::Tariffs(command) => {
                command.exec(optional_config_path).await?;
            }
            Command::Jobs(command) => {
                command.exec(optional_config_path).await?;
            }
            Command::Modules(command) => {
                command.exec::<Modules>()?;
            }
            Command::Openapi(command) => {
                command.exec()?;
            }
            Command::Health(command) => {
                command.exec(optional_config_path).await?;
            }
            Command::Reload(command) => {
                command.exec()?;
            }
        }
        Ok(())
    }
}
