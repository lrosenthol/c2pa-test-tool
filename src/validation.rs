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
use indexmap::IndexMap;
use json_formula_rs::{JsonFormula, JsonFormulaError};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct ValidationTestCase {
    pub description: String,
    pub inputs: Inputs,
    #[serde(default)]
    pub globals: serde_json::Value,
    #[serde(default)]
    pub expressions: IndexMap<String, String>,
    pub formula: Option<String>,
    pub manifests: Option<Vec<ManifestExpectation>>,
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

#[derive(Debug, Deserialize)]
pub struct ManifestExpectation {
    pub formula: String,
}


pub fn is_truthy(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Null => false,
        serde_json::Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        serde_json::Value::String(s) => !s.is_empty(),
        serde_json::Value::Array(a) => !a.is_empty(),
        serde_json::Value::Object(o) => !o.is_empty(),
    }
}

fn build_globals(
    globals: &serde_json::Value,
    expressions: &IndexMap<String, String>,
) -> serde_json::Value {
    let mut merged = match globals.as_object() {
        Some(m) => m.clone(),
        None => serde_json::Map::new(),
    };
    for (name, expr) in expressions {
        merged.insert(name.clone(), serde_json::Value::String(expr.clone()));
    }
    serde_json::Value::Object(merged)
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
        let banner = if self.overall_pass {
            "✅ PASS"
        } else {
            "❌ FAIL"
        };
        let mut lines = vec![format!("{} — {}", banner, self.description)];
        if self.validation_time_ignored {
            lines.push(
                "  ⚠ validationTime was specified but ignored (not supported by c2pa-rs)"
                    .to_string(),
            );
        }
        for m in &self.manifests {
            let label = if m.pass { "✅ PASS" } else { "❌ FAIL" };
            lines.push(format!("  Manifest[{}]: {}", m.index, label));
            if !m.pass {
                for r in &m.reasons {
                    lines.push(format!("    ✗ {}", r));
                }
                if !m.actual_successes.is_empty() {
                    lines.push(format!(
                        "    actual successes:      {:?}",
                        m.actual_successes
                    ));
                }
                if !m.actual_failures.is_empty() {
                    lines.push(format!(
                        "    actual failures:       {:?}",
                        m.actual_failures
                    ));
                }
                if !m.actual_informationals.is_empty() {
                    lines.push(format!(
                        "    actual informationals: {:?}",
                        m.actual_informationals
                    ));
                }
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

    if test_case.inputs.validation_time.is_some() {
        eprintln!(
            "Warning: 'validationTime' is specified in the test case but is not yet \
             supported by c2pa-rs. Validation will use the current system clock."
        );
    }

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

    let mut jf = JsonFormula::new();
    let merged_globals = build_globals(&test_case.globals, &test_case.expressions);
    let globals_arg = if merged_globals.as_object().map(|o| o.is_empty()).unwrap_or(true) {
        None
    } else {
        Some(merged_globals.clone())
    };

    match (&test_case.formula, &test_case.manifests) {
        (Some(_), Some(_)) => {
            anyhow::bail!(
                "test case '{}': 'formula' and 'manifests' are mutually exclusive",
                test_case.description
            );
        }
        (None, None) => {
            anyhow::bail!(
                "test case '{}': one of 'formula' or 'manifests' must be present",
                test_case.description
            );
        }
        (Some(top_formula), None) => {
            let result = jf.search(top_formula, &crjson_value, globals_arg.as_ref(), None);
            let (pass, reason) = formula_result_to_pass(result, top_formula);
            return Ok(ValidationReport {
                description: test_case.description,
                overall_pass: pass,
                manifests: if pass {
                    vec![]
                } else {
                    vec![ManifestResult {
                        index: 0,
                        pass: false,
                        reasons: vec![reason],
                        actual_successes: vec![],
                        actual_failures: vec![],
                        actual_informationals: vec![],
                    }]
                },
                validation_time_ignored,
            });
        }
        (None, Some(expected_manifests)) => {
            let mut manifest_results = Vec::new();
            let mut overall_pass = true;

            for (i, expected) in expected_manifests.iter().enumerate() {
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

                let result = jf.search(&expected.formula, vr, globals_arg.as_ref(), None);
                let (pass, reason) = formula_result_to_pass(result, &expected.formula);

                if !pass {
                    overall_pass = false;
                }
                manifest_results.push(ManifestResult {
                    index: i,
                    pass,
                    reasons: if pass { vec![] } else { vec![reason] },
                    actual_successes,
                    actual_failures,
                    actual_informationals,
                });
            }

            return Ok(ValidationReport {
                description: test_case.description,
                overall_pass,
                manifests: manifest_results,
                validation_time_ignored,
            });
        }
    }
}

fn formula_result_to_pass(
    result: Result<serde_json::Value, JsonFormulaError>,
    formula: &str,
) -> (bool, String) {
    match result {
        Ok(v) if is_truthy(&v) => (true, String::new()),
        Ok(_) => (false, format!("formula returned falsy: {}", formula)),
        Err(e) => (false, format!("formula error: {} (formula: {})", e, formula)),
    }
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
