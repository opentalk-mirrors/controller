// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::path::Path;

use clap::Subcommand;
use opentalk_signaling_core::RegisterModules;

use crate::Result;

#[derive(Subcommand, Debug, Clone)]
#[clap(rename_all = "kebab_case")]
#[allow(clippy::large_enum_variant)]
pub(super) enum Command {
    /// Recreate all ACL entries from the current database content. Existing entries will not be touched unless the
    /// command is told to delete them all beforehand.
    FixAcl(super::fix_acl::Command),

    /// Modify the ACLs.
    #[clap(subcommand)]
    Acl(super::acl::Command),

    /// Migrate the db. This is done automatically during start of the controller,
    /// but can be done without starting the controller using this command.
    MigrateDb(super::migrate_db::Command),

    /// Manage existing tenants
    #[clap(subcommand)]
    Tenants(super::tenants::Command),

    /// Manage tariffs
    #[clap(subcommand)]
    Tariffs(super::tariffs::Command),

    /// Manage and execute maintenance jobs
    #[clap(subcommand)]
    Jobs(super::jobs::Command),

    /// Manage modules
    #[clap(subcommand)]
    Modules(super::modules::Command),

    /// Get information on the OpenAPI specification
    #[clap(subcommand)]
    Openapi(super::openapi::Command),

    /// Triggers a reload of reloadable configuration options for already
    /// running opentalk-controller processes
    Reload(super::reload::Command),
}

impl Command {
    pub(super) async fn exec<M: RegisterModules>(
        self,
        optional_config_path: Option<&Path>,
    ) -> Result<()> {
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
                command.exec::<M>()?;
            }
            Command::Openapi(command) => {
                command.exec()?;
            }
            Command::Reload(command) => {
                command.exec()?;
            }
        }
        Ok(())
    }
}
