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

//! Tests for rubric evaluation against signed assets.
//!
//! All tests use the `rubrics/` directory (rubric_metadata format) and the signed
//! asset fixtures under `tests/fixtures/assets/signed/`.

use anyhow::Result;
use profile_evaluator_rs::load_profile;
use std::path::PathBuf;
use std::process::Command;

mod common;

fn rubrics_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("rubrics")
}

// ============================================================================
// Rubric loading tests — all rubric files
// ============================================================================

#[test]
fn test_load_conformance_rubric_spec24() {
    let path = rubrics_dir().join("asset-rubric-conformance0.2-spec2.4.yml");
    assert!(path.exists(), "rubric should exist at {path:?}");
    load_profile(&path).expect("conformance rubric (spec 2.4) should load without error");
}

#[test]
fn test_load_conformance_rubric_spec22_v02() {
    let path = rubrics_dir().join("asset-rubric-conformance0.2-spec2.2.yml");
    assert!(path.exists(), "rubric should exist at {path:?}");
    load_profile(&path).expect("conformance rubric 0.2 (spec 2.2) should load without error");
}

#[test]
fn test_load_conformance_rubric_spec22_v01() {
    let path = rubrics_dir().join("asset-rubric-conformance0.1-spec2.2.yml");
    assert!(path.exists(), "rubric should exist at {path:?}");
    load_profile(&path).expect("conformance rubric 0.1 (spec 2.2) should load without error");
}

#[test]
fn test_load_integrity_rubric() {
    let path = rubrics_dir().join("asset-rubric-integrity.yml");
    assert!(path.exists(), "rubric should exist at {path:?}");
    load_profile(&path).expect("integrity rubric should load without error");
}

#[test]
fn test_load_signals_local_rubric() {
    let path = rubrics_dir().join("asset-rubric-signals-local.yml");
    assert!(path.exists(), "rubric should exist at {path:?}");
    load_profile(&path).expect("signals-local rubric should load without error");
}

// ============================================================================
// CLI integration tests — --rubric flag
// ============================================================================

/// Default output format is YAML: omitting --report-format should produce a .yaml report.
#[test]
fn test_cli_rubric_default_output_is_yaml() -> Result<()> {
    let binary = common::cli_binary_path();
    let asset = common::signed_assets_dir().join("sig_es256.jpg");
    let rubric = rubrics_dir().join("asset-rubric-conformance0.2-spec2.4.yml");

    let out_dir = common::output_dir().join("rubric_default_format");
    std::fs::create_dir_all(&out_dir)?;
    let asset_copy = out_dir.join("sig_es256.jpg");
    std::fs::copy(&asset, &asset_copy)?;

    let result = Command::new(&binary)
        .arg("--rubric").arg(&rubric)
        .arg(&asset_copy)
        .output()?;

    assert!(
        result.status.success(),
        "CLI --rubric should succeed:\nstderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let report_path = out_dir.join("sig_es256-report.yml");
    assert!(report_path.exists(), "default report should be YAML at {report_path:?}");

    // Must be valid YAML with a statements field
    let content = std::fs::read_to_string(&report_path)?;
    let parsed: serde_json::Value = serde_yaml::from_str(&content)?;
    assert!(parsed.get("statements").is_some(), "report should have statements");

    // Confirm a .json report was NOT written
    assert!(
        !out_dir.join("sig_es256-report.json").exists(),
        "a .json report should not exist when using the default format"
    );

    println!("✓ CLI --rubric: default output is YAML");
    Ok(())
}

/// Explicit --report-format json produces a .json report.
#[test]
fn test_cli_rubric_explicit_json_output() -> Result<()> {
    let binary = common::cli_binary_path();
    let asset = common::signed_assets_dir().join("sig_es256.jpg");
    let rubric = rubrics_dir().join("asset-rubric-conformance0.2-spec2.4.yml");

    let out_dir = common::output_dir().join("rubric_explicit_json");
    std::fs::create_dir_all(&out_dir)?;
    let asset_copy = out_dir.join("sig_es256.jpg");
    std::fs::copy(&asset, &asset_copy)?;

    let result = Command::new(&binary)
        .arg("--rubric").arg(&rubric)
        .arg("--report-format").arg("json")
        .arg(&asset_copy)
        .output()?;

    assert!(
        result.status.success(),
        "CLI --rubric --report-format json should succeed:\nstderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let report_path = out_dir.join("sig_es256-report.json");
    assert!(report_path.exists(), "JSON report should be at {report_path:?}");

    let content = std::fs::read_to_string(&report_path)?;
    let parsed: serde_json::Value = serde_json::from_str(&content)?;
    assert!(parsed.get("statements").is_some(), "JSON report should have statements");

    println!("✓ CLI --rubric --report-format json: writes JSON report");
    Ok(())
}

/// Multiple input files each get their own report.
#[test]
fn test_cli_rubric_multiple_inputs_produce_separate_reports() -> Result<()> {
    let binary = common::cli_binary_path();
    let asset = common::signed_assets_dir().join("sig_es256.jpg");
    let rubric = rubrics_dir().join("asset-rubric-conformance0.2-spec2.4.yml");

    let out_dir = common::output_dir().join("rubric_multi_input");
    std::fs::create_dir_all(&out_dir)?;

    let copy1 = out_dir.join("asset_a.jpg");
    let copy2 = out_dir.join("asset_b.jpg");
    std::fs::copy(&asset, &copy1)?;
    std::fs::copy(&asset, &copy2)?;

    let result = Command::new(&binary)
        .arg("--rubric").arg(&rubric)
        .arg(&copy1)
        .arg(&copy2)
        .output()?;

    println!("stdout: {}", String::from_utf8_lossy(&result.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&result.stderr));

    assert!(
        result.status.success(),
        "multi-file rubric eval should succeed:\nstderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let report1 = out_dir.join("asset_a-report.yml");
    let report2 = out_dir.join("asset_b-report.yml");
    assert!(report1.exists(), "report for asset_a should be written: {report1:?}");
    assert!(report2.exists(), "report for asset_b should be written: {report2:?}");

    println!("✓ CLI --rubric multi-input: separate report written for each file");
    Ok(())
}

/// An unsupported file type (e.g. .txt) should cause a non-zero exit code.
#[test]
fn test_cli_rubric_unsupported_file_type_fails() -> Result<()> {
    let binary = common::cli_binary_path();
    let rubric = rubrics_dir().join("asset-rubric-conformance0.2-spec2.4.yml");

    let out_dir = common::output_dir().join("rubric_unsupported_type");
    std::fs::create_dir_all(&out_dir)?;
    let bad_file = out_dir.join("not_an_asset.txt");
    std::fs::write(&bad_file, "this is not a supported asset or crJSON file")?;

    let result = Command::new(&binary)
        .arg("--rubric").arg(&rubric)
        .arg(&bad_file)
        .output()?;

    println!("stderr: {}", String::from_utf8_lossy(&result.stderr));

    assert!(
        !result.status.success(),
        "CLI should fail with a non-zero exit code for an unsupported file type"
    );

    println!("✓ CLI --rubric unsupported file type: correctly fails");
    Ok(())
}

/// A non-zero exit code is returned when at least one input fails evaluation.
#[test]
fn test_cli_rubric_partial_failure_exits_nonzero() -> Result<()> {
    let binary = common::cli_binary_path();
    let rubric = rubrics_dir().join("asset-rubric-conformance0.2-spec2.4.yml");

    let out_dir = common::output_dir().join("rubric_partial_failure");
    std::fs::create_dir_all(&out_dir)?;

    // One valid signed asset and one file that will fail evaluation
    let good = out_dir.join("good.jpg");
    std::fs::copy(common::signed_assets_dir().join("sig_es256.jpg"), &good)?;

    let bad = out_dir.join("bad.jpg");
    std::fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/assets/raw/Dog.jpg"),
        &bad,
    )?;

    let result = Command::new(&binary)
        .arg("--rubric").arg(&rubric)
        .arg(&good)
        .arg(&bad)
        .output()?;

    println!("stdout: {}", String::from_utf8_lossy(&result.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&result.stderr));

    assert!(
        !result.status.success(),
        "CLI should exit non-zero when at least one input fails"
    );

    println!("✓ CLI --rubric partial failure: exits non-zero");
    Ok(())
}

/// A raw (unsigned) image has no C2PA manifest — should fail cleanly, not crash.
#[test]
fn test_cli_rubric_unsigned_asset_fails_cleanly() -> Result<()> {
    let binary = common::cli_binary_path();
    let raw_asset = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/assets/raw/Dog.jpg");
    let rubric = rubrics_dir().join("asset-rubric-conformance0.2-spec2.4.yml");

    assert!(raw_asset.exists(), "raw asset fixture should exist at {raw_asset:?}");

    let out_dir = common::output_dir().join("rubric_no_manifest");
    std::fs::create_dir_all(&out_dir)?;
    let asset_copy = out_dir.join("Dog.jpg");
    std::fs::copy(&raw_asset, &asset_copy)?;

    let result = Command::new(&binary)
        .arg("--rubric").arg(&rubric)
        .arg(&asset_copy)
        .output()?;

    println!("stdout: {}", String::from_utf8_lossy(&result.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&result.stderr));

    // Should exit non-zero (no manifest to extract) but must not crash/panic
    assert!(
        !result.status.success(),
        "CLI should fail gracefully for an asset with no C2PA manifest"
    );
    // Ensure it produced an error message, not a bare panic
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        !stderr.contains("panicked"),
        "process should not have panicked; stderr: {stderr}"
    );

    println!("✓ CLI --rubric unsigned asset: fails cleanly with no panic");
    Ok(())
}

/// A missing rubric file must cause a non-zero exit code.
#[test]
fn test_cli_rubric_missing_file_fails() -> Result<()> {
    let binary = common::cli_binary_path();
    let asset = common::signed_assets_dir().join("sig_es256.jpg");

    let out_dir = common::output_dir().join("rubric_missing_rubric");
    std::fs::create_dir_all(&out_dir)?;
    let asset_copy = out_dir.join("sig_es256.jpg");
    std::fs::copy(&asset, &asset_copy)?;

    let result = Command::new(&binary)
        .arg("--rubric").arg("/nonexistent/rubric.yml")
        .arg(&asset_copy)
        .output()?;

    println!("stderr: {}", String::from_utf8_lossy(&result.stderr));

    assert!(
        !result.status.success(),
        "CLI should fail when rubric file does not exist"
    );

    println!("✓ CLI --rubric missing rubric file: correctly fails");
    Ok(())
}

/// `--profile` still works as a backward-compatible alias for `--rubric`.
#[test]
fn test_cli_profile_alias_still_works() -> Result<()> {
    let binary = common::cli_binary_path();
    let asset = common::signed_assets_dir().join("sig_es256.jpg");
    let rubric = rubrics_dir().join("asset-rubric-conformance0.2-spec2.4.yml");

    let out_dir = common::output_dir().join("rubric_profile_alias");
    std::fs::create_dir_all(&out_dir)?;
    let asset_copy = out_dir.join("sig_es256.jpg");
    std::fs::copy(&asset, &asset_copy)?;

    let result = Command::new(&binary)
        .arg("--profile")           // deprecated alias — should still work
        .arg(&rubric)
        .arg(&asset_copy)
        .output()?;

    assert!(
        result.status.success(),
        "--profile alias should still be accepted:\nstderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let report_path = out_dir.join("sig_es256-report.yml");
    assert!(report_path.exists(), "report should be written when using --profile alias");

    println!("✓ CLI --profile alias: accepted, report written");
    Ok(())
}
