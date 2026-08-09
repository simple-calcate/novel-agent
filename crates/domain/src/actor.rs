use crate::PluginId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Actor {
    User { user_id: Option<String> },
    Agent { model: String },
    Plugin { plugin_id: PluginId },
    System,
    Import,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventSource {
    Editor,
    Importer,
    Agent,
    Plugin,
    Workflow,
    Sync,
    System,
}
