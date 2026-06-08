/*
Copyright 2026 Adobe. All rights reserved.
This file is licensed to you under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License. You may obtain a copy
of the License at http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software distributed under
the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR REPRESENTATIONS
OF ANY KIND, either express or implied. See the License for the specific language
governing permissions and limitations under the License.
*/

//! Tests for profile/rubric evaluation against crJSON indicators.
//!
//! The "profile" tests use the legacy `profiles/` directory (profile_metadata format).
//! The "rubric" tests use the `rubrics/` directory (rubric_metadata format) and exercise
//! the new `--rubric` CLI flag.

use anyhow::Result;
use profile_evaluator_rs::{evaluate_files, load_profile, serialize_report, OutputFormat};
use std::path::PathBuf;
use std::process::Command;

mod common;

fn profiles_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("profiles")
}

fn rubrics_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("rubrics")
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/jsons")
}

/// Find the compliance statement in the report and return its boolean value.
fn compliance_value(report: &serde_json::Value) -> Option<bool> {
    report
        .get("statements")
        .and_then(|s| s.as_array())
        .and_then(|sections| {
            sections.iter().find_map(|section| {
                section.as_array()?.iter().find_map(|stmt| {
                    if stmt.get("id")?.as_str()? == "c2pa:profile_compliance" {
                        stmt.get("value")?.as_bool()
                    } else {
                        None
                    }
                })
            })
        })
}

// ============================================================================
// Profile loading tests
// ============================================================================

#[test]
fn test_load_real_life_capture_profile() {
    let profile_path = profiles_dir().join("real-life-capture_profile.yml");
    assert!(profile_path.exists(), "Profile file should exist");
    load_profile(&profile_path).expect("real-life-capture profile should load without error");
}

#[test]
fn test_load_real_media_profile() {
    let profile_path = profiles_dir().join("real-media_profile.yml");
    assert!(profile_path.exists(), "Profile file should exist");
    load_profile(&profile_path).expect("real-media profile should load without error");
}

#[test]
fn test_load_human_illustration_profile() {
    let profile_path = profiles_dir().join("human-illustration_profile.yml");
    assert!(profile_path.exists(), "Profile file should exist");
    load_profile(&profile_path).expect("human-illustration profile should load without error");
}

#[test]
fn test_load_fully_generative_ai_profile() {
    let profile_path = profiles_dir().join("fully-generative-ai_profile.yml");
    assert!(profile_path.exists(), "Profile file should exist");
    load_profile(&profile_path).expect("fully-generative-ai profile should load without error");
}

// ============================================================================
// Compliance evaluation tests – compliant fixtures
// ============================================================================

#[test]
fn test_real_life_capture_profile_compliant() -> Result<()> {
    let report = evaluate_files(
        profiles_dir().join("real-life-capture_profile.yml"),
        fixtures_dir().join("real_life_capture_indicators.json"),
    )?;

    assert_eq!(
        compliance_value(&report),
        Some(true),
        "real_life_capture_indicators should be compliant with real-life-capture profile; report: {}",
        serde_json::to_string_pretty(&report).unwrap_or_default()
    );
    println!("✓ real-life-capture profile: compliant fixture passes");
    Ok(())
}

#[test]
fn test_real_media_profile_compliant() -> Result<()> {
    let report = evaluate_files(
        profiles_dir().join("real-media_profile.yml"),
        fixtures_dir().join("real_life_capture_indicators.json"),
    )?;

    assert_eq!(
        compliance_value(&report),
        Some(true),
        "real_life_capture_indicators should be compliant with real-media profile"
    );
    println!("✓ real-media profile: compliant fixture passes");
    Ok(())
}

#[test]
fn test_human_illustration_profile_compliant() -> Result<()> {
    let report = evaluate_files(
        profiles_dir().join("human-illustration_profile.yml"),
        fixtures_dir().join("human_illustration_indicators.json"),
    )?;

    assert_eq!(
        compliance_value(&report),
        Some(true),
        "human_illustration_indicators should be compliant with human-illustration profile; report: {}",
        serde_json::to_string_pretty(&report).unwrap_or_default()
    );
    println!("✓ human-illustration profile: compliant fixture passes");
    Ok(())
}

#[test]
fn test_fully_generative_ai_profile_compliant() -> Result<()> {
    let report = evaluate_files(
        profiles_dir().join("fully-generative-ai_profile.yml"),
        fixtures_dir().join("generative_ai_indicators.json"),
    )?;

    assert_eq!(
        compliance_value(&report),
        Some(true),
        "generative_ai_indicators should be compliant with fully-generative-ai profile; report: {}",
        serde_json::to_string_pretty(&report).unwrap_or_default()
    );
    println!("✓ fully-generative-ai profile: compliant fixture passes");
    Ok(())
}

// ============================================================================
// Compliance evaluation tests – non-compliant fixture
// ============================================================================

#[test]
fn test_real_life_capture_profile_non_compliant() -> Result<()> {
    let report = evaluate_files(
        profiles_dir().join("real-life-capture_profile.yml"),
        fixtures_dir().join("non_compliant_indicators.json"),
    )?;

    assert_eq!(
        compliance_value(&report),
        Some(false),
        "non_compliant_indicators should NOT be compliant with real-life-capture profile"
    );
    println!("✓ real-life-capture profile: non-compliant fixture correctly fails");
    Ok(())
}

#[test]
fn test_real_media_profile_non_compliant() -> Result<()> {
    let report = evaluate_files(
        profiles_dir().join("real-media_profile.yml"),
        fixtures_dir().join("non_compliant_indicators.json"),
    )?;

    assert_eq!(
        compliance_value(&report),
        Some(false),
        "non_compliant_indicators should NOT be compliant with real-media profile"
    );
    println!("✓ real-media profile: non-compliant fixture correctly fails");
    Ok(())
}

#[test]
fn test_generative_ai_profile_non_compliant() -> Result<()> {
    let report = evaluate_files(
        profiles_dir().join("fully-generative-ai_profile.yml"),
        fixtures_dir().join("non_compliant_indicators.json"),
    )?;

    assert_eq!(
        compliance_value(&report),
        Some(false),
        "non_compliant_indicators should NOT be compliant with fully-generative-ai profile"
    );
    println!("✓ fully-generative-ai profile: non-compliant fixture correctly fails");
    Ok(())
}

/// Generative-AI indicators should not be compliant with the real-life-capture profile.
#[test]
fn test_cross_profile_gen_ai_fails_real_life_capture() -> Result<()> {
    let report = evaluate_files(
        profiles_dir().join("real-life-capture_profile.yml"),
        fixtures_dir().join("generative_ai_indicators.json"),
    )?;

    assert_eq!(
        compliance_value(&report),
        Some(false),
        "generative_ai_indicators should NOT be compliant with real-life-capture profile"
    );
    println!("✓ cross-profile: gen-AI indicators correctly fail real-life-capture profile");
    Ok(())
}

/// Real-life-capture indicators should not be compliant with the fully-generative-ai profile.
#[test]
fn test_cross_profile_real_capture_fails_generative_ai() -> Result<()> {
    let report = evaluate_files(
        profiles_dir().join("fully-generative-ai_profile.yml"),
        fixtures_dir().join("real_life_capture_indicators.json"),
    )?;

    assert_eq!(
        compliance_value(&report),
        Some(false),
        "real_life_capture_indicators should NOT be compliant with fully-generative-ai profile"
    );
    println!("✓ cross-profile: real-capture indicators correctly fail fully-generative-ai profile");
    Ok(())
}

// ============================================================================
// Report serialization tests
// ============================================================================

#[test]
fn test_serialize_report_json() -> Result<()> {
    let report = evaluate_files(
        profiles_dir().join("real-media_profile.yml"),
        fixtures_dir().join("real_life_capture_indicators.json"),
    )?;

    let json_str = serialize_report(&report, OutputFormat::Json)?;

    // Valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&json_str)?;
    assert!(
        parsed.get("statements").is_some(),
        "JSON report should have statements"
    );
    assert!(
        json_str.contains('\n'),
        "JSON report should be pretty-printed"
    );

    println!("✓ Report serializes to valid pretty-printed JSON");
    Ok(())
}

#[test]
fn test_serialize_report_yaml() -> Result<()> {
    let report = evaluate_files(
        profiles_dir().join("real-media_profile.yml"),
        fixtures_dir().join("real_life_capture_indicators.json"),
    )?;

    let yaml_str = serialize_report(&report, OutputFormat::Yaml)?;

    // Must round-trip through serde_yaml
    let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml_str)?;
    assert!(
        parsed.get("statements").is_some(),
        "YAML report should have statements"
    );

    println!("✓ Report serializes to valid YAML");
    Ok(())
}

// ============================================================================
// Cross-profile compliance tests — missing combinations
// ============================================================================

/// Human-illustration indicators should not comply with the real-life-capture profile.
#[test]
fn test_cross_profile_human_illustration_fails_real_life_capture() -> Result<()> {
    let report = evaluate_files(
        profiles_dir().join("real-life-capture_profile.yml"),
        fixtures_dir().join("human_illustration_indicators.json"),
    )?;

    assert_eq!(
        compliance_value(&report),
        Some(false),
        "human_illustration_indicators should NOT comply with real-life-capture profile"
    );
    println!(
        "✓ cross-profile: human-illustration indicators correctly fail real-life-capture profile"
    );
    Ok(())
}

/// Human-illustration indicators should not comply with the fully-generative-ai profile.
#[test]
fn test_cross_profile_human_illustration_fails_fully_generative_ai() -> Result<()> {
    let report = evaluate_files(
        profiles_dir().join("fully-generative-ai_profile.yml"),
        fixtures_dir().join("human_illustration_indicators.json"),
    )?;

    assert_eq!(
        compliance_value(&report),
        Some(false),
        "human_illustration_indicators should NOT comply with fully-generative-ai profile"
    );
    println!(
        "✓ cross-profile: human-illustration indicators correctly fail fully-generative-ai profile"
    );
    Ok(())
}

/// Human-illustration profile non-compliant fixture test.
#[test]
fn test_human_illustration_profile_non_compliant() -> Result<()> {
    let report = evaluate_files(
        profiles_dir().join("human-illustration_profile.yml"),
        fixtures_dir().join("non_compliant_indicators.json"),
    )?;

    assert_eq!(
        compliance_value(&report),
        Some(false),
        "non_compliant_indicators should NOT comply with human-illustration profile"
    );
    println!("✓ human-illustration profile: non-compliant fixture correctly fails");
    Ok(())
}

// ============================================================================
// CLI integration tests (--profile flag)
// ============================================================================

#[test]
fn test_cli_standalone_profile_eval_json_output() -> Result<()> {
    let binary = common::cli_binary_path();
    let indicators = fixtures_dir().join("real_life_capture_indicators.json");
    let profile = profiles_dir().join("real-life-capture_profile.yml");

    let out_dir = common::output_dir().join("profile_eval");
    std::fs::create_dir_all(&out_dir)?;

    // Copy the indicators file into the output dir so the report lands there too
    let indicators_copy = out_dir.join("rlc_indicators.json");
    std::fs::copy(&indicators, &indicators_copy)?;

    let result = Command::new(&binary)
        .arg("--profile")
        .arg(&profile)
        .arg("--report-format")
        .arg("json")
        .arg(&indicators_copy)
        .output()?;

    assert!(
        result.status.success(),
        "CLI profile eval should succeed: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let report_path = out_dir.join("rlc_indicators-report.json");
    assert!(
        report_path.exists(),
        "Report file should be created at {:?}",
        report_path
    );

    let content = std::fs::read_to_string(&report_path)?;
    let parsed: serde_json::Value = serde_json::from_str(&content)?;
    assert!(
        parsed.get("statements").is_some(),
        "Report should have statements"
    );

    println!("✓ CLI --profile standalone eval writes JSON report");
    Ok(())
}

#[test]
fn test_cli_standalone_profile_eval_yaml_output() -> Result<()> {
    let binary = common::cli_binary_path();
    let indicators = fixtures_dir().join("generative_ai_indicators.json");
    let profile = profiles_dir().join("fully-generative-ai_profile.yml");

    let out_dir = common::output_dir().join("profile_eval");
    std::fs::create_dir_all(&out_dir)?;

    let indicators_copy = out_dir.join("genai_indicators.json");
    std::fs::copy(&indicators, &indicators_copy)?;

    let result = Command::new(&binary)
        .arg("--profile")
        .arg(&profile)
        .arg("--report-format")
        .arg("yaml")
        .arg(&indicators_copy)
        .output()?;

    assert!(
        result.status.success(),
        "CLI profile eval should succeed: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let report_path = out_dir.join("genai_indicators-report.yaml");
    assert!(
        report_path.exists(),
        "YAML report file should be created at {:?}",
        report_path
    );

    let content = std::fs::read_to_string(&report_path)?;
    let parsed: serde_yaml::Value = serde_yaml::from_str(&content)?;
    assert!(
        parsed.get("statements").is_some(),
        "YAML report should have statements"
    );

    println!("✓ CLI --profile standalone eval writes YAML report");
    Ok(())
}

/// CLI profile eval with non-compliant indicators: should exit 0 (evaluation itself succeeded),
/// but the written report must record compliance as false.
#[test]
fn test_cli_standalone_profile_eval_non_compliant_exits_zero() -> Result<()> {
    let binary = common::cli_binary_path();
    let indicators = fixtures_dir().join("non_compliant_indicators.json");
    let profile = profiles_dir().join("real-life-capture_profile.yml");

    let out_dir = common::output_dir().join("profile_eval_non_compliant");
    std::fs::create_dir_all(&out_dir)?;

    let indicators_copy = out_dir.join("non_compliant_indicators.json");
    std::fs::copy(&indicators, &indicators_copy)?;

    let result = Command::new(&binary)
        .arg("--profile")
        .arg(&profile)
        .arg(&indicators_copy)
        .output()?;

    println!("stdout: {}", String::from_utf8_lossy(&result.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&result.stderr));

    assert!(
        result.status.success(),
        "CLI should exit 0 even for non-compliant content (evaluation ran successfully): {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let report_path = out_dir.join("non_compliant_indicators-report.json");
    assert!(
        report_path.exists(),
        "Report file should be written: {report_path:?}"
    );

    let content = std::fs::read_to_string(&report_path)?;
    let parsed: serde_json::Value = serde_json::from_str(&content)?;
    assert_eq!(
        compliance_value(&parsed),
        Some(false),
        "Non-compliant input must produce compliance=false in the report"
    );

    println!("✓ CLI --profile non-compliant: exits 0, report has compliance=false");
    Ok(())
}

/// CLI profile eval with wrong-profile cross-test: gen-AI indicators against real-life-capture.
/// Should exit 0 and write a report with compliance=false.
#[test]
fn test_cli_standalone_profile_eval_wrong_profile_non_compliant() -> Result<()> {
    let binary = common::cli_binary_path();
    let indicators = fixtures_dir().join("generative_ai_indicators.json");
    let profile = profiles_dir().join("real-life-capture_profile.yml");

    let out_dir = common::output_dir().join("profile_eval_cross");
    std::fs::create_dir_all(&out_dir)?;

    let indicators_copy = out_dir.join("genai_for_rlc.json");
    std::fs::copy(&indicators, &indicators_copy)?;

    let result = Command::new(&binary)
        .arg("--profile")
        .arg(&profile)
        .arg(&indicators_copy)
        .output()?;

    assert!(
        result.status.success(),
        "CLI should exit 0 even for cross-profile non-compliance: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let report_path = out_dir.join("genai_for_rlc-report.json");
    assert!(
        report_path.exists(),
        "Report should be written: {report_path:?}"
    );

    let content = std::fs::read_to_string(&report_path)?;
    let parsed: serde_json::Value = serde_json::from_str(&content)?;
    assert_eq!(
        compliance_value(&parsed),
        Some(false),
        "Gen-AI indicators should not comply with real-life-capture profile"
    );

    println!("✓ CLI --profile cross-profile: exits 0, report has compliance=false");
    Ok(())
}

/// CLI profile eval with multiple input files: a separate report should be written for each.
#[test]
fn test_cli_standalone_profile_eval_multiple_inputs() -> Result<()> {
    let binary = common::cli_binary_path();
    let profile = profiles_dir().join("real-life-capture_profile.yml");

    let out_dir = common::output_dir().join("profile_eval_multi");
    std::fs::create_dir_all(&out_dir)?;

    // Two indicator files that should both pass the real-life-capture profile
    let copy1 = out_dir.join("rlc_multi_1.json");
    let copy2 = out_dir.join("rlc_multi_2.json");
    std::fs::copy(
        fixtures_dir().join("real_life_capture_indicators.json"),
        &copy1,
    )?;
    std::fs::copy(
        fixtures_dir().join("real_life_capture_indicators.json"),
        &copy2,
    )?;

    let result = Command::new(&binary)
        .arg("--profile")
        .arg(&profile)
        .arg(&copy1)
        .arg(&copy2)
        .output()?;

    println!("stdout: {}", String::from_utf8_lossy(&result.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&result.stderr));

    assert!(
        result.status.success(),
        "Multi-file profile eval should succeed: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let report1 = out_dir.join("rlc_multi_1-report.json");
    let report2 = out_dir.join("rlc_multi_2-report.json");
    assert!(report1.exists(), "Report 1 should be written: {report1:?}");
    assert!(report2.exists(), "Report 2 should be written: {report2:?}");

    println!("✓ CLI --profile multi-input: two reports written");
    Ok(())
}

/// CLI profile eval with a nonexistent profile file must fail with a non-zero exit code.
#[test]
fn test_cli_profile_missing_profile_file_fails() -> Result<()> {
    let binary = common::cli_binary_path();
    let indicators = fixtures_dir().join("real_life_capture_indicators.json");

    let out_dir = common::output_dir().join("profile_eval_error");
    std::fs::create_dir_all(&out_dir)?;

    let indicators_copy = out_dir.join("rlc_indicators_error.json");
    std::fs::copy(&indicators, &indicators_copy)?;

    let result = Command::new(&binary)
        .arg("--profile")
        .arg("/nonexistent/profile.yml")
        .arg(&indicators_copy)
        .output()?;

    println!("stderr: {}", String::from_utf8_lossy(&result.stderr));

    assert!(
        !result.status.success(),
        "CLI should fail when profile file does not exist"
    );

    println!("✓ CLI --profile missing profile file: correctly fails");
    Ok(())
}

// (--extract --profile combined-mode tests removed — manifest extraction is not part of this tool)

// ============================================================================
// Rubric support tests (rubric_metadata format + failIfMatched + reportText)
// ============================================================================

/// The conformance rubric uses `rubric_metadata` (not `profile_metadata`) and camelCase
/// field names (`reportText`, `failIfMatched`).  Verify it loads without error.
#[test]
fn test_load_conformance_rubric_spec24() {
    let rubric_path = rubrics_dir().join("asset-rubric-conformance0.2-spec2.4.yml");
    assert!(rubric_path.exists(), "Conformance rubric file should exist at {rubric_path:?}");
    load_profile(&rubric_path)
        .expect("conformance rubric (spec 2.4) should load without error");
}

/// Evaluating a crJSON against the conformance rubric should produce a well-formed report
/// with a non-empty `statements` array.  We use `valid_indicators.json` which is a
/// structurally complete crJSON and exercises the rubric's expressions.
#[test]
fn test_evaluate_conformance_rubric_produces_report() -> Result<()> {
    let rubric_path = rubrics_dir().join("asset-rubric-conformance0.2-spec2.4.yml");
    let indicators_path = fixtures_dir().join("valid_indicators.json");

    let report = evaluate_files(&rubric_path, &indicators_path)?;

    let statements = report
        .get("statements")
        .and_then(|s| s.as_array())
        .expect("report should contain a 'statements' array");
    assert!(!statements.is_empty(), "statements should not be empty");

    // Every entry in each section must have an 'id' and a 'value' or 'error' field.
    for section in statements {
        for stmt in section.as_array().expect("section should be an array") {
            assert!(
                stmt.get("id").is_some(),
                "each statement must have an id; got: {stmt}"
            );
        }
    }

    println!("✓ conformance rubric (spec 2.4): produces well-formed report");
    Ok(())
}

/// Verify that `failIfMatched` statements produce boolean values in the report:
/// the `value` field must be either `true` or `false` (never a raw array).
#[test]
fn test_fail_if_matched_statements_produce_booleans() -> Result<()> {
    let rubric_path = rubrics_dir().join("asset-rubric-conformance0.2-spec2.4.yml");
    let indicators_path = fixtures_dir().join("valid_indicators.json");

    let report = evaluate_files(&rubric_path, &indicators_path)?;

    // All validation:* statements use failIfMatched; verify their values are boolean.
    let mut checked = 0usize;
    if let Some(sections) = report.get("statements").and_then(|s| s.as_array()) {
        for section in sections {
            for stmt in section.as_array().unwrap_or(&vec![]) {
                let id = stmt.get("id").and_then(|v| v.as_str()).unwrap_or("");
                if id.starts_with("validation:") {
                    if let Some(value) = stmt.get("value") {
                        assert!(
                            value.is_boolean(),
                            "failIfMatched statement '{id}' should have a boolean value, got: {value}"
                        );
                        checked += 1;
                    }
                }
            }
        }
    }

    assert!(checked > 0, "expected at least one validation:* statement with a value");
    println!("✓ failIfMatched: {checked} validation statements have boolean values");
    Ok(())
}

// ============================================================================
// CLI integration tests — --rubric flag
// ============================================================================

/// `--rubric` is the primary flag; verify it produces a JSON report.
#[test]
fn test_cli_rubric_flag_json_output() -> Result<()> {
    let binary = common::cli_binary_path();
    let indicators = fixtures_dir().join("valid_indicators.json");
    let rubric = rubrics_dir().join("asset-rubric-conformance0.2-spec2.4.yml");

    let out_dir = common::output_dir().join("rubric_eval");
    std::fs::create_dir_all(&out_dir)?;

    let indicators_copy = out_dir.join("valid_indicators_rubric.json");
    std::fs::copy(&indicators, &indicators_copy)?;

    let result = Command::new(&binary)
        .arg("--rubric")
        .arg(&rubric)
        .arg("--report-format")
        .arg("json")
        .arg(&indicators_copy)
        .output()?;

    assert!(
        result.status.success(),
        "CLI --rubric eval should succeed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let report_path = out_dir.join("valid_indicators_rubric-report.json");
    assert!(report_path.exists(), "report should be written at {report_path:?}");

    let content = std::fs::read_to_string(&report_path)?;
    let parsed: serde_json::Value = serde_json::from_str(&content)?;
    assert!(parsed.get("statements").is_some(), "report should have statements");

    println!("✓ CLI --rubric: writes JSON report");
    Ok(())
}

/// `--profile` still works as a backward-compatible alias for `--rubric`.
#[test]
fn test_cli_profile_alias_still_works() -> Result<()> {
    let binary = common::cli_binary_path();
    let indicators = fixtures_dir().join("valid_indicators.json");
    let rubric = rubrics_dir().join("asset-rubric-conformance0.2-spec2.4.yml");

    let out_dir = common::output_dir().join("rubric_eval_alias");
    std::fs::create_dir_all(&out_dir)?;

    let indicators_copy = out_dir.join("valid_indicators_alias.json");
    std::fs::copy(&indicators, &indicators_copy)?;

    let result = Command::new(&binary)
        .arg("--profile")           // deprecated alias — should still work
        .arg(&rubric)
        .arg(&indicators_copy)
        .output()?;

    assert!(
        result.status.success(),
        "--profile alias should still be accepted:\nstderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let report_path = out_dir.join("valid_indicators_alias-report.json");
    assert!(report_path.exists(), "report should be written when using --profile alias");

    println!("✓ CLI --profile alias: accepted, report written");
    Ok(())
}
