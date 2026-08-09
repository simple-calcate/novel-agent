use novel_domain::{PluginManifest, PluginPlatform};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("unsupported plugin api version {0}")]
    UnsupportedApi(u32),
    #[error("plugin does not support current platform")]
    UnsupportedPlatform,
}

pub const CURRENT_PLUGIN_API: u32 = 1;

pub fn parse_manifest(json: &str, platform: PluginPlatform) -> Result<PluginManifest, ManifestError> {
    let manifest: PluginManifest = serde_json::from_str(json)?;
    if manifest.api_version > CURRENT_PLUGIN_API {
        return Err(ManifestError::UnsupportedApi(manifest.api_version));
    }
    if !manifest.platforms.contains(&platform) {
        return Err(ManifestError::UnsupportedPlatform);
    }
    Ok(manifest)
}
