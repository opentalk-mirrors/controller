// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::convert::Infallible;

use clap::Subcommand;
use itertools::Itertools as _;
use opentalk_signaling_core::{
    ModulesRegistrar, RegisterModules, SignalingModule, SignalingModuleDescription,
    SignalingModuleFeatureDescription,
};
use opentalk_types_common::modules::ModuleId;
use snafu::ResultExt as _;

use crate::Result;

#[derive(Subcommand, Debug, Clone)]
#[clap(rename_all = "kebab_case")]
pub enum Command {
    /// List available modules and their features
    List,

    /// Print a documentation of the modules (in Markdown)
    PrintDocumentation,
}

impl Command {
    pub fn exec<M: RegisterModules>(self) -> Result<()> {
        match self {
            Command::List => M::register(&mut ModuleConsolePrinter),
            Command::PrintDocumentation => M::register(&mut ModulesMarkdownPrinter),
        }
        .whatever_context("Modules command failed")?;
        Ok(())
    }
}

struct ModuleConsolePrinter;

impl ModulesRegistrar for ModuleConsolePrinter {
    type Error = Infallible;

    fn register<M: SignalingModuleDescription>(&mut self) -> Result<(), Infallible> {
        println!(
            "{}: [{}]",
            M::MODULE_ID,
            M::FEATURES
                .iter()
                .map(|f| format!("\"{}\"", f.feature_id))
                .join(", ")
        );
        Ok(())
    }
}

struct ModulesMarkdownPrinter;

impl ModulesRegistrar for ModulesMarkdownPrinter {
    type Error = Infallible;

    fn register<M: SignalingModule>(&mut self) -> Result<(), Infallible> {
        println!("{}", M::generate_markdown());
        Ok(())
    }
}

trait MarkdownDocumentGenerator {
    fn generate_markdown() -> String;
}

impl<T: SignalingModuleDescription> MarkdownDocumentGenerator for T {
    fn generate_markdown() -> String {
        format!(
            "## Module `{}`\n\n{}\n\n### Features\n\n{}",
            T::MODULE_ID,
            T::DESCRIPTION,
            generate_features_documentation(&T::MODULE_ID, T::FEATURES)
        )
    }
}

fn generate_features_documentation(
    module_id: &ModuleId,
    features: &[SignalingModuleFeatureDescription],
) -> String {
    if features.is_empty() {
        return "This module does not provide any configurable features.\n".to_string();
    }

    format!(
        "The following features can be configured for the module. All features are enabled by default and can be disabled either [by configuration](https://docs.opentalk.eu/admin/controller/advanced/defaults/) or [by tariff](https://docs.opentalk.eu/admin/controller/advanced/tariffs/).\n\n{}\n",
        features
            .iter()
            .map(|feature| generate_feature_documentation(module_id, feature))
            .join("\n\n")
    )
}

fn generate_feature_documentation(
    module_id: &ModuleId,
    feature: &SignalingModuleFeatureDescription,
) -> String {
    format!(
        "#### `{}::{}`\n\n{}",
        module_id, feature.feature_id, feature.description
    )
}
