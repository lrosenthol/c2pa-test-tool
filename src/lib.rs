/*
Copyright 2025 Adobe. All rights reserved.
This file is licensed to you under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License. You may obtain a copy
of the License at http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software distributed under
the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR REPRESENTATIONS
OF ANY KIND, either express or implied. See the License for the specific language
governing permissions and limitations under the License.
*/

//! # c2pa-test-tool Library
//!
//! Core library for creating and working with C2PA test assets.

use anyhow::Result;
use std::path::Path;

/// Re-export so callers (e.g. tests) can use explicit Settings without depending on c2pa.
pub use c2pa::Settings;

pub mod validation;

/// File extensions for asset types supported by c2pa-rs for reading/embedding C2PA manifests.
/// Matches the formats listed in c2pa-rs [supported-formats](https://github.com/contentauth/c2pa-rs/blob/main/docs/supported-formats.md).
pub const SUPPORTED_ASSET_EXTENSIONS: &[&str] = &[
    "avi", "avif", "c2pa", "dng", "gif", "heic", "heif", "jpg", "jpeg", "m4a", "mov", "mp3", "mp4",
    "pdf", "png", "svg", "tif", "tiff", "wav", "webp",
];

/// Returns whether a file path has an extension that c2pa-rs supports for C2PA operations.
pub fn is_supported_asset_path<P: AsRef<Path>>(path: P) -> bool {
    let ext = match path.as_ref().extension().and_then(|e| e.to_str()) {
        Some(e) => e.to_lowercase(),
        None => return false,
    };
    SUPPORTED_ASSET_EXTENSIONS.contains(&ext.as_str())
}

/// Trust list URLs: official C2PA trust list and Content Credentials interim list.
/// See <https://opensource.contentauthenticity.org/docs/c2patool/docs/usage/#configuring-trust-support>.
pub const C2PA_TRUST_ANCHORS_URL: &str =
    "https://raw.githubusercontent.com/c2pa-org/conformance-public/refs/heads/main/trust-list/C2PA-TRUST-LIST.pem";
pub const INTERIM_TRUST_ANCHORS_URL: &str = "https://contentcredentials.org/trust/anchors.pem";
pub const INTERIM_ALLOWED_LIST_URL: &str =
    "https://contentcredentials.org/trust/allowed.sha256.txt";
pub const INTERIM_TRUST_CONFIG_URL: &str = "https://contentcredentials.org/trust/store.cfg";

fn trust_settings_toml(
    trust_anchors: &str,
    allowed_list: Option<&str>,
    trust_config: Option<&str>,
) -> String {
    fn escape_toml_literal(s: &str) -> String {
        s.replace('\'', "''")
    }
    let mut toml = format!(
        "[trust]\ntrust_anchors = '''{}'''\n",
        escape_toml_literal(trust_anchors)
    );
    if let Some(al) = allowed_list {
        toml.push_str(&format!(
            "allowed_list = '''{}'''\n",
            escape_toml_literal(al)
        ));
    }
    if let Some(tc) = trust_config {
        toml.push_str(&format!(
            "trust_config = '''{}'''\n",
            escape_toml_literal(tc)
        ));
    }
    toml.push_str("\n[verify]\nverify_trust = true\n");
    toml
}

/// Builds Settings with trust validation enabled (trust anchors, optional allowed list and trust config).
pub fn build_trust_settings(
    trust_anchors: &str,
    allowed_list: Option<&str>,
    trust_config: Option<&str>,
) -> Result<Settings> {
    let toml = trust_settings_toml(trust_anchors, allowed_list, trust_config);
    Settings::default()
        .with_toml(&toml)
        .map_err(|e| anyhow::anyhow!("Failed to build trust settings: {}", e))
}

/// Returns default Settings for operations when trust lists are not used.
pub fn default_settings() -> Settings {
    Settings::default()
}

/// Applies C2PA trust settings to the thread-local Settings.
#[allow(deprecated)]
pub fn apply_trust_settings(
    trust_anchors: &str,
    allowed_list: Option<&str>,
    trust_config: Option<&str>,
) -> Result<()> {
    let toml = trust_settings_toml(trust_anchors, allowed_list, trust_config);
    Settings::from_toml(&toml)
        .map_err(|e| anyhow::anyhow!("Failed to apply trust settings: {}", e))?;
    Ok(())
}
