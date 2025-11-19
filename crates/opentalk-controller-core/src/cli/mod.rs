// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::path::PathBuf;

use build_info::BuildInfo;
use clap::Parser;
use command::Command;
use opentalk_signaling_core::RegisterModules;
use opentalk_version::InfoArgs;

use crate::Result;

mod acl;
mod command;
mod fix_acl;
mod jobs;
mod license;
mod migrate_db;
mod modules;
mod openapi;
mod reload;
mod tariffs;
mod tenants;

#[derive(Parser, Debug, Clone)]
#[clap(name = "opentalk-controller")]
pub struct Args {
    /// Path of the configuration file.
    ///
    /// If present, exactly this config file will be used.
    ///
    /// If absent, `controller` looks for a config file in these locations and uses the first one that is found:
    ///
    /// - `config.toml` in the current directory (deprecated, for backwards compatiblity only)
    /// - `controller.toml` in the current directory
    /// - `<XDG_CONFIG_HOME>/opentalk/controller.toml` (where `XDG_CONFIG_HOME` is usually `~/.config`)
    /// - `/etc/opentalk/controller.toml`
    #[clap(short, long, verbatim_doc_comment)]
    pub config: Option<PathBuf>,

    /// Triggers a reload of reloadable configuration options
    #[clap(long)]
    pub reload: bool,

    #[clap(subcommand)]
    cmd: Option<Command>,

    #[command(flatten)]
    pub(crate) info: InfoArgs,
}

impl Args {
    /// Returns true if we want to startup the controller after we finished the cli part
    pub fn controller_should_start(&self) -> bool {
        !(self.reload || self.cmd.is_some() || self.info.should_print())
    }
}

/// Parses the CLI-Arguments into [`Args`]
///
/// Also runs (optional) cli commands if necessary
pub async fn parse_args<M: RegisterModules>() -> Result<Args> {
    let args = Args::parse();

    if args.info.should_print() {
        print_info(&args.info);
    }

    if args.reload {
        reload::trigger_reload()?;
    }

    if let Some(command) = args.cmd.clone() {
        command.exec::<M>(args.config.as_deref()).await?;
    }

    Ok(args)
}

opentalk_version::build_info!();

fn print_info(info_args: &InfoArgs) {
    let build_info = BuildInfo::with_license(license::LICENSE.to_owned());
    if let Some(text) = build_info.format(info_args) {
        println!("{text}");
    }
}
