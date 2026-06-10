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
use serde::Deserialize;
use std::path::Path;


#[derive(Debug, Deserialize)]
pub struct ValidationTestCase {
    pub description: String,
    pub inputs: Inputs,
    pub manifests: Vec<ManifestExpectation>,
    #[serde(rename = "validatorSpecVersions", default)]
    pub validator_spec_versions: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Inputs {
    #[serde(rename = "assetPath")]
    pub asset_path: String,
    #[serde(rename = "claimSignerTrustListPaths")]
    pub claim_signer_trust_list_paths: Vec<String>,
    #[serde(rename = "tsaTrustListPaths")]
    pub tsa_trust_list_paths: Vec<String>,
    #[serde(rename = "validationTime", default)]
    pub validation_time: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ManifestExpectation {
    #[serde(default)]
    pub failures: StatusCodesExpectations,
    #[serde(default)]
    pub successes: StatusCodesExpectations,
    #[serde(default)]
    pub informationals: StatusCodesExpectations,
}

/// Unit struct for the schema's empty-object sentinel `{}`.
/// serde_yaml parses `isEmpty: {}` into this cleanly.
#[derive(Debug, Deserialize, Default)]
pub struct EmptyObject {}

#[derive(Debug, Deserialize, Default)]
pub struct StatusCodesExpectations {
    #[serde(rename = "isEmpty")]
    pub is_empty: Option<EmptyObject>,
    #[serde(rename = "isNotEmpty")]
    pub is_not_empty: Option<EmptyObject>,
    #[serde(rename = "containsExactly")]
    pub contains_exactly: Option<StatusCodeSet>,
    #[serde(rename = "containsAllOf")]
    pub contains_all_of: Option<StatusCodeSet>,
    #[serde(rename = "containsNoneOf")]
    pub contains_none_of: Option<StatusCodeSet>,
    #[serde(rename = "containsAnyOf")]
    pub contains_any_of: Option<Vec<StatusCodeSet>>,
}

#[derive(Debug, Deserialize)]
pub struct StatusCodeSet {
    #[serde(default)]
    pub codes: Vec<String>,
}

impl StatusCodesExpectations {
    /// Returns (passed, Vec<failure_reason>).
    pub fn check(&self, actual: &[String]) -> (bool, Vec<String>) {
        let mut failures = Vec::new();
        let actual_set: std::collections::HashSet<&str> =
            actual.iter().map(String::as_str).collect();

        if self.is_empty.is_some() && !actual.is_empty() {
            failures.push(format!("expected empty, got: {:?}", actual));
        }

        if self.is_not_empty.is_some() && actual.is_empty() {
            failures.push("expected non-empty, got empty set".to_string());
        }

        if let Some(set) = &self.contains_exactly {
            let expected_set: std::collections::HashSet<&str> =
                set.codes.iter().map(String::as_str).collect();
            if actual_set != expected_set {
                failures.push(format!(
                    "expected exactly {:?}, got {:?}",
                    set.codes, actual
                ));
            }
        }

        if let Some(set) = &self.contains_all_of {
            let missing: Vec<&str> = set
                .codes
                .iter()
                .map(String::as_str)
                .filter(|c| !actual_set.contains(c))
                .collect();
            if !missing.is_empty() {
                failures.push(format!("missing required codes: {:?}", missing));
            }
        }

        if let Some(set) = &self.contains_none_of {
            let found: Vec<&str> = set
                .codes
                .iter()
                .map(String::as_str)
                .filter(|c| actual_set.contains(c))
                .collect();
            if !found.is_empty() {
                failures.push(format!("unexpected codes present: {:?}", found));
            }
        }

        if let Some(any_of_sets) = &self.contains_any_of {
            for (i, set) in any_of_sets.iter().enumerate() {
                let has_any = set.codes.iter().any(|c| actual_set.contains(c.as_str()));
                if !has_any {
                    failures.push(format!(
                        "containsAnyOf[{}]: none of {:?} found in actual",
                        i, set.codes
                    ));
                }
            }
        }

        (failures.is_empty(), failures)
    }
}

#[derive(Debug)]
pub struct ManifestResult {
    pub index: usize,
    pub pass: bool,
    pub reasons: Vec<String>,
    pub actual_successes: Vec<String>,
    pub actual_failures: Vec<String>,
    pub actual_informationals: Vec<String>,
}

#[derive(Debug)]
pub struct ValidationReport {
    pub description: String,
    pub overall_pass: bool,
    pub manifests: Vec<ManifestResult>,
    pub validation_time_ignored: bool,
}

impl ValidationReport {
    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("Description: {}", self.description));
        lines.push(format!(
            "Overall: {}",
            if self.overall_pass { "PASS" } else { "FAIL" }
        ));
        for m in &self.manifests {
            lines.push(format!(
                "  Manifest[{}]: {}",
                m.index,
                if m.pass { "PASS" } else { "FAIL" }
            ));
            for r in &m.reasons {
                lines.push(format!("    - {}", r));
            }
        }
        lines.join("\n")
    }
}

/// Load and evaluate a validation test-case YAML file.
/// Paths inside the YAML are resolved relative to the YAML file's parent directory.
pub fn run_validation(yaml_path: &Path) -> Result<ValidationReport> {
    let yaml_dir = yaml_path.parent().unwrap_or_else(|| Path::new("."));

    let yaml_str = std::fs::read_to_string(yaml_path)
        .with_context(|| format!("Failed to read validation YAML: {}", yaml_path.display()))?;

    let test_case: ValidationTestCase = serde_yaml::from_str(&yaml_str)
        .with_context(|| format!("Failed to parse validation YAML: {}", yaml_path.display()))?;

    let asset_path = yaml_dir.join(&test_case.inputs.asset_path);

    // Collect unique PEM file paths (claim signer + TSA trust lists combined)
    let all_pem_paths: Vec<&String> = test_case
        .inputs
        .claim_signer_trust_list_paths
        .iter()
        .chain(test_case.inputs.tsa_trust_list_paths.iter())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let mut trust_pem = String::new();
    for pem_path in &all_pem_paths {
        let full = yaml_dir.join(pem_path.as_str());
        let pem = std::fs::read_to_string(&full)
            .with_context(|| format!("Failed to read trust list PEM: {}", full.display()))?;
        trust_pem.push_str(&pem);
    }

    if trust_pem.trim().is_empty() {
        anyhow::bail!(
            "No trust anchors provided (claimSignerTrustListPaths and tsaTrustListPaths are both empty)"
        );
    }

    let settings = crate::build_trust_settings(&trust_pem, None, None)
        .context("Failed to configure trust anchors")?;

    let context = c2pa::Context::new()
        .with_settings(settings)
        .context("Failed to create c2pa context with trust settings")?;

    let reader = c2pa::Reader::from_context(context)
        .with_file(&asset_path)
        .with_context(|| format!("Failed to read asset: {}", asset_path.display()))?;

    let crjson_value: serde_json::Value =
        serde_json::from_str(&reader.crjson()).context("Failed to parse crJSON output")?;

    let manifests_json = crjson_value["manifests"]
        .as_array()
        .context("crJSON has no 'manifests' array")?;

    let validation_time_ignored = test_case.inputs.validation_time.is_some();

    // Empty manifests in test case = expect no C2PA manifests in asset
    if test_case.manifests.is_empty() {
        let overall_pass = manifests_json.is_empty();
        return Ok(ValidationReport {
            description: test_case.description,
            overall_pass,
            manifests: if overall_pass {
                vec![]
            } else {
                vec![ManifestResult {
                    index: 0,
                    pass: false,
                    reasons: vec![format!(
                        "expected no manifests but asset contains {}",
                        manifests_json.len()
                    )],
                    actual_successes: vec![],
                    actual_failures: vec![],
                    actual_informationals: vec![],
                }]
            },
            validation_time_ignored,
        });
    }

    let mut manifest_results = Vec::new();
    let mut overall_pass = true;

    for (i, expected) in test_case.manifests.iter().enumerate() {
        let manifest_json = manifests_json.get(i).with_context(|| {
            format!(
                "crJSON has {} manifest(s) but test case expects at least {}",
                manifests_json.len(),
                i + 1
            )
        })?;

        let vr = &manifest_json["validationResults"];

        let actual_successes = extract_codes(&vr["success"]);
        let actual_failures = extract_codes(&vr["failure"]);
        let actual_informationals = extract_codes(&vr["informational"]);

        let mut reasons = Vec::new();

        let (s_pass, s_reasons) = expected.successes.check(&actual_successes);
        for r in s_reasons {
            reasons.push(format!("[successes] {}", r));
        }

        let (f_pass, f_reasons) = expected.failures.check(&actual_failures);
        for r in f_reasons {
            reasons.push(format!("[failures] {}", r));
        }

        let (i_pass, i_reasons) = expected.informationals.check(&actual_informationals);
        for r in i_reasons {
            reasons.push(format!("[informationals] {}", r));
        }

        let pass = s_pass && f_pass && i_pass;
        if !pass {
            overall_pass = false;
        }
        manifest_results.push(ManifestResult {
            index: i,
            pass,
            reasons,
            actual_successes,
            actual_failures,
            actual_informationals,
        });
    }

    Ok(ValidationReport {
        description: test_case.description,
        overall_pass,
        manifests: manifest_results,
        validation_time_ignored,
    })
}

fn extract_codes(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|entry| entry["code"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}
