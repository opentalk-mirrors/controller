// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use crate::{CasbinRule, NewCasbinRule, Result};

/// A trait for retrieving and storing event entities.
#[async_trait::async_trait]
pub trait KustosInventory: Send {
    /// Load the casbin rules.
    async fn load_casbin_policy(&mut self) -> Result<Vec<CasbinRule>>;

    /// Clear the casbin rules.
    async fn clear_casbin_policy(&mut self) -> Result<()>;

    /// Save the casbin rules.
    async fn save_casbin_policy(&mut self, rules: Vec<NewCasbinRule>) -> Result<()>;

    /// Add casbin rules.
    async fn add_casbin_policies(&mut self, rules: Vec<NewCasbinRule>) -> Result<()>;

    /// Add a casbin rule.
    async fn add_casbin_policy(&mut self, rule: NewCasbinRule) -> Result<()>;

    /// Remove a casbin rule. Returns `true` if the rule was present before the deletion.
    async fn remove_casbin_policy(&mut self, ptype_c: &str, rule: Vec<String>) -> Result<bool>;

    /// Remove casbin rules. Returns `true` if the rules were present before the deletion.
    async fn remove_casbin_policies(
        &mut self,
        ptype_c: &str,
        rules: Vec<Vec<String>>,
    ) -> Result<bool>;

    /// Remove casbin rules, filtered by the field values.
    /// Returns `true` if the rules were present before the deletion.
    async fn remove_casbin_policy_filtered(
        &mut self,
        ptype_c: &str,
        field_index: usize,
        field_values: Vec<String>,
    ) -> Result<bool>;
}
