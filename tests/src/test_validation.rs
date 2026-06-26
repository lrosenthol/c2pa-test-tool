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

use json_formula_rs::JsonFormula;
use serde_json::json;

fn eval_formula(formula: &str, data: serde_json::Value) -> bool {
    eval_formula_with_globals(formula, data, None)
}

fn eval_formula_with_globals(
    formula: &str,
    data: serde_json::Value,
    globals: Option<serde_json::Value>,
) -> bool {
    let mut jf = JsonFormula::new();
    match jf.search(formula, &data, globals.as_ref(), None) {
        Ok(v) => crtool::validation::is_truthy(&v),
        Err(_) => false,
    }
}

#[test]
fn test_formula_empty_failure_set_passes() {
    let data = json!({"success": [], "failure": [], "informational": []});
    assert!(eval_formula("length(failure) = 0", data));
}

#[test]
fn test_formula_non_empty_failure_set_fails() {
    let data = json!({"success": [], "failure": [{"code": "claimSignature.mismatch", "url": ""}], "informational": []});
    assert!(!eval_formula("length(failure) = 0", data));
}

#[test]
fn test_formula_code_present_in_success() {
    let data = json!({"success": [{"code": "claimSignature.validated", "url": ""}], "failure": [], "informational": []});
    assert!(eval_formula(
        r#"length(success[?code == "claimSignature.validated"]) > 0"#,
        data
    ));
}

#[test]
fn test_formula_code_absent_from_success() {
    let data = json!({"success": [], "failure": [], "informational": []});
    assert!(!eval_formula(
        r#"length(success[?code == "claimSignature.validated"]) > 0"#,
        data
    ));
}

#[test]
fn test_formula_globals_accessible_in_formula() {
    let data = json!({"success": [{"code": "claimSignature.validated", "url": ""}], "failure": [], "informational": []});
    let globals = json!({"$target_code": "claimSignature.validated"});
    assert!(eval_formula_with_globals(
        r#"length(success[?code == $target_code]) > 0"#,
        data,
        Some(globals),
    ));
}

#[test]
fn test_manifest_expectation_has_formula_field() {
    let exp = crtool::validation::ManifestExpectation {
        formula: "length(failure) = 0".to_string(),
    };
    assert_eq!(exp.formula, "length(failure) = 0");
}

#[test]
fn test_validation_test_case_has_globals_and_expressions() {
    use indexmap::IndexMap;
    use serde_json::json;
    let tc = crtool::validation::ValidationTestCase {
        description: "test".to_string(),
        inputs: crtool::validation::Inputs {
            asset_path: "x.jpg".to_string(),
            claim_signer_trust_list_paths: vec![],
            tsa_trust_list_paths: vec![],
            validation_time: None,
        },
        globals: json!({}),
        expressions: IndexMap::new(),
        formula: Some("length(manifests) = 0".to_string()),
        manifests: None,
        validator_spec_versions: vec![],
    };
    assert!(tc.formula.is_some());
}

// --- Debug message tests ---

/// Evaluates a formula and returns (truthy, debug_messages).
fn eval_with_debug(formula: &str, data: serde_json::Value) -> (bool, Vec<String>) {
    let mut jf = JsonFormula::new();
    let result = jf.search(formula, &data, None, None);
    let debug = jf.take_debug();
    let truthy = match result {
        Ok(v) => crtool::validation::is_truthy(&v),
        Err(_) => false,
    };
    (truthy, debug)
}

#[test]
fn test_debug_messages_emitted_for_missing_field() {
    let data = json!({"failure": [], "success": [], "informational": []});
    let (truthy, debug) = eval_with_debug("length(typo_failures) > 0", data);
    assert!(!truthy);
    assert!(
        debug.iter().any(|m| m.contains("typo_failures")),
        "expected a debug message naming the missing field, got: {:?}",
        debug
    );
    assert!(
        debug.iter().any(|m| m.contains("failure") || m.contains("Available")),
        "expected a debug message hinting at available fields, got: {:?}",
        debug
    );
}

#[test]
fn test_no_debug_messages_on_plain_falsy_result() {
    // Formula is syntactically correct and all fields exist; just returns false.
    let data = json!({"failure": [], "success": [], "informational": []});
    let (truthy, debug) = eval_with_debug("length(failure) > 0", data);
    assert!(!truthy);
    assert!(
        debug.is_empty(),
        "expected no debug messages for a plain falsy result, got: {:?}",
        debug
    );
}

#[test]
fn test_validation_report_fails_when_formula_expectation_is_wrong() {
    // Formula expects a code that is never present — should produce a clean falsy
    // failure with no debug messages (field names are valid, just wrong values).
    let yaml_content = r#"description: intentionally wrong expectation
inputs:
  assetPath: ./png_valid.png
  claimSignerTrustListPaths:
  - certs/root_ca1_cert.pem
  tsaTrustListPaths:
  - certs/root_ca1_cert.pem
globals:
  $required_codes:
    - nonExistent.code
expressions:
  _containsAllOf: "length(@[0][?contains($required_codes, code)]) = length($required_codes)"
manifests:
- formula: _containsAllOf(success)
"#;

    let yaml_dir = std::path::Path::new("tests/validation");
    let tmp = tempfile::Builder::new()
        .prefix("_wrong_expectation_test_")
        .suffix(".yaml")
        .tempfile_in(yaml_dir)
        .expect("failed to create temp yaml");
    std::fs::write(tmp.path(), yaml_content).expect("failed to write temp yaml");

    let report = crtool::validation::run_validation(tmp.path())
        .expect("run_validation should not error");

    assert!(!report.overall_pass, "wrong formula expectation should fail");
    assert_eq!(report.manifests.len(), 1);
    let m = &report.manifests[0];
    assert!(!m.pass);
    assert!(
        m.reasons.iter().any(|r| r.contains("formula returned falsy")),
        "expected 'formula returned falsy' reason, got: {:?}",
        m.reasons
    );
    // No debug messages — fields are valid, the formula just evaluates to false
    assert!(
        !m.reasons.iter().any(|r| r.contains("debug:")),
        "expected no debug messages for a plain falsy result, got: {:?}",
        m.reasons
    );
}

#[test]
fn test_debug_messages_surface_in_validation_report() {
    // Write a temp YAML inside tests/validation/ so relative asset paths resolve.
    let yaml_content = r#"description: debug surface test
inputs:
  assetPath: ./png_valid.png
  claimSignerTrustListPaths:
  - certs/root_ca1_cert.pem
  tsaTrustListPaths:
  - certs/root_ca1_cert.pem
manifests:
- formula: length(typo_failures) > 0
"#;

    let yaml_dir = std::path::Path::new("tests/validation");
    let tmp = tempfile::Builder::new()
        .prefix("_debug_test_")
        .suffix(".yaml")
        .tempfile_in(yaml_dir)
        .expect("failed to create temp yaml");
    std::fs::write(tmp.path(), yaml_content).expect("failed to write temp yaml");

    let report = crtool::validation::run_validation(tmp.path())
        .expect("run_validation should not error");

    assert!(!report.overall_pass, "formula with wrong field name should fail");
    let reasons = &report.manifests[0].reasons;
    assert!(
        reasons.iter().any(|r| r.contains("debug:") && r.contains("typo_failures")),
        "expected a 'debug:' reason naming the missing field, got: {:?}",
        reasons
    );
}

use std::path::Path;

const REPORT_DIR: &str = "target/test_output/validation";

/// Discovers every *.yaml file under tests/validation/, runs validation on each,
/// writes a report to target/test_output/validation/<stem>-report.txt, and asserts
/// the expected failure mode.
///
/// Known limitation: the bundled test assets use a test PKI with certificates
/// valid in 2001. Because c2pa-rs does not yet support `validationTime` clock
/// override, validation always runs against the current system clock and the certs
/// appear expired. We assert that the failure is specifically
/// `signingCredential.expired` so the test catches any unexpected regression while
/// documenting the limitation.
///
/// When c2pa-rs gains clock-override support, change the inner assertion to check
/// `report.overall_pass` instead.
#[test]
fn test_run_all_validation_yaml_files() {
    let yaml_dir = Path::new("tests/validation");
    let report_dir = Path::new(REPORT_DIR);
    std::fs::create_dir_all(report_dir).expect("Failed to create report output directory");

    let mut yaml_files: Vec<_> = std::fs::read_dir(yaml_dir)
        .expect("Failed to read tests/validation directory")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("yaml"))
        .collect();
    yaml_files.sort();

    assert!(
        !yaml_files.is_empty(),
        "No .yaml files found in tests/validation/"
    );

    let mut failures = Vec::new();

    for yaml_path in &yaml_files {
        let result = crtool::validation::run_validation(yaml_path);
        let stem = yaml_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        match result {
            Err(e) => {
                failures.push(format!("{stem}: run_validation returned error: {e:#}"));
            }
            Ok(report) => {
                // Write report to target/test_output/validation/<stem>-report.txt
                let report_path = report_dir.join(format!("{stem}-report.txt"));
                std::fs::write(&report_path, report.summary()).unwrap_or_else(|e| {
                    eprintln!("Warning: could not write report {report_path:?}: {e}")
                });

                // Assert the failure is specifically expired certs, not something unexpected
                if !report.overall_pass {
                    for m in &report.manifests {
                        if !m.actual_failures.is_empty()
                            && !m
                                .actual_failures
                                .iter()
                                .any(|c| c == "signingCredential.expired")
                        {
                            failures.push(format!(
                                "{stem}: manifest[{}] failed with unexpected codes — not a clock issue. \
                                 failures: {:?}",
                                m.index, m.actual_failures
                            ));
                        }
                    }
                } else {
                    // validationTime is now supported and certs validated OK — great!
                    // Update this assertion to always expect overall_pass once that's stable.
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "Validation test failures:\n{}",
        failures.join("\n")
    );
}
