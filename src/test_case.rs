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

/// A C2PA validator test case loaded from a JSON file.
/// Follows the schema defined in `INTERNAL/schemas/test-case.schema.json`.
#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TestCase {
    pub test_id: String,
    pub title: Option<String>,
    #[allow(dead_code)]
    pub description: Option<String>,
    pub input_asset: Option<String>,
    pub manifest: serde_json::Value,
    pub signing_cert: String,
    pub signing_key: Option<String>,
    pub tsa_url: Option<String>,
    #[allow(dead_code)]
    pub expected_results: Option<serde_json::Value>,
}

/// Handle the `--create-test` mode: read a test case JSON file and produce a signed asset.
/// If `input_override` is provided, it takes precedence over the `inputAsset` field in the
/// test case JSON. If neither is present, an error is returned.
pub fn handle_create_test(
    test_case_path: &Path,
    input_override: Option<&Path>,
    output: &Path,
) -> Result<()> {
    println!(
        "=== Creating test asset from test case: {:?} ===",
        test_case_path
    );

    let json_str =
        fs::read_to_string(test_case_path).context("Failed to read test case JSON file")?;
    let test_case: TestCase = serde_json::from_str(&json_str)
        .context("Failed to parse test case JSON (does it match the test case schema?)")?;

    // All paths in the test case are resolved relative to the test case file's directory
    let base_dir = test_case_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    // CLI input overrides the JSON inputAsset field; error if neither is provided
    let input_asset = if let Some(override_path) = input_override {
        override_path.to_path_buf()
    } else if let Some(ref asset) = test_case.input_asset {
        base_dir.join(asset)
    } else {
        anyhow::bail!(
            "No input asset specified: the test case JSON does not include 'inputAsset' and \
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

    // Serialize the manifest object back to JSON string for the builder
    let manifest_json = serde_json::to_string(&test_case.manifest)
        .context("Failed to serialize manifest from test case")?;

    // Determine signing algorithm from manifest.alg, or auto-detect from certificate
    let signing_alg = if let Some(alg_str) = test_case.manifest.get("alg").and_then(|v| v.as_str())
    {
        parse_signing_algorithm(alg_str)?
    } else {
        println!("No alg in manifest — auto-detecting signing algorithm from certificate...");
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
