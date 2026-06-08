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

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::processing::{
    detect_signing_algorithm, parse_signing_algorithm, process_single_file, ProcessingConfig,
};

/// A C2PA validator test case loaded from a YAML file.
/// Follows the schema defined in `INTERNAL/schemas/test-case.schema.json`.
#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TestCase {
    pub test_id: String,
    pub title: Option<String>,
    #[allow(dead_code)]
    pub description: Option<String>,
    pub input_asset: Option<String>,
    pub alg: Option<String>,
    pub manifest: String,
    pub signing_cert: String,
    pub signing_key: Option<String>,
    pub tsa_url: Option<String>,
    #[allow(dead_code)]
    pub expected_results: Option<serde_json::Value>,
}

/// Handle the `--create-test` mode: read a test case YAML file and produce a signed asset.
/// If `input_override` is provided, it takes precedence over the `inputAsset` field in the
/// test case YAML. If neither is present, an error is returned.
pub fn handle_create_test(
    test_case_path: &Path,
    input_override: Option<&Path>,
    output: &Path,
) -> Result<()> {
    println!(
        "=== Creating test asset from test case: {:?} ===",
        test_case_path
    );

    let yaml_str =
        fs::read_to_string(test_case_path).context("Failed to read test case YAML file")?;
    let test_case: TestCase = serde_yaml::from_str(&yaml_str)
        .context("Failed to parse test case YAML (does it match the test case schema?)")?;

    // All paths in the test case are resolved relative to the test case file's directory
    let base_dir = test_case_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    // CLI input overrides the YAML inputAsset field; error if neither is provided
    let input_asset = if let Some(override_path) = input_override {
        override_path.to_path_buf()
    } else if let Some(ref asset) = test_case.input_asset {
        base_dir.join(asset)
    } else {
        anyhow::bail!(
            "No input asset specified: the test case YAML does not include 'inputAsset' and \
            no input file was provided on the command line."
        )
    };
    let cert = base_dir.join(&test_case.signing_cert);
    let key = base_dir.join(
        test_case
            .signing_key
            .as_deref()
            .unwrap_or(&test_case.signing_cert),
    );

    // Strip trailing newline added by YAML block scalar, use directly as JSON string
    let manifest_json = test_case.manifest.trim().to_owned();

    // Determine signing algorithm from top-level alg field, or auto-detect from certificate
    let signing_alg = if let Some(ref alg_str) = test_case.alg {
        parse_signing_algorithm(alg_str)?
    } else {
        println!("No alg in test case — auto-detecting signing algorithm from certificate...");
        let detected = detect_signing_algorithm(&cert)?;
        println!("  Detected: {:?}", detected);
        detected
    };

    println!("  Test ID:   {}", test_case.test_id);
    if let Some(title) = &test_case.title {
        println!("  Title:     {}", title);
    }
    println!("  Input:     {:?}", input_asset);
    println!("  Cert:      {:?}", cert);
    println!("  Algorithm: {:?}", signing_alg);
    if let Some(tsa) = &test_case.tsa_url {
        println!("  TSA URL:   {}", tsa);
    }

    let config = ProcessingConfig {
        manifest_json: &manifest_json,
        ingredients_base_dir: &base_dir,
        cert: &cert,
        key: &key,
        signing_alg,
        tsa_url: test_case.tsa_url.clone(),
        allow_self_signed: true, // test certs are typically self-signed
    };

    process_single_file(&input_asset, output, &config)?;

    println!("\n✓ Test asset created successfully");
    println!("  Output: {:?}", output);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_YAML: &str = r#"
testId: test.minimal
signingCert: certs/test.pem
manifest: |
  {"claim_generator_info": [{"name": "test"}], "assertions": []}
"#;

    const YAML_WITH_ALG: &str = r#"
testId: test.with-alg
alg: Es256
signingCert: certs/test.pem
manifest: |
  {"claim_generator_info": [{"name": "test"}], "assertions": []}
"#;

    #[test]
    fn test_parse_yaml_minimal() {
        let tc: TestCase = serde_yaml::from_str(MINIMAL_YAML).expect("failed to parse YAML");
        assert_eq!(tc.test_id, "test.minimal");
        assert_eq!(tc.signing_cert, "certs/test.pem");
        assert!(tc.manifest.trim().starts_with('{'));
        assert!(tc.alg.is_none());
    }

    #[test]
    fn test_parse_yaml_with_alg() {
        let tc: TestCase = serde_yaml::from_str(YAML_WITH_ALG).expect("failed to parse YAML");
        assert_eq!(tc.alg.as_deref(), Some("Es256"));
    }

    #[test]
    fn test_manifest_is_valid_json_after_trim() {
        let tc: TestCase = serde_yaml::from_str(MINIMAL_YAML).expect("failed to parse YAML");
        let trimmed = tc.manifest.trim();
        serde_json::from_str::<serde_json::Value>(trimmed)
            .expect("trimmed manifest must be valid JSON");
    }
}
