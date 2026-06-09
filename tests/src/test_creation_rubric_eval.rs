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

//! Runs every asset produced by testfiles-creation through the conformance rubric
//! (spec 2.4) and writes the evaluation report alongside the asset as
//! `<asset-filename>.yml`.

use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

mod common;
use common::{cli_binary_path, output_dir};

/// Asset extensions produced by the testfiles-creation test.
const ASSET_EXTENSIONS: &[&str] = &["jpg", "png", "webp", "svg", "mp4"];

/// Evaluate every signed asset in the testfiles-creation output directory
/// against the conformance rubric for spec 2.4.  A YAML report is written
/// alongside each asset with the naming pattern `<asset-filename>.yml`.
///
/// The test skips gracefully when the directory is empty (i.e., when
/// `test_all_testfiles_creation` has not been run yet).  It fails when an
/// asset is present but the evaluation cannot produce a report at all.
#[test]
fn test_rubric_eval_on_testfiles_creation_outputs() -> Result<()> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let assets_dir = output_dir().join("testfiles-creation");
    let rubric = manifest_dir.join("rubrics/asset-rubric-conformance0.2-spec2.4.yml");
    let binary = cli_binary_path();

    assert!(
        rubric.exists(),
        "rubric not found at {rubric:?} — was the repo checked out completely?"
    );

    // Collect all signed asset files (skip any .yml/.yaml report files that
    // may already be present from a prior run).
    let mut asset_paths: Vec<PathBuf> = match std::fs::read_dir(&assets_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .map(|ext| ASSET_EXTENSIONS.contains(&ext))
                    .unwrap_or(false)
            })
            .collect(),
        Err(_) => {
            println!(
                "testfiles-creation output directory not found — skipping rubric evaluation.\n\
                 Run `test_all_testfiles_creation` first to generate assets."
            );
            return Ok(());
        }
    };

    if asset_paths.is_empty() {
        println!(
            "No assets found in {assets_dir:?} — skipping.\n\
             Run `test_all_testfiles_creation` first to generate assets."
        );
        return Ok(());
    }

    asset_paths.sort();

    let mut failures: Vec<String> = Vec::new();

    for asset_path in &asset_paths {
        let asset_filename = asset_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // The CLI writes the report as `<stem>-report.yml` in the same directory.
        let stem = asset_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let report_path = assets_dir.join(format!("{stem}-report.yml"));

        // Remove stale report from a prior run so we can detect a fresh write.
        let _ = std::fs::remove_file(&report_path);

        // Run the CLI — exit code may be non-zero when rubric checks fail (expected
        // for negative test cases), but a report should always be written for any
        // asset that has a valid C2PA manifest.
        let result = Command::new(&binary)
            .arg("--rubric")
            .arg(&rubric)
            .arg(&asset_path)
            .output();

        let output = match result {
            Ok(o) => o,
            Err(e) => {
                failures.push(format!("  FAIL {asset_filename}: could not launch CLI: {e}"));
                continue;
            }
        };

        if !report_path.exists() {
            failures.push(format!(
                "  FAIL {asset_filename}: no report written\n    stdout: {}\n    stderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            ));
            continue;
        }

        if report_path.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
            failures.push(format!("  FAIL {asset_filename}: report file is empty"));
        }
    }

    assert!(
        failures.is_empty(),
        "{} asset(s) failed rubric evaluation:\n{}",
        failures.len(),
        failures.join("\n")
    );

    println!(
        "✓ Rubric reports written for {} asset(s) in {assets_dir:?}",
        asset_paths.len()
    );
    Ok(())
}
