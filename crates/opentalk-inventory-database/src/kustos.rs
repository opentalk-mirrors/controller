// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use kustos_db::db::{
    add_policies, add_policy, clear_policy, load_policy, remove_filtered_policy, remove_policies,
    remove_policy, save_policy,
};
use opentalk_kustos_inventory::{CasbinRule, KustosInventory, NewCasbinRule};
use snafu::ResultExt as _;

use crate::{DatabaseConnection, Result, error::DatabaseSnafu};

#[async_trait::async_trait]
impl KustosInventory for DatabaseConnection {
    #[tracing::instrument(err, skip_all)]
    async fn load_casbin_policy(&mut self) -> Result<Vec<CasbinRule>> {
        let rules = load_policy(&mut self.inner).await.context(DatabaseSnafu)?;
        Ok(rules
            .into_iter()
            .map(
                |kustos_db::db::CasbinRule {
                     id,
                     ptype,
                     v0,
                     v1,
                     v2,
                     v3,
                     v4,
                     v5,
                 }| CasbinRule {
                    id,
                    ptype,
                    v0,
                    v1,
                    v2,
                    v3,
                    v4,
                    v5,
                },
            )
            .collect())
    }

    #[tracing::instrument(err, skip_all)]
    async fn clear_casbin_policy(&mut self) -> Result<()> {
        clear_policy(&mut self.inner).await.context(DatabaseSnafu)?;
        Ok(())
    }

    #[tracing::instrument(err, skip_all)]
    async fn save_casbin_policy(&mut self, rules: Vec<NewCasbinRule>) -> Result<()> {
        let rules = rules
            .into_iter()
            .map(
                |NewCasbinRule {
                     ptype,
                     v0,
                     v1,
                     v2,
                     v3,
                     v4,
                     v5,
                 }| kustos_db::db::NewCasbinRule {
                    ptype,
                    v0,
                    v1,
                    v2,
                    v3,
                    v4,
                    v5,
                },
            )
            .collect();
        save_policy(&mut self.inner, rules)
            .await
            .context(DatabaseSnafu)?;
        Ok(())
    }

    #[tracing::instrument(err, skip_all)]
    async fn add_casbin_policies(&mut self, rules: Vec<NewCasbinRule>) -> Result<()> {
        let rules = rules
            .into_iter()
            .map(
                |NewCasbinRule {
                     ptype,
                     v0,
                     v1,
                     v2,
                     v3,
                     v4,
                     v5,
                 }| kustos_db::db::NewCasbinRule {
                    ptype,
                    v0,
                    v1,
                    v2,
                    v3,
                    v4,
                    v5,
                },
            )
            .collect();
        add_policies(&mut self.inner, rules)
            .await
            .context(DatabaseSnafu)?;
        Ok(())
    }

    #[tracing::instrument(err, skip_all)]
    async fn add_casbin_policy(
        &mut self,
        NewCasbinRule {
            ptype,
            v0,
            v1,
            v2,
            v3,
            v4,
            v5,
        }: NewCasbinRule,
    ) -> Result<()> {
        let rule = kustos_db::db::NewCasbinRule {
            ptype,
            v0,
            v1,
            v2,
            v3,
            v4,
            v5,
        };

        add_policy(&mut self.inner, rule)
            .await
            .context(DatabaseSnafu)?;
        Ok(())
    }

    async fn remove_casbin_policy(&mut self, ptype_c: &str, rule: Vec<String>) -> Result<bool> {
        let removed = remove_policy(&mut self.inner, ptype_c, rule)
            .await
            .context(DatabaseSnafu)?;
        Ok(removed)
    }

    async fn remove_casbin_policies(
        &mut self,
        ptype_c: &str,
        rules: Vec<Vec<String>>,
    ) -> Result<bool> {
        let removed = remove_policies(&mut self.inner, ptype_c, rules)
            .await
            .context(DatabaseSnafu)?;
        Ok(removed)
    }

    async fn remove_casbin_policy_filtered(
        &mut self,
        ptype_c: &str,
        field_index: usize,
        field_values: Vec<String>,
    ) -> Result<bool> {
        let removed = remove_filtered_policy(&mut self.inner, ptype_c, field_index, field_values)
            .await
            .context(DatabaseSnafu)?;
        Ok(removed)
    }
}
