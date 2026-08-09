use novel_domain::{Capability, PluginGrant, PluginManifest};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDecision {
    pub granted: BTreeSet<Capability>,
    pub denied: BTreeSet<Capability>,
}

pub fn grant_intersection(
    manifest: &PluginManifest,
    approved: &BTreeSet<Capability>,
) -> PluginGrant {
    let capabilities = manifest
        .requested_capabilities
        .intersection(approved)
        .cloned()
        .collect();
    PluginGrant {
        plugin_id: manifest.id.clone(),
        capabilities,
        enabled: true,
    }
}

pub fn evaluate(
    manifest: &PluginManifest,
    approved: &BTreeSet<Capability>,
) -> CapabilityDecision {
    let granted = manifest
        .requested_capabilities
        .intersection(approved)
        .cloned()
        .collect();
    let denied = manifest
        .requested_capabilities
        .difference(approved)
        .cloned()
        .collect();
    CapabilityDecision { granted, denied }
}
