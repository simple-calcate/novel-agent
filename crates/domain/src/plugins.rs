use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 插件 id：与插件生态（清单 JSON、TS SDK）一致的字符串 slug。
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: u32,
    pub platforms: Vec<PluginPlatform>,
    pub operations: Vec<PluginOperation>,
    pub requested_capabilities: BTreeSet<Capability>,
    #[serde(default)]
    pub settings_schema: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PluginPlatform {
    Linux,
    Windows,
    Android,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginOperation {
    pub name: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub triggers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Capability {
    ReadSelection,
    ReadChapter {
        scope: String,
    },
    ReadStoryModel,
    ProposePatch,
    Log,
    Network {
        allowlist: Vec<String>,
    },
    Model {
        provider: String,
        max_cost_micros: u64,
    },
    PrivateStorage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginGrant {
    pub plugin_id: String,
    pub capabilities: BTreeSet<Capability>,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginResult {
    pub output: Value,
    pub logs: Vec<String>,
}
