// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{module_resources::ModuleResourceId, users::UserId};

/// A helper struct for filtering module resources
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct ModuleResourceFilter {
    id: Option<ModuleResourceId>,
    namespace: Option<String>,
    created_by: Option<UserId>,
    tag: Option<String>,
    json: Option<serde_json::Value>,
}

impl ModuleResourceFilter {
    /// Create a new [`ModuleResourceFilter`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by the id of the module resource
    pub fn with_id(mut self, id: ModuleResourceId) -> Self {
        self.id = Some(id);

        self
    }

    /// Filter by the namespace of the module resource
    pub fn with_namespace(mut self, namespace: String) -> Self {
        self.namespace = Some(namespace);

        self
    }

    /// Filter by the creator of the module resource
    pub fn with_created_by(mut self, created_by: UserId) -> Self {
        self.created_by = Some(created_by);

        self
    }

    /// Filter by the tag of the module resource
    pub fn with_tag(mut self, tag: String) -> Self {
        self.tag = Some(tag);

        self
    }

    /// Filter by the content of the module resource
    pub fn with_json(mut self, json: serde_json::Value) -> Self {
        self.json = Some(json);

        self
    }
}

impl From<ModuleResourceFilter> for opentalk_db_storage::module_resources::Filter {
    fn from(
        ModuleResourceFilter {
            id,
            namespace,
            created_by,
            tag,
            json,
        }: ModuleResourceFilter,
    ) -> Self {
        let mut filter = Self::default();
        if let Some(id) = id {
            filter = filter.with_id(id);
        }
        if let Some(namespace) = namespace {
            filter = filter.with_namespace(namespace);
        }
        if let Some(created_by) = created_by {
            filter = filter.with_created_by(created_by);
        }
        if let Some(tag) = tag {
            filter = filter.with_tag(tag);
        }
        if let Some(json) = json {
            filter = filter.with_json(json);
        }
        filter
    }
}
